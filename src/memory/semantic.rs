//! Semantic Memory Layer - Entity Graph and Knowledge Representation
//!
//! Implements the semantic memory layer from the Multi-Layer Memory Architecture:
//! - Entity extraction (NER-style)
//! - Relationship tracking between entities
//! - Trust scoring based on confirmation count
//! - Concept hierarchy and fact storage
//!
//! Based on Jia et al. LOCOMO consistency model for >98% accuracy.

use anyhow::{anyhow, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tokio::sync::RwLock;
use tracing::{debug, info};

use super::entities::{
    EntityGraphStats, SemanticEntity, SemanticEntityType, SemanticRelation, UpsertEntityRequest,
    UpsertRelationRequest,
};

/// Semantic Memory - manages entities and their relationships.
///
/// This implements the third layer of the Multi-Layer Memory Architecture,
/// sitting above Working Memory and Episodic Memory. It stores:
/// - Entities (people, organizations, products, concepts, locations, events)
/// - Relationships between entities
/// - Trust scores reflecting confirmation count
/// - Properties and metadata for each entity
pub struct SemanticMemory {
    /// Entity nodes keyed by ID
    entities: RwLock<HashMap<String, SemanticEntity>>,
    /// Relationship edges
    relations: RwLock<Vec<SemanticRelation>>,
    /// Outgoing relations map (source_id -> target_ids)
    outgoing: RwLock<HashMap<String, HashSet<String>>>,
    /// Incoming relations map (target_id -> source_ids)
    incoming: RwLock<HashMap<String, HashSet<String>>>,
    /// Entity name lookup (normalized name -> entity_id)
    name_index: RwLock<HashMap<String, String>>,
}

impl Default for SemanticMemory {
    fn default() -> Self {
        Self::new()
    }
}

impl SemanticMemory {
    /// Create a new semantic memory instance
    pub fn new() -> Self {
        Self {
            entities: RwLock::new(HashMap::new()),
            relations: RwLock::new(Vec::new()),
            outgoing: RwLock::new(HashMap::new()),
            incoming: RwLock::new(HashMap::new()),
            name_index: RwLock::new(HashMap::new()),
        }
    }

    // =====================================================================
    // Entity Operations
    // =====================================================================

    /// Add or update an entity
    pub async fn upsert_entity(&self, request: UpsertEntityRequest) -> Result<SemanticEntity> {
        let normalized = normalize_name(&request.name);

        // Check if entity already exists by name
        let entity_id = {
            let name_index = self.name_index.read().await;
            name_index.get(&normalized).cloned()
        };

        let mut entities = self.entities.write().await;

        if let Some(existing_id) = entity_id {
            // Update existing entity
            if let Some(existing) = entities.get_mut(&existing_id) {
                existing.last_updated = Utc::now();
                existing.confirmation_count += 1;

                // Update type if different
                if existing.entity_type != request.entity_type {
                    existing.entity_type = request.entity_type;
                }

                // Add new properties
                for (key, value) in request.properties {
                    existing.properties.insert(key, value);
                }

                // Add aliases
                for alias in request.aliases {
                    if !existing.aliases.contains(&alias) {
                        existing.aliases.push(alias);
                    }
                }

                // Add source memory
                if let Some(memory_id) = request.source_memory {
                    if !existing.source_memories.contains(&memory_id) {
                        existing.source_memories.push(memory_id);
                    }
                }

                // Update trust score
                self.update_entity_trust_internal(existing);

                return Ok(existing.clone());
            }
        }

        // Create new entity
        let mut entity = SemanticEntity::new(request.name, request.entity_type);
        entity.properties = request.properties;
        entity.aliases = request.aliases;
        if let Some(memory_id) = request.source_memory {
            entity.source_memories.push(memory_id);
        }

        let entity_id = entity.id.clone();
        let _entity_normalized = entity.normalized_name();

        // Add to entities
        entities.insert(entity_id.clone(), entity.clone());

        // Update name index
        drop(entities);
        self.rebuild_indexes().await;

        debug!(
            "Created new semantic entity: {} ({})",
            entity.name, entity_id
        );
        Ok(entity)
    }

    /// Get an entity by ID or name
    pub async fn get_entity(&self, id_or_name: &str) -> Option<SemanticEntity> {
        // Try ID first
        {
            let entities = self.entities.read().await;
            if let Some(entity) = entities.get(id_or_name) {
                return Some(entity.clone());
            }
        }

        // Try name lookup
        let normalized = normalize_name(id_or_name);
        let name_index = self.name_index.read().await;
        let entity_id = name_index.get(&normalized).cloned();
        if let Some(id) = entity_id {
            let entities = self.entities.read().await;
            entities.get(&id).cloned()
        } else {
            None
        }
    }

    /// Get all entities, optionally filtered by type
    pub async fn get_entities(
        &self,
        entity_type: Option<SemanticEntityType>,
    ) -> Vec<SemanticEntity> {
        let entities = self.entities.read().await;
        match entity_type {
            Some(t) => entities
                .values()
                .filter(|e| e.entity_type == t)
                .cloned()
                .collect(),
            None => entities.values().cloned().collect(),
        }
    }

    /// Delete an entity and its relations
    pub async fn delete_entity(&self, entity_id: &str) -> Result<()> {
        // Remove from entities
        let removed = {
            let mut entities = self.entities.write().await;
            entities.remove(entity_id).is_some()
        };

        if !removed {
            return Err(anyhow!("Entity not found: {}", entity_id));
        }

        // Remove relations involving this entity
        {
            let mut relations = self.relations.write().await;
            relations.retain(|r| r.source_id != entity_id && r.target_id != entity_id);
        }

        // Rebuild indexes
        self.rebuild_indexes().await;

        info!("Deleted semantic entity: {}", entity_id);
        Ok(())
    }

    /// Merge two entities (secondary merged into primary)
    pub async fn merge_entities(
        &self,
        primary_id: &str,
        secondary_id: &str,
    ) -> Result<SemanticEntity> {
        // Get secondary first since we'll remove it
        let secondary = {
            let entities = self.entities.read().await;
            entities.get(secondary_id).cloned()
        };
        let secondary =
            secondary.ok_or_else(|| anyhow!("Secondary entity not found: {}", secondary_id))?;

        // Remove secondary and get primary in same write guard
        {
            let mut entities = self.entities.write().await;

            // Remove secondary first
            let _ = entities.remove(secondary_id);

            let primary = entities
                .get_mut(primary_id)
                .ok_or_else(|| anyhow!("Primary entity not found: {}", primary_id))?;

            // Merge properties
            primary.merge_properties(&secondary);

            // Merge aliases
            for alias in &secondary.aliases {
                if !primary.aliases.contains(alias) {
                    primary.aliases.push(alias.clone());
                }
            }

            // Merge source memories
            for memory_id in &secondary.source_memories {
                if !primary.source_memories.contains(memory_id) {
                    primary.source_memories.push(memory_id.clone());
                }
            }

            // Update confirmation count
            primary.confirmation_count += secondary.confirmation_count;
            primary.last_updated = Utc::now();

            // Recalculate trust
            drop(entities);
            self.update_entity_trust_internal_id(primary_id).await?;
        }

        // Update relations: change secondary_id to primary_id in all relations
        {
            let mut relations = self.relations.write().await;
            for relation in relations.iter_mut() {
                if relation.source_id == secondary_id {
                    relation.source_id = primary_id.to_string();
                }
                if relation.target_id == secondary_id {
                    relation.target_id = primary_id.to_string();
                }
            }
        }

        self.rebuild_indexes().await;

        info!("Merged entity {} into {}", secondary_id, primary_id);

        self.get_entity(primary_id)
            .await
            .ok_or_else(|| anyhow!("Entity not found after merge"))
    }

    // =====================================================================
    // Relationship Operations
    // =====================================================================

    /// Add or update a relationship between two entities
    pub async fn upsert_relation(
        &self,
        request: UpsertRelationRequest,
    ) -> Result<SemanticRelation> {
        // Resolve entity names to IDs
        let source_id = self
            .resolve_entity(&request.source_name)
            .await?
            .ok_or_else(|| anyhow!("Source entity not found: {}", request.source_name))?;
        let target_id = self
            .resolve_entity(&request.target_name)
            .await?
            .ok_or_else(|| anyhow!("Target entity not found: {}", request.target_name))?;

        let _normalized_key = relation_key(&source_id, &target_id, &request.relation_type);

        let mut relations = self.relations.write().await;

        // Check if relation exists
        if let Some(existing) = relations.iter_mut().find(|r| {
            r.source_id == source_id
                && r.target_id == target_id
                && r.relation_type == request.relation_type
        }) {
            // Update existing
            existing.confirmation_count += 1;
            existing.updated_at = Utc::now();
            existing.weight = (existing.weight + request.weight) / 2.0;

            if let Some(memory_id) = request.source_memory {
                if !existing.source_memories.contains(&memory_id) {
                    existing.source_memories.push(memory_id);
                }
            }

            return Ok(existing.clone());
        }

        // Create new relation
        let mut relation = SemanticRelation::new(
            source_id.clone(),
            target_id.clone(),
            request.relation_type.clone(),
        );
        relation.weight = request.weight;
        if let Some(memory_id) = request.source_memory {
            relation.source_memories.push(memory_id);
        }
        relation.properties = request.properties;

        relations.push(relation.clone());

        // Update adjacency maps
        {
            let mut outgoing = self.outgoing.write().await;
            outgoing
                .entry(source_id.clone())
                .or_default()
                .insert(target_id.clone());
        }
        {
            let mut incoming = self.incoming.write().await;
            incoming
                .entry(target_id.clone())
                .or_default()
                .insert(source_id.clone());
        }

        debug!(
            "Created semantic relation: {} -> {} ({})",
            request.source_name, request.target_name, request.relation_type
        );
        Ok(relation)
    }

    /// Get relations for an entity
    pub async fn get_relations(
        &self,
        entity_id: &str,
        direction: RelationDirection,
    ) -> Vec<SemanticRelation> {
        let relations = self.relations.read().await;

        match direction {
            RelationDirection::Outgoing => relations
                .iter()
                .filter(|r| r.source_id == entity_id)
                .cloned()
                .collect(),
            RelationDirection::Incoming => relations
                .iter()
                .filter(|r| r.target_id == entity_id)
                .cloned()
                .collect(),
            RelationDirection::Both => relations
                .iter()
                .filter(|r| r.source_id == entity_id || r.target_id == entity_id)
                .cloned()
                .collect(),
        }
    }

    /// Get entity neighbors (entities connected by relations)
    pub async fn get_neighbors(
        &self,
        entity_id: &str,
        direction: RelationDirection,
    ) -> Vec<(SemanticEntity, SemanticRelation)> {
        let relations = self.relations.read().await;
        let entities = self.entities.read().await;

        let neighbor_ids: HashSet<String> = match direction {
            RelationDirection::Outgoing => relations
                .iter()
                .filter(|r| r.source_id == entity_id)
                .map(|r| r.target_id.clone())
                .collect(),
            RelationDirection::Incoming => relations
                .iter()
                .filter(|r| r.target_id == entity_id)
                .map(|r| r.source_id.clone())
                .collect(),
            RelationDirection::Both => relations
                .iter()
                .filter(|r| r.source_id == entity_id || r.target_id == entity_id)
                .map(|r| {
                    if r.source_id == entity_id {
                        r.target_id.clone()
                    } else {
                        r.source_id.clone()
                    }
                })
                .collect(),
        };

        neighbor_ids
            .iter()
            .filter_map(|nid| {
                let entity = entities.get(nid)?.clone();
                let relation = relations
                    .iter()
                    .find(|r| {
                        (r.source_id == entity_id && r.target_id == *nid)
                            || (r.target_id == entity_id && r.source_id == *nid)
                    })
                    .cloned()?;
                Some((entity, relation))
            })
            .collect()
    }

    /// Traverse the entity graph (BFS)
    pub async fn traverse(
        &self,
        start_id: &str,
        max_depth: usize,
        direction: RelationDirection,
    ) -> Vec<TraversalResult> {
        let relations = self.relations.read().await;
        let _entities = self.entities.read().await;

        let mut visited = HashSet::new();
        let mut queue = Vec::new();
        let mut results = Vec::new();

        queue.push((start_id.to_string(), 0, Vec::<String>::new()));

        while let Some((current_id, depth, path)) = queue.pop() {
            if depth >= max_depth || visited.contains(&current_id) {
                continue;
            }
            visited.insert(current_id.clone());

            let mut next_path = path.clone();
            next_path.push(current_id.clone());

            // Get neighbors
            let neighbors: Vec<(String, String)> = match direction {
                RelationDirection::Outgoing => relations
                    .iter()
                    .filter(|r| r.source_id == current_id)
                    .map(|r| (r.target_id.clone(), r.relation_type.clone()))
                    .collect(),
                RelationDirection::Incoming => relations
                    .iter()
                    .filter(|r| r.target_id == current_id)
                    .map(|r| (r.source_id.clone(), r.relation_type.clone()))
                    .collect(),
                RelationDirection::Both => relations
                    .iter()
                    .filter(|r| r.source_id == current_id || r.target_id == current_id)
                    .map(|r| {
                        if r.source_id == current_id {
                            (r.target_id.clone(), r.relation_type.clone())
                        } else {
                            (r.source_id.clone(), r.relation_type.clone())
                        }
                    })
                    .collect(),
            };

            for (neighbor_id, rel_type) in neighbors {
                if !visited.contains(&neighbor_id) {
                    results.push(TraversalResult {
                        entity_id: neighbor_id.clone(),
                        relation_type: rel_type,
                        depth: depth + 1,
                        path: next_path.clone(),
                    });
                    queue.push((neighbor_id, depth + 1, next_path.clone()));
                }
            }
        }

        results
    }

    // =====================================================================
    // Entity Extraction (NER-style)
    // =====================================================================

    /// Extract entities from text content using pattern matching.
    /// This is a simplified NER-style extraction based on the existing
    /// entity_graph.rs patterns.
    pub async fn extract_entities_from_text(
        &self,
        text: &str,
        source_memory: Option<String>,
    ) -> Vec<SemanticEntity> {
        let extracted = super::entity_graph::EntityGraph::extract_entities(text);
        let mut entities = Vec::new();

        for ext in extracted {
            let request = UpsertEntityRequest {
                name: ext.name,
                entity_type: map_entity_type(ext.entity_type),
                properties: HashMap::new(),
                aliases: Vec::new(),
                source_memory: source_memory.clone(),
            };

            if let Ok(entity) = self.upsert_entity(request).await {
                entities.push(entity);
            }
        }

        entities
    }

    /// Extract relations from text content.
    /// Uses regex patterns to find relationship mentions.
    pub async fn extract_relations_from_text(
        &self,
        text: &str,
        source_memory: Option<String>,
    ) -> Vec<SemanticRelation> {
        use super::entity_graph::EntityGraph;

        let relations = EntityGraph::extract_relation_candidates(text);
        let mut semantic_relations = Vec::new();

        for raw in relations {
            let request = UpsertRelationRequest {
                source_name: raw.source,
                target_name: raw.target,
                relation_type: raw.relation_type,
                weight: raw.score,
                source_memory: source_memory.clone(),
                properties: HashMap::new(),
            };

            if let Ok(relation) = self.upsert_relation(request).await {
                semantic_relations.push(relation);
            }
        }

        semantic_relations
    }

    /// Process a memory document and extract entities + relations
    pub async fn index_memory(&self, memory_id: &str, content: &str) -> Result<IndexMemoryResult> {
        let entities = self
            .extract_entities_from_text(content, Some(memory_id.to_string()))
            .await;
        let relations = self
            .extract_relations_from_text(content, Some(memory_id.to_string()))
            .await;

        Ok(IndexMemoryResult {
            memory_id: memory_id.to_string(),
            entities_extracted: entities.len(),
            relations_extracted: relations.len(),
            entity_ids: entities.iter().map(|e| e.id.clone()).collect(),
        })
    }

    // =====================================================================
    // Statistics and Utility
    // =====================================================================

    /// Get entity graph statistics
    pub async fn stats(&self) -> EntityGraphStats {
        let entities = self.entities.read().await;
        let relations = self.relations.read().await;

        let mut by_type = HashMap::new();
        let mut total_trust = 0.0f32;
        let mut high_trust_count = 0;

        for entity in entities.values() {
            *by_type
                .entry(entity.entity_type.as_str().to_string())
                .or_insert(0) += 1;
            total_trust += entity.trust_score;
            if entity.trust_score > 0.7 {
                high_trust_count += 1;
            }
        }

        let count = entities.len() as f32;
        EntityGraphStats {
            total_entities: entities.len(),
            total_relations: relations.len(),
            by_type,
            avg_trust_score: if count > 0.0 {
                total_trust / count
            } else {
                0.0
            },
            high_trust_count,
        }
    }

    /// Search entities by name pattern
    pub async fn search_entities(&self, query: &str) -> Vec<SemanticEntity> {
        let query_lower = query.to_ascii_lowercase();
        let entities = self.entities.read().await;

        entities
            .values()
            .filter(|e| {
                e.name.to_ascii_lowercase().contains(&query_lower)
                    || e.aliases
                        .iter()
                        .any(|a| a.to_ascii_lowercase().contains(&query_lower))
            })
            .cloned()
            .collect()
    }

    /// Clear all semantic memory (for testing/reset)
    pub async fn clear(&self) {
        let mut entities = self.entities.write().await;
        let mut relations = self.relations.write().await;
        let mut outgoing = self.outgoing.write().await;
        let mut incoming = self.incoming.write().await;
        let mut name_index = self.name_index.write().await;

        entities.clear();
        relations.clear();
        outgoing.clear();
        incoming.clear();
        name_index.clear();
    }

    /// Get all relations
    pub async fn all_relations(&self) -> Vec<SemanticRelation> {
        self.relations.read().await.clone()
    }

    // =====================================================================
    // Internal Helpers
    // =====================================================================

    /// Resolve entity name to ID
    async fn resolve_entity(&self, name: &str) -> Result<Option<String>> {
        let normalized = normalize_name(name);
        let name_index = self.name_index.read().await;
        Ok(name_index.get(&normalized).cloned())
    }

    /// Rebuild all indexes
    async fn rebuild_indexes(&self) {
        let entities = self.entities.read().await;
        let relations = self.relations.read().await;

        let mut outgoing = self.outgoing.write().await;
        let mut incoming = self.incoming.write().await;
        let mut name_index = self.name_index.write().await;

        outgoing.clear();
        incoming.clear();
        name_index.clear();

        // Index entities
        for entity in entities.values() {
            name_index.insert(entity.normalized_name(), entity.id.clone());
            for alias in &entity.aliases {
                name_index.insert(normalize_name(alias), entity.id.clone());
            }
        }

        // Index relations
        for relation in relations.iter() {
            outgoing
                .entry(relation.source_id.clone())
                .or_default()
                .insert(relation.target_id.clone());
            incoming
                .entry(relation.target_id.clone())
                .or_default()
                .insert(relation.source_id.clone());
        }
    }

    fn update_entity_trust_internal(&self, entity: &mut SemanticEntity) {
        use std::f32::consts::LN_2;
        let log_factor = (1.0 + entity.confirmation_count as f32).ln() / LN_2;
        entity.trust_score = (0.3 + 0.3 * log_factor).min(1.0);
    }

    async fn update_entity_trust_internal_id(&self, entity_id: &str) -> Result<()> {
        let mut entities = self.entities.write().await;
        if let Some(entity) = entities.get_mut(entity_id) {
            use std::f32::consts::LN_2;
            let log_factor = (1.0 + entity.confirmation_count as f32).ln() / LN_2;
            entity.trust_score = (0.3 + 0.3 * log_factor).min(1.0);
        }
        Ok(())
    }
}

/// Direction for relation queries
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationDirection {
    Outgoing,
    Incoming,
    Both,
}

impl Default for RelationDirection {
    fn default() -> Self {
        Self::Both
    }
}

/// Result from graph traversal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraversalResult {
    pub entity_id: String,
    pub relation_type: String,
    pub depth: usize,
    pub path: Vec<String>,
}

/// Result from indexing a memory document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexMemoryResult {
    pub memory_id: String,
    pub entities_extracted: usize,
    pub relations_extracted: usize,
    pub entity_ids: Vec<String>,
}

// =====================================================================
// Helper Functions
// =====================================================================

fn normalize_name(name: &str) -> String {
    name.split_whitespace()
        .map(|part| {
            part.trim_matches(|c: char| {
                matches!(
                    c,
                    ',' | '.' | ';' | ':' | '"' | '\'' | '(' | ')' | '[' | ']' | '-'
                )
            })
        })
        .filter(|part| !part.is_empty())
        .map(|part| part.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join(" ")
}

fn relation_key(source: &str, target: &str, relation_type: &str) -> String {
    format!("{}|{}|{}", source, target, relation_type)
}

fn map_entity_type(from: super::entity_graph::EntityType) -> SemanticEntityType {
    match from {
        super::entity_graph::EntityType::Person => SemanticEntityType::Person,
        super::entity_graph::EntityType::Organization => SemanticEntityType::Organization,
        super::entity_graph::EntityType::Location => SemanticEntityType::Location,
        super::entity_graph::EntityType::Product => SemanticEntityType::Product,
        super::entity_graph::EntityType::Concept => SemanticEntityType::Concept,
        super::entity_graph::EntityType::Unknown => SemanticEntityType::Concept,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_upsert_and_get_entity() {
        let sem = SemanticMemory::new();

        let entity = sem
            .upsert_entity(UpsertEntityRequest::new("Alice".to_string()))
            .await
            .unwrap();
        assert_eq!(entity.name, "Alice");
        assert_eq!(entity.entity_type, SemanticEntityType::Concept);

        let retrieved = sem.get_entity("Alice").await.unwrap();
        assert_eq!(retrieved.id, entity.id);
    }

    #[tokio::test]
    async fn test_upsert_relation() {
        let sem = SemanticMemory::new();

        // Create entities
        sem.upsert_entity(UpsertEntityRequest::new("Alice".to_string()))
            .await
            .unwrap();
        sem.upsert_entity(UpsertEntityRequest::new("SWAL".to_string()))
            .await
            .unwrap();

        // Create relation
        let relation = sem
            .upsert_relation(UpsertRelationRequest::new(
                "Alice".to_string(),
                "SWAL".to_string(),
                "works_at".to_string(),
            ))
            .await
            .unwrap();

        assert_eq!(relation.relation_type, "works_at");

        // Check neighbors
        let neighbors = sem
            .get_neighbors(&relation.source_id, RelationDirection::Outgoing)
            .await;
        assert_eq!(neighbors.len(), 1);
    }

    #[tokio::test]
    async fn test_entity_extraction() {
        let sem = SemanticMemory::new();

        let entities = sem
            .extract_entities_from_text(
                "BELA works at SWAL and knows Leonardo in Bogota.",
                Some("memory-1".to_string()),
            )
            .await;

        assert!(entities.len() >= 3); // BELA, SWAL, Leonardo, Bogota
    }

    #[tokio::test]
    async fn test_stats() {
        let sem = SemanticMemory::new();

        sem.upsert_entity(
            UpsertEntityRequest::new("Alice".to_string()).with_type(SemanticEntityType::Person),
        )
        .await
        .unwrap();
        sem.upsert_entity(
            UpsertEntityRequest::new("Bob".to_string()).with_type(SemanticEntityType::Person),
        )
        .await
        .unwrap();
        sem.upsert_entity(
            UpsertEntityRequest::new("Acme".to_string())
                .with_type(SemanticEntityType::Organization),
        )
        .await
        .unwrap();

        let stats = sem.stats().await;
        assert_eq!(stats.total_entities, 3);
        assert_eq!(stats.by_type.get("person"), Some(&2));
        assert_eq!(stats.by_type.get("organization"), Some(&1));
    }

    #[tokio::test]
    async fn test_traverse() {
        let sem = SemanticMemory::new();

        // Create chain: Alice -> Bob -> Carol
        sem.upsert_entity(UpsertEntityRequest::new("Alice".to_string()))
            .await
            .unwrap();
        sem.upsert_entity(UpsertEntityRequest::new("Bob".to_string()))
            .await
            .unwrap();
        sem.upsert_entity(UpsertEntityRequest::new("Carol".to_string()))
            .await
            .unwrap();

        sem.upsert_relation(UpsertRelationRequest::new(
            "Alice".to_string(),
            "Bob".to_string(),
            "knows".to_string(),
        ))
        .await
        .unwrap();
        sem.upsert_relation(UpsertRelationRequest::new(
            "Bob".to_string(),
            "Carol".to_string(),
            "knows".to_string(),
        ))
        .await
        .unwrap();

        let alice = sem.get_entity("Alice").await.unwrap();
        let traversal = sem
            .traverse(&alice.id, 2, RelationDirection::Outgoing)
            .await;

        assert!(traversal.len() >= 2); // Bob at depth 1, Carol at depth 2
    }
}

/// Extension trait to bridge SemanticMemory into workspace operations.
/// This allows WorkspaceState to call SemanticMemory::index_memory on added content.
#[async_trait::async_trait]
pub trait SemanticMemoryExt {
    /// Index a memory document: extract entities and relations from content.
    async fn index_memory(&self, memory_id: &str, content: &str) -> Result<IndexMemoryResult>;
}

#[async_trait::async_trait]
impl SemanticMemoryExt for SemanticMemory {
    async fn index_memory(&self, memory_id: &str, content: &str) -> Result<IndexMemoryResult> {
        SemanticMemory::index_memory(self, memory_id, content).await
    }
}
