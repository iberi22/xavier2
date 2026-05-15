use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BeliefNode {
    pub id: String,
    pub concept: String,
    pub confidence: f32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BeliefEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub relation_type: String,
    pub weight: f32,
    pub confidence_score: f32,
    pub provenance_id: String,
    pub contradicts_edge_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl BeliefEdge {
    pub fn new(
        source: String,
        target: String,
        relation_type: String,
        confidence_score: f32,
        provenance_id: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: ulid::Ulid::new().to_string(),
            source,
            target,
            relation_type,
            weight: confidence_score,
            confidence_score,
            provenance_id,
            contradicts_edge_id: None,
            created_at: now,
            updated_at: now,
        }
    }
}
