//! Belief Graph - conceptual graph used by the Xavier reasoning layers.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, RwLock};
use tokio::sync::RwLock as AsyncRwLock;
use tracing::info;
use chrono::Utc;

use crate::domain::memory::belief::{BeliefEdge, BeliefNode};
use crate::agents::belief_evaluator::BeliefEvaluator;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Confidence {
    High,
    Medium,
    Low,
}

impl Confidence {
    pub fn score(self) -> f32 {
        match self {
            Self::High => 0.9,
            Self::Medium => 0.6,
            Self::Low => 0.3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Belief {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub confidence: Confidence,
}

impl Belief {
    pub fn new(subject: String, predicate: String, object: String, confidence: Confidence) -> Self {
        Self {
            subject,
            predicate,
            object,
            confidence,
        }
    }
}

/// Thread-safe belief graph that exposes both sync and async-friendly helpers.
#[derive(Debug)]
pub struct BeliefGraph {
    nodes: RwLock<HashMap<String, BeliefNode>>,
    edges: RwLock<Vec<BeliefEdge>>,
    adjacency: RwLock<HashMap<String, HashSet<String>>>,
    evaluator: BeliefEvaluator,
}

impl BeliefGraph {
    pub fn new() -> Self {
        Self {
            nodes: RwLock::new(HashMap::new()),
            edges: RwLock::new(Vec::new()),
            adjacency: RwLock::new(HashMap::new()),
            evaluator: BeliefEvaluator::new(),
        }
    }

    pub fn add_node(&self, concept: String, confidence: f32) {
        let id = ulid::Ulid::new().to_string();
        let node = BeliefNode {
            id: id.clone(),
            concept: concept.clone(),
            confidence,
            created_at: Utc::now(),
        };

        self.nodes
            .write()
            .expect("belief_graph: nodes write lock poisoned")
            .insert(id, node);
        self.adjacency
            .write()
            .expect("belief_graph: adjacency write lock poisoned")
            .entry(concept.clone())
            .or_default();

        info!("Added node: {}", concept);
    }

    pub async fn add_edge(&self, from: String, to: String, relation: String) {
        let _ = self.add_relation(from, to, relation, None, None).await;
    }

    pub async fn add_relation(
        &self,
        source: String,
        target: String,
        relation_type: String,
        provenance_id: Option<String>,
        source_type: Option<&str>,
    ) -> Result<()> {
        let provenance_id = provenance_id.unwrap_or_else(|| "unknown".to_string());
        let confidence_score = self.evaluator.evaluate_confidence(source_type.unwrap_or("unknown"), &relation_type).await;

        let mut new_edge = BeliefEdge::new(
            source.clone(),
            target.clone(),
            relation_type,
            confidence_score,
            provenance_id,
        );

        let existing_edges = self.get_edges_async().await;
        if let Some(contradicts_id) = self.evaluator.find_contradiction(&new_edge, &existing_edges) {
            new_edge.contradicts_edge_id = Some(contradicts_id);
            info!("Contradiction detected for {} -> {} ({}). Adding competing belief.", source, target, new_edge.relation_type);
        }

        self.edges
            .write()
            .expect("belief_graph: edges write lock poisoned")
            .push(new_edge.clone());

        let mut adjacency = self
            .adjacency
            .write()
            .expect("belief_graph: adjacency write lock poisoned");
        adjacency
            .entry(source.clone())
            .or_default()
            .insert(target.clone());
        adjacency.entry(target.clone()).or_default();

        info!(
            "Added relation: {} -> {} ({}) [confidence: {}]",
            source, target, new_edge.relation_type, confidence_score
        );

        Ok(())
    }

    pub fn get_related(&self, concept: &str) -> Vec<String> {
        self.adjacency
            .read()
            .expect("belief_graph: adjacency read lock poisoned")
            .get(concept)
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn get_node(&self, concept: &str) -> Option<BeliefNode> {
        self.nodes
            .read()
            .expect("belief_graph: nodes read lock poisoned")
            .values()
            .find(|node| node.concept == concept)
            .cloned()
    }

    pub fn list_nodes(&self) -> Vec<BeliefNode> {
        self.nodes
            .read()
            .expect("belief_graph: nodes read lock poisoned")
            .values()
            .cloned()
            .collect()
    }

    pub fn get_edges(&self) -> Vec<BeliefEdge> {
        self.edges
            .read()
            .expect("belief_graph: edges write lock poisoned")
            .clone()
    }

    pub async fn get_edges_async(&self) -> Vec<BeliefEdge> {
        self.get_edges()
    }

    pub fn get_relations(&self) -> Vec<BeliefEdge> {
        self.get_edges()
    }

    pub fn replace_relations(&self, edges: Vec<BeliefEdge>) {
        let mut nodes = HashMap::new();
        let mut adjacency = HashMap::<String, HashSet<String>>::new();

        for edge in &edges {
            nodes.entry(edge.source.clone()).or_insert(BeliefNode {
                id: edge.source.clone(),
                concept: edge.source.clone(),
                confidence: edge.confidence_score,
                created_at: edge.created_at,
            });
            nodes.entry(edge.target.clone()).or_insert(BeliefNode {
                id: edge.target.clone(),
                concept: edge.target.clone(),
                confidence: edge.confidence_score,
                created_at: edge.created_at,
            });

            adjacency
                .entry(edge.source.clone())
                .or_default()
                .insert(edge.target.clone());
            adjacency.entry(edge.target.clone()).or_default();
        }

        *self
            .nodes
            .write()
            .expect("belief_graph: nodes write lock poisoned") = nodes;
        *self
            .adjacency
            .write()
            .expect("belief_graph: adjacency write lock poisoned") = adjacency;
        *self
            .edges
            .write()
            .expect("belief_graph: edges write lock poisoned") = edges;
    }

    pub async fn add_belief(
        &self,
        belief: Belief,
        source_memory_id: Option<String>,
    ) -> Result<()> {
        let confidence_score = belief.confidence.score();

        if self.get_node(&belief.subject).is_none() {
            self.add_node(belief.subject.clone(), confidence_score);
        }

        if self.get_node(&belief.object).is_none() {
            self.add_node(belief.object.clone(), confidence_score);
        }

        self.add_relation(
            belief.subject,
            belief.object,
            belief.predicate,
            source_memory_id,
            None,
        ).await
    }

    /// Returns the highest-confidence paths or multiple beliefs if ambiguity exists.
    pub async fn search(&self, query: &str) -> Vec<BeliefEdge> {
        let query_lower = query.to_lowercase();
        let words: Vec<_> = query_lower
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() > 2)
            .collect();

        if words.is_empty() {
            return Vec::new();
        }

        let mut results = self.get_edges()
            .into_iter()
            .filter(|edge| {
                let s = edge.source.to_lowercase();
                let t = edge.target.to_lowercase();
                let r = edge.relation_type.to_lowercase();

                words
                    .iter()
                    .any(|w| s.contains(w) || t.contains(w) || r.contains(w))
            })
            .collect::<Vec<_>>();

        results.sort_by(|a, b| b.confidence_score.partial_cmp(&a.confidence_score).unwrap_or(std::cmp::Ordering::Equal));
        results
    }

    pub async fn bfs(&self, start: &str) -> Vec<String> {
        let adjacency = self
            .adjacency
            .read()
            .expect("belief_graph: adjacency read lock poisoned")
            .clone();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::from([start.to_string()]);
        let mut ordered = Vec::new();

        while let Some(current) = queue.pop_front() {
            if !visited.insert(current.clone()) {
                continue;
            }

            if current != start {
                ordered.push(current.clone());
            }

            if let Some(neighbors) = adjacency.get(&current) {
                for neighbor in neighbors {
                    if !visited.contains(neighbor) {
                        queue.push_back(neighbor.clone());
                    }
                }
            }
        }

        ordered
    }

    /// Finds the highest-confidence path between two concepts.
    pub async fn find_highest_confidence_path(&self, start: &str, end: &str) -> Vec<BeliefEdge> {
        let edges = self.get_edges();
        let mut distances = HashMap::new();
        let mut previous = HashMap::new();
        let mut queue = HashSet::new();

        distances.insert(start.to_string(), 0.0f32);
        queue.insert(start.to_string());

        // Simple Dijkstra-like approach using confidence as weight (higher is better, so we use 1.0 - confidence as cost)
        while !queue.is_empty() {
            let current = queue.iter().min_by(|a, b| {
                let da = distances.get(*a).unwrap_or(&f32::INFINITY);
                let db = distances.get(*b).unwrap_or(&f32::INFINITY);
                da.partial_cmp(db).unwrap_or(std::cmp::Ordering::Equal)
            }).cloned().unwrap();

            queue.remove(&current);

            if current == end {
                break;
            }

            for edge in edges.iter().filter(|e| e.source == current) {
                let alt = distances.get(&current).unwrap_or(&f32::INFINITY) + (1.0 - edge.confidence_score);
                if alt < *distances.get(&edge.target).unwrap_or(&f32::INFINITY) {
                    distances.insert(edge.target.clone(), alt);
                    previous.insert(edge.target.clone(), edge.clone());
                    queue.insert(edge.target.clone());
                }
            }
        }

        let mut path = Vec::new();
        let mut curr = end.to_string();
        while let Some(edge) = previous.get(&curr) {
            path.push(edge.clone());
            curr = edge.source.clone();
        }
        path.reverse();
        path
    }

    pub async fn has_supporting_beliefs(&self, memory_id: &str) -> bool {
        self.get_edges()
            .iter()
            .any(|e| e.provenance_id == memory_id)
    }
}

impl Default for BeliefGraph {
    fn default() -> Self {
        Self::new()
    }
}

pub type SharedBeliefGraph = Arc<AsyncRwLock<BeliefGraph>>;
