//! Domain models for Xavier's Belief Graph.
//!
//! Provides strict schema definitions for Entities and Relationships
//! to ensure determinism and prevent entity drift.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Strict entity types for the Belief Graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphEntityType {
    Person,
    Organization,
    Location,
    Product,
    Concept,
    Event,
    TechnicalTerm,
    Tool,
}

impl GraphEntityType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Person => "person",
            Self::Organization => "organization",
            Self::Location => "location",
            Self::Product => "product",
            Self::Concept => "concept",
            Self::Event => "event",
            Self::TechnicalTerm => "technical_term",
            Self::Tool => "tool",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "person" => Some(Self::Person),
            "organization" | "org" => Some(Self::Organization),
            "location" => Some(Self::Location),
            "product" => Some(Self::Product),
            "concept" => Some(Self::Concept),
            "event" => Some(Self::Event),
            "technical_term" | "tech_term" => Some(Self::TechnicalTerm),
            "tool" => Some(Self::Tool),
            _ => None,
        }
    }
}

/// Strict relationship types for the Belief Graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphRelationshipType {
    WorksAt,
    LocatedIn,
    PartOf,
    IsA,
    Uses,
    CreatedBy,
    RelatedTo,
    CollaboratesWith,
    MemberOf,
    AcquiredBy,
    Supports,
    CompetesWith,
}

impl GraphRelationshipType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::WorksAt => "works_at",
            Self::LocatedIn => "located_in",
            Self::PartOf => "part_of",
            Self::IsA => "is_a",
            Self::Uses => "uses",
            Self::CreatedBy => "created_by",
            Self::RelatedTo => "related_to",
            Self::CollaboratesWith => "collaborates_with",
            Self::MemberOf => "member_of",
            Self::AcquiredBy => "acquired_by",
            Self::Supports => "supports",
            Self::CompetesWith => "competes_with",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "works_at" => Some(Self::WorksAt),
            "located_in" => Some(Self::LocatedIn),
            "part_of" => Some(Self::PartOf),
            "is_a" => Some(Self::IsA),
            "uses" => Some(Self::Uses),
            "created_by" => Some(Self::CreatedBy),
            "related_to" => Some(Self::RelatedTo),
            "collaborates_with" => Some(Self::CollaboratesWith),
            "member_of" => Some(Self::MemberOf),
            "acquired_by" => Some(Self::AcquiredBy),
            "supports" => Some(Self::Supports),
            "competes_with" => Some(Self::CompetesWith),
            _ => None,
        }
    }
}

/// A node in the Belief Graph representing a resolved entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEntity {
    pub id: String,
    pub name: String,
    pub normalized_name: String,
    pub entity_type: GraphEntityType,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub description: Option<String>,
    pub trust_score: f32,
    pub confirmation_count: u32,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl GraphEntity {
    pub fn new(name: String, entity_type: GraphEntityType) -> Self {
        let now = Utc::now();
        let normalized_name = Self::normalize(&name);
        Self {
            id: ulid::Ulid::new().to_string(),
            name,
            normalized_name,
            entity_type,
            aliases: Vec::new(),
            description: None,
            trust_score: 0.5,
            confirmation_count: 1,
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn normalize(name: &str) -> String {
        name.trim().to_lowercase()
    }

    pub fn add_alias(&mut self, alias: String) {
        let normalized = Self::normalize(&alias);
        if normalized != self.normalized_name && !self.aliases.contains(&normalized) {
            self.aliases.push(normalized);
            self.updated_at = Utc::now();
        }
    }

    pub fn confirm(&mut self) {
        self.confirmation_count += 1;
        // Simple trust score update: 0.5 -> 0.7 -> 0.83 -> 0.9 -> ...
        self.trust_score = (self.trust_score + 0.2 * (1.0 - self.trust_score)).min(1.0);
        self.updated_at = Utc::now();
    }
}

/// An edge in the Belief Graph representing a directed relationship between entities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphRelationship {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub relation_type: GraphRelationshipType,
    pub weight: f32,
    pub confirmation_count: u32,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl GraphRelationship {
    pub fn new(
        source_id: String,
        target_id: String,
        relation_type: GraphRelationshipType,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: ulid::Ulid::new().to_string(),
            source_id,
            target_id,
            relation_type,
            weight: 0.5,
            confirmation_count: 1,
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn confirm(&mut self) {
        self.confirmation_count += 1;
        self.weight = (self.weight + 0.2 * (1.0 - self.weight)).min(1.0);
        self.updated_at = Utc::now();
    }
}
