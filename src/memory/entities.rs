//! Entities - Entity types for semantic memory layer.
//!
//! Defines the core entity types and structures for the entity graph
//! in the semantic memory layer. Supports Person, Organization, Product,
//! Concept, Location, and Event entity types with trust scoring and
//! property tracking.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Entity types supported by the semantic memory layer.
/// These map to Named Entity Recognition (NER) categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SemanticEntityType {
    Person,
    Organization,
    Product,
    #[default]
    Concept,
    Location,
    Event,
}

impl SemanticEntityType {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Person => "person",
            Self::Organization => "organization",
            Self::Product => "product",
            Self::Concept => "concept",
            Self::Location => "location",
            Self::Event => "event",
        }
    }

    /// Create from string (case-insensitive)
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "person" => Some(Self::Person),
            "organization" | "org" => Some(Self::Organization),
            "product" => Some(Self::Product),
            "concept" => Some(Self::Concept),
            "location" => Some(Self::Location),
            "event" => Some(Self::Event),
            _ => None,
        }
    }
}

impl std::fmt::Display for SemanticEntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A semantic entity with metadata, properties, and trust scoring.
///
/// This is the primary node type in the EntityGraph (semantic memory).
/// Trust scores are based on confirmation count - more confirmations
/// from different sources lead to higher trust.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticEntity {
    /// Unique identifier (ULID)
    pub id: String,
    /// Primary name of the entity
    pub name: String,
    /// Entity type classification
    pub entity_type: SemanticEntityType,
    /// Arbitrary key-value properties for entity-specific data
    #[serde(default)]
    pub properties: HashMap<String, serde_json::Value>,
    /// Trust score [0.0, 1.0] based on confirmation count
    pub trust_score: f32,
    /// Timestamp of last update
    pub last_updated: DateTime<Utc>,
    /// When this entity was first created
    pub created_at: DateTime<Utc>,
    /// Number of times this entity has been confirmed
    #[serde(default)]
    pub confirmation_count: u32,
    /// Alternative names/aliases for this entity
    #[serde(default)]
    pub aliases: Vec<String>,
    /// Source memories where this entity was mentioned
    #[serde(default)]
    pub source_memories: Vec<String>,
}

impl SemanticEntity {
    /// Create a new entity with default trust score
    pub fn new(name: String, entity_type: SemanticEntityType) -> Self {
        let now = Utc::now();
        Self {
            id: ulid::Ulid::new().to_string(),
            name,
            entity_type,
            properties: HashMap::new(),
            trust_score: 0.5, // Initial trust score
            last_updated: now,
            created_at: now,
            confirmation_count: 1,
            aliases: Vec::new(),
            source_memories: Vec::new(),
        }
    }

    /// Create a new entity with specific ID (for testing/import)
    pub fn with_id(id: String, name: String, entity_type: SemanticEntityType) -> Self {
        let now = Utc::now();
        Self {
            id,
            name,
            entity_type,
            properties: HashMap::new(),
            trust_score: 0.5,
            last_updated: now,
            created_at: now,
            confirmation_count: 1,
            aliases: Vec::new(),
            source_memories: Vec::new(),
        }
    }

    /// Add a property to the entity
    pub fn add_property(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.properties.insert(key.into(), value);
        self.last_updated = Utc::now();
    }

    /// Get a property value
    pub fn get_property(&self, key: &str) -> Option<&serde_json::Value> {
        self.properties.get(key)
    }

    /// Add an alias
    pub fn add_alias(&mut self, alias: String) {
        if !self.aliases.contains(&alias) {
            self.aliases.push(alias);
            self.last_updated = Utc::now();
        }
    }

    /// Add a source memory ID
    pub fn add_source(&mut self, memory_id: String) {
        if !self.source_memories.contains(&memory_id) {
            self.source_memories.push(memory_id);
            self.confirmation_count += 1;
            self.last_updated = Utc::now();
            // Update trust score based on confirmation count
            self.update_trust_score();
        }
    }

    /// Update trust score based on confirmation count.
    /// Uses a logarithmic scale: trust increases rapidly at first,
    /// then more slowly as confirmations accumulate.
    /// Formula: trust = min(1.0, 0.3 + 0.3 * ln(1 + confirmation_count))
    fn update_trust_score(&mut self) {
        use std::f32::consts::LN_2;
        // ln(1 + n) / ln(2) gives us log2(1 + n)
        // This means: 1 confirmation -> 0.3 + 0.3*1 = 0.6
        //             3 confirmations -> 0.3 + 0.3*2 = 0.9 (capped)
        //             7 confirmations -> ~0.99
        let log_factor = (1.0 + self.confirmation_count as f32).ln() / LN_2;
        self.trust_score = (0.3 + 0.3 * log_factor).min(1.0);
    }

    /// Check if entity matches a name (exact or alias)
    pub fn matches_name(&self, name: &str) -> bool {
        let name_lower = name.to_ascii_lowercase();
        self.name.to_ascii_lowercase() == name_lower
            || self
                .aliases
                .iter()
                .any(|a| a.to_ascii_lowercase() == name_lower)
    }

    /// Get the normalized (lowercase) name
    pub fn normalized_name(&self) -> String {
        self.name.to_ascii_lowercase()
    }

    /// Merge properties from another entity (for entity merging)
    pub fn merge_properties(&mut self, other: &SemanticEntity) {
        for (key, value) in &other.properties {
            if !self.properties.contains_key(key) {
                self.properties.insert(key.clone(), value.clone());
            }
        }
    }

    /// Create a summary of the entity for display
    pub fn summary(&self) -> EntitySummary {
        EntitySummary {
            id: self.id.clone(),
            name: self.name.clone(),
            entity_type: self.entity_type,
            trust_score: self.trust_score,
            confirmation_count: self.confirmation_count,
            last_updated: self.last_updated,
        }
    }
}

/// Lightweight entity summary for list views
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySummary {
    pub id: String,
    pub name: String,
    pub entity_type: SemanticEntityType,
    pub trust_score: f32,
    pub confirmation_count: u32,
    pub last_updated: DateTime<Utc>,
}

/// A relationship/edge between two entities in the semantic graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticRelation {
    /// Unique identifier
    pub id: String,
    /// Source entity ID
    pub source_id: String,
    /// Target entity ID
    pub target_id: String,
    /// Type of relationship (e.g., "works_at", "knows", "part_of")
    pub relation_type: String,
    /// Weight/confidence of this relationship [0.0, 1.0]
    pub weight: f32,
    /// Number of times this relationship has been confirmed
    #[serde(default)]
    pub confirmation_count: u32,
    /// When this relation was first created
    pub created_at: DateTime<Utc>,
    /// When this relation was last updated
    pub updated_at: DateTime<Utc>,
    /// Source memories where this relationship was mentioned
    #[serde(default)]
    pub source_memories: Vec<String>,
    /// Optional metadata (describes the context of the relationship)
    #[serde(default)]
    pub properties: HashMap<String, serde_json::Value>,
}

impl SemanticRelation {
    /// Create a new relation
    pub fn new(source_id: String, target_id: String, relation_type: String) -> Self {
        let now = Utc::now();
        Self {
            id: ulid::Ulid::new().to_string(),
            source_id,
            target_id,
            relation_type,
            weight: 0.5,
            confirmation_count: 1,
            created_at: now,
            updated_at: now,
            source_memories: Vec::new(),
            properties: HashMap::new(),
        }
    }

    /// Add a source memory and update confirmation
    pub fn confirm(&mut self, memory_id: String) {
        if !self.source_memories.contains(&memory_id) {
            self.source_memories.push(memory_id);
            self.confirmation_count += 1;
            self.updated_at = Utc::now();
            // Update weight based on confirmation count
            self.weight = (0.3 + 0.4 * (self.confirmation_count as f32).min(3.0) / 3.0).min(1.0);
        }
    }

    /// Add a property to the relation
    pub fn add_property(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.properties.insert(key.into(), value);
        self.updated_at = Utc::now();
    }
}

/// Statistics about the entity graph
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EntityGraphStats {
    pub total_entities: usize,
    pub total_relations: usize,
    pub by_type: HashMap<String, usize>,
    pub avg_trust_score: f32,
    pub high_trust_count: usize, // entities with trust > 0.7
}

/// Request to create or update an entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertEntityRequest {
    pub name: String,
    #[serde(default)]
    pub entity_type: SemanticEntityType,
    #[serde(default)]
    pub properties: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub source_memory: Option<String>,
}

impl UpsertEntityRequest {
    pub fn new(name: String) -> Self {
        Self {
            name,
            entity_type: SemanticEntityType::Concept,
            properties: HashMap::new(),
            aliases: Vec::new(),
            source_memory: None,
        }
    }

    pub fn with_type(mut self, entity_type: SemanticEntityType) -> Self {
        self.entity_type = entity_type;
        self
    }
}

/// Request to create a relationship between entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertRelationRequest {
    pub source_name: String,
    pub target_name: String,
    pub relation_type: String,
    #[serde(default)]
    pub weight: f32,
    #[serde(default)]
    pub source_memory: Option<String>,
    #[serde(default)]
    pub properties: HashMap<String, serde_json::Value>,
}

impl UpsertRelationRequest {
    pub fn new(source: String, target: String, relation_type: String) -> Self {
        Self {
            source_name: source,
            target_name: target,
            relation_type,
            weight: 0.5,
            source_memory: None,
            properties: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_creation() {
        let entity = SemanticEntity::new("Alice".to_string(), SemanticEntityType::Person);
        assert_eq!(entity.name, "Alice");
        assert_eq!(entity.entity_type, SemanticEntityType::Person);
        assert!(entity.trust_score > 0.0);
    }

    #[test]
    fn test_entity_trust_score_increase() {
        let mut entity = SemanticEntity::new("Bob".to_string(), SemanticEntityType::Person);
        let initial_trust = entity.trust_score;

        entity.add_source("memory-1".to_string());
        entity.add_source("memory-2".to_string());
        entity.add_source("memory-3".to_string());

        assert!(entity.trust_score > initial_trust);
        assert_eq!(entity.confirmation_count, 4); // 1 initial + 3 added
    }

    #[test]
    fn test_entity_aliases() {
        let mut entity = SemanticEntity::new("Robert".to_string(), SemanticEntityType::Person);
        entity.add_alias("Bob".to_string());
        entity.add_alias("Rob".to_string());

        assert!(entity.matches_name("Robert"));
        assert!(entity.matches_name("Bob"));
        assert!(entity.matches_name("rob"));
        assert!(!entity.matches_name("Alice"));
    }

    #[test]
    fn test_relation_creation() {
        let relation = SemanticRelation::new(
            "entity-1".to_string(),
            "entity-2".to_string(),
            "works_at".to_string(),
        );
        assert_eq!(relation.relation_type, "works_at");
        assert_eq!(relation.weight, 0.5);
    }

    #[test]
    fn test_entity_type_from_str() {
        assert_eq!(
            SemanticEntityType::parse("person"),
            Some(SemanticEntityType::Person)
        );
        assert_eq!(
            SemanticEntityType::parse("ORGANIZATION"),
            Some(SemanticEntityType::Organization)
        );
        assert_eq!(SemanticEntityType::parse("invalid"), None);
    }

    #[test]
    fn test_upsert_request() {
        let req = UpsertEntityRequest::new("Test Entity".to_string())
            .with_type(SemanticEntityType::Product);

        assert_eq!(req.name, "Test Entity");
        assert_eq!(req.entity_type, SemanticEntityType::Product);
    }
}
