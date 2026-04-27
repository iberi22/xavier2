use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecord {
    pub id: String,
    pub content: String,
    pub kind: MemoryKind,
    pub namespace: MemoryNamespace,
    pub provenance: MemoryProvenance,
    pub metadata: serde_json::Value,
    pub embedding: Option<Vec<f32>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MemoryKind {
    Fact,
    Preference,
    Context,
    Task,
    Conversation,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum EvidenceKind {
    Direct,
    Inferred,
    Reported,
    Derived,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MemoryNamespace {
    Global,
    Project,
    Session,
    Ephemeral,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryProvenance {
    pub source: String,
    pub evidence_kind: EvidenceKind,
    pub confidence: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryQueryFilters {
    pub namespace: Option<MemoryNamespace>,
    pub kinds: Option<Vec<MemoryKind>>,
    pub limit: Option<usize>,
    pub min_confidence: Option<f32>,
}
