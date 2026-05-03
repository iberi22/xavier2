//! Adaptive Retrieval Gating - Multi-layer memory retrieval with weighted fusion
//!
//! Implements adaptive gating that scores and fuses results from Working, Episodic,
//! and Semantic memory layers using RRF (Reciprocal Rank Fusion).

use serde::{Deserialize, Serialize};

use crate::memory::entity_graph::EntityRecord;
use crate::memory::qmd_memory::MemoryDocument;
use crate::retrieval::config;
use crate::search::rrf::{reciprocal_rank_fusion, ScoredResult};

/// Layer weights for multi-layer retrieval fusion.
/// These control how much each memory layer contributes to final results.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LayerWeights {
    /// Weight for working memory layer (default 0.3)
    pub working: f32,
    /// Weight for episodic memory layer (default 0.3)
    pub episodic: f32,
    /// Weight for semantic memory layer (default 0.4)
    pub semantic: f32,
}

impl Default for LayerWeights {
    fn default() -> Self {
        Self {
            working: config::DEFAULT_WORKING_WEIGHT,
            episodic: config::DEFAULT_EPISODIC_WEIGHT,
            semantic: config::DEFAULT_SEMANTIC_WEIGHT,
        }
    }
}

impl LayerWeights {
    pub fn new(working: f32, episodic: f32, semantic: f32) -> Self {
        Self {
            working,
            episodic,
            semantic,
        }
    }

    /// Validate that weights sum to approximately 1.0
    pub fn is_valid(&self) -> bool {
        let sum = self.working + self.episodic + self.semantic;
        (sum - 1.0).abs() < 0.001
    }

    /// Get weight for a specific layer by name
    pub fn weight_for(&self, layer: &str) -> f32 {
        match layer {
            "working" => self.working,
            "episodic" => self.episodic,
            "semantic" => self.semantic,
            _ => 0.0,
        }
    }
}

/// Configuration for adaptive gating
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatingConfig {
    /// Layer weights for fusion
    pub layer_weights: LayerWeights,
    /// Minimum relevance score threshold (0.0-1.0)
    pub relevance_threshold: f32,
    /// RRF k parameter (default 60)
    pub rrf_k: u32,
    /// Maximum results to return
    pub max_results: usize,
}

impl Default for GatingConfig {
    fn default() -> Self {
        Self {
            layer_weights: LayerWeights::default(),
            relevance_threshold: config::DEFAULT_RELEVANCE_THRESHOLD,
            rrf_k: config::DEFAULT_RRF_K,
            max_results: config::DEFAULT_MAX_RESULTS,
        }
    }
}

/// Result from a single layer's search
#[derive(Debug, Clone)]
pub struct LayerSearchResult {
    pub layer: &'static str,
    pub results: Vec<ScoredResult>,
    pub scores: Vec<f32>,
}

/// Adaptive gating for multi-layer memory retrieval
#[derive(Debug, Clone)]
pub struct AdaptiveGating {
    config: GatingConfig,
}

impl AdaptiveGating {
    pub fn new(config: GatingConfig) -> Self {
        Self { config }
    }

    pub fn with_defaults() -> Self {
        Self {
            config: GatingConfig::default(),
        }
    }

    /// Retrieve from all memory layers and fuse results
    pub fn retrieve(
        &self,
        working: &[MemoryDocument],
        episodic: &[SessionSummary],
        semantic: &[EntityRecord],
        query: &str,
    ) -> Vec<ScoredResult> {
        // 1. Score each layer independently
        let working_results = self.score_working_layer(working, query);
        let episodic_results = self.score_episodic_layer(episodic, query);
        let semantic_results = self.score_semantic_layer(semantic, query);

        // 2. Apply layer weights to scores
        let weighted_working =
            self.apply_weights(working_results, self.config.layer_weights.working);
        let weighted_episodic =
            self.apply_weights(episodic_results, self.config.layer_weights.episodic);
        let weighted_semantic =
            self.apply_weights(semantic_results, self.config.layer_weights.semantic);

        // 3. Fuse with RRF
        let fused = reciprocal_rank_fusion(
            vec![weighted_working, weighted_episodic, weighted_semantic],
            self.config.rrf_k,
        );

        // 4. Filter by threshold and limit results
        fused
            .into_iter()
            .filter(|r| r.score >= self.config.relevance_threshold)
            .take(self.config.max_results)
            .collect()
    }

    /// Retrieve only from working memory
    pub fn retrieve_working(&self, working: &[MemoryDocument], query: &str) -> Vec<ScoredResult> {
        self.score_working_layer(working, query)
    }

    /// Retrieve only from episodic memory
    pub fn retrieve_episodic(&self, episodic: &[SessionSummary], query: &str) -> Vec<ScoredResult> {
        self.score_episodic_layer(episodic, query)
    }

    /// Retrieve only from semantic memory
    pub fn retrieve_semantic(&self, semantic: &[EntityRecord], query: &str) -> Vec<ScoredResult> {
        self.score_semantic_layer(semantic, query)
    }

    /// Score working memory layer using keyword matching
    fn score_working_layer(&self, working: &[MemoryDocument], query: &str) -> Vec<ScoredResult> {
        let query_lower = query.to_lowercase();
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();

        let mut results: Vec<ScoredResult> = working
            .iter()
            .filter_map(|doc| {
                let content_lower = doc.content.to_lowercase();
                let mut score = 0.0_f32;

                // Exact phrase match bonus
                if content_lower.contains(&query_lower) {
                    score += config::EXACT_PHRASE_MATCH_BONUS;
                }

                // Term frequency scoring
                for term in &query_terms {
                    if content_lower.contains(term) {
                        score += config::TERM_MATCH_BONUS;
                        // Additional bonus for multiple occurrences
                        let count = content_lower.matches(term).count() as f32;
                        score += (count * config::TERM_OCCURRENCE_BONUS)
                            .min(config::MAX_TERM_OCCURRENCE_BONUS);
                    }
                }

                if score > 0.0 {
                    Some(ScoredResult {
                        id: doc.id.clone().unwrap_or_default(),
                        content: doc.content.clone(),
                        score: score.min(1.0),
                        source: "working".to_string(),
                        path: doc.path.clone(),
                        updated_at: doc
                            .metadata
                            .get("updated_at")
                            .and_then(|v| v.as_str())
                            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                            .map(|dt| dt.timestamp_millis()),
                    })
                } else {
                    None
                }
            })
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results
    }

    /// Score episodic memory layer using summary and event matching
    fn score_episodic_layer(&self, episodic: &[SessionSummary], query: &str) -> Vec<ScoredResult> {
        let query_lower = query.to_lowercase();
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();

        let mut results: Vec<ScoredResult> = episodic
            .iter()
            .filter_map(|session| {
                let summary_lower = session.summary.to_lowercase();
                let mut score = 0.0_f32;

                // Summary match
                if summary_lower.contains(&query_lower) {
                    score += config::EXACT_PHRASE_MATCH_BONUS;
                }

                // Term frequency in summary
                for term in &query_terms {
                    if summary_lower.contains(term) {
                        score += config::TERM_MATCH_BONUS;
                        let count = summary_lower.matches(term).count() as f32;
                        score += (count * config::TERM_OCCURRENCE_BONUS)
                            .min(config::MAX_TERM_OCCURRENCE_BONUS);
                    }
                }

                // Event matching
                for event in &session.key_events {
                    let event_lower = event.description.to_lowercase();
                    if event_lower.contains(&query_lower) {
                        score += config::EVENT_PHRASE_MATCH_BONUS;
                    }
                    for term in &query_terms {
                        if event_lower.contains(term) {
                            score += config::EVENT_TERM_MATCH_BONUS;
                        }
                    }
                }

                if score > 0.0 {
                    Some(ScoredResult {
                        id: session.session_id.clone(),
                        content: session.summary.clone(),
                        score: score.min(1.0),
                        source: "episodic".to_string(),
                        path: format!("sessions/{}", session.session_id),
                        updated_at: Some(session.start_time.timestamp_millis()),
                    })
                } else {
                    None
                }
            })
            .collect();

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results
    }

    /// Score semantic memory layer using entity matching
    fn score_semantic_layer(&self, semantic: &[EntityRecord], query: &str) -> Vec<ScoredResult> {
        let query_lower = query.to_lowercase();

        let mut results: Vec<ScoredResult> = semantic
            .iter()
            .filter_map(|entity| {
                let name_lower = entity.name.to_lowercase();
                let normalized_lower = entity.normalized_name.to_lowercase();
                let mut score = 0.0_f32;

                // Exact name match
                if name_lower == query_lower || normalized_lower == query_lower {
                    score = config::EXACT_ENTITY_MATCH_SCORE;
                }
                // Partial name match
                else if name_lower.contains(&query_lower) || query_lower.contains(&name_lower) {
                    score = config::PARTIAL_ENTITY_MATCH_SCORE;
                }
                // Description match
                else if let Some(desc) = &entity.description {
                    let desc_lower = desc.to_lowercase();
                    if desc_lower.contains(&query_lower) {
                        score = config::ENTITY_DESCRIPTION_MATCH_SCORE;
                    }
                    let query_terms: Vec<&str> = query_lower.split_whitespace().collect();
                    for term in &query_terms {
                        if desc_lower.contains(term) {
                            score += config::ENTITY_DESCRIPTION_TERM_BONUS;
                        }
                    }
                }
                // Alias matching
                else {
                    for alias in &entity.aliases {
                        if alias.to_lowercase().contains(&query_lower) {
                            score = config::ENTITY_ALIAS_MATCH_SCORE;
                            break;
                        }
                    }
                }

                // Boost by confirmation count (normalized)
                let final_score = score * config::SEMANTIC_CONFIDENCE_MULTIPLIER;

                if final_score > 0.0 {
                    Some(ScoredResult {
                        id: entity.id.clone(),
                        content: entity.name.clone(),
                        score: final_score.min(1.0),
                        source: "semantic".to_string(),
                        path: format!("entities/{}", entity.id),
                        updated_at: Some(entity.last_seen.timestamp_millis()),
                    })
                } else {
                    None
                }
            })
            .collect();

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results
    }

    /// Apply layer weight to all scores in a result set
    fn apply_weights(&self, results: Vec<ScoredResult>, weight: f32) -> Vec<ScoredResult> {
        results
            .into_iter()
            .map(|mut r| {
                r.score *= weight;
                r
            })
            .collect()
    }

    /// Get configuration reference
    pub fn config(&self) -> &GatingConfig {
        &self.config
    }

    /// Update layer weights
    pub fn set_weights(&mut self, weights: LayerWeights) {
        self.config.layer_weights = weights;
    }

    /// Update relevance threshold
    pub fn set_threshold(&mut self, threshold: f32) {
        self.config.relevance_threshold = threshold.clamp(0.0, 1.0);
    }
}

/// Session summary for episodic memory layer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub summary: String,
    pub key_events: Vec<Event>,
    #[serde(default)]
    pub sentiment_timeline: Vec<f32>,
}

/// Key event within a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub description: String,
    pub event_type: String,
}

/// Layer statistics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerStats {
    pub working_count: usize,
    pub episodic_count: usize,
    pub semantic_count: usize,
    pub last_retrieval_layer_weights: LayerWeights,
    pub total_queries: u64,
}

impl Default for LayerStats {
    fn default() -> Self {
        Self {
            working_count: 0,
            episodic_count: 0,
            semantic_count: 0,
            last_retrieval_layer_weights: LayerWeights::default(),
            total_queries: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layer_weights_default() {
        let weights = LayerWeights::default();
        assert!((weights.working - 0.3).abs() < 0.001);
        assert!((weights.episodic - 0.3).abs() < 0.001);
        assert!((weights.semantic - 0.4).abs() < 0.001);
        assert!(weights.is_valid());
    }

    #[test]
    fn test_layer_weights_custom() {
        let weights = LayerWeights::new(0.2, 0.3, 0.5);
        assert!((weights.working - 0.2).abs() < 0.001);
        assert!((weights.semantic - 0.5).abs() < 0.001);
        assert!(weights.is_valid());
    }

    #[test]
    fn test_weight_for_layer() {
        let weights = LayerWeights::default();
        assert!((weights.weight_for("working") - 0.3).abs() < 0.001);
        assert!((weights.weight_for("episodic") - 0.3).abs() < 0.001);
        assert!((weights.weight_for("semantic") - 0.4).abs() < 0.001);
        assert!((weights.weight_for("unknown")).abs() < 0.001);
    }

    #[test]
    fn test_working_layer_scoring() {
        let gating = AdaptiveGating::with_defaults();
        let docs = vec![
            MemoryDocument {
                id: Some("doc1".to_string()),
                path: "test/path1".to_string(),
                content: "BELA works at SWAL".to_string(),
                metadata: serde_json::json!({}),
                content_vector: None,
                embedding: vec![],
            },
            MemoryDocument {
                id: Some("doc2".to_string()),
                path: "test/path2".to_string(),
                content: "Something else entirely".to_string(),
                metadata: serde_json::json!({}),
                content_vector: None,
                embedding: vec![],
            },
        ];

        let results = gating.score_working_layer(&docs, "BELA");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "doc1");
        assert!(results[0].score > 0.0);
    }

    #[test]
    fn test_semantic_layer_scoring() {
        let gating = AdaptiveGating::with_defaults();
        let entities = vec![
            EntityRecord {
                id: "entity1".to_string(),
                name: "BELA".to_string(),
                normalized_name: "bela".to_string(),
                entity_type: crate::memory::entity_graph::EntityType::Person,
                aliases: vec![],
                description: Some("Developer at SWAL".to_string()),
                occurrence_count: 5,
                memory_count: 3,
                first_seen: chrono::Utc::now(),
                last_seen: chrono::Utc::now(),
                merged_from: vec![],
                trust_score: 0.5,
                trust_rank: 1,
            },
            EntityRecord {
                id: "entity2".to_string(),
                name: "SWAL".to_string(),
                normalized_name: "swal".to_string(),
                entity_type: crate::memory::entity_graph::EntityType::Organization,
                aliases: vec![],
                description: Some("Southwest AI Labs".to_string()),
                occurrence_count: 10,
                memory_count: 5,
                first_seen: chrono::Utc::now(),
                last_seen: chrono::Utc::now(),
                merged_from: vec![],
                trust_score: 0.5,
                trust_rank: 1,
            },
        ];

        let results = gating.score_semantic_layer(&entities, "BELA");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "entity1");
        // Exact matches are scaled by the fixed semantic confidence multiplier.
        assert_eq!(results[0].score, 0.5);
    }

    #[test]
    fn test_multi_layer_retrieval() {
        let mut gating = AdaptiveGating::with_defaults();
        gating.set_threshold(0.0);
        let docs = vec![MemoryDocument {
            id: Some("doc1".to_string()),
            path: "test".to_string(),
            content: "BELA works at SWAL".to_string(),
            metadata: serde_json::json!({}),
            content_vector: None,
            embedding: vec![],
        }];
        let sessions: Vec<SessionSummary> = vec![];
        let entities: Vec<EntityRecord> = vec![];

        let results = gating.retrieve(&docs, &sessions, &entities, "BELA");
        assert!(!results.is_empty());
        assert_eq!(results[0].source, "hybrid");
    }
}
