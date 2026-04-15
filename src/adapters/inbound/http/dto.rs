use crate::domain::pattern::{PatternCategory, PatternVerification};
use crate::domain::security::ThreatLevel;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct PatternDiscoverRequest {
    pub pattern: String,
    pub category: PatternCategory,
    pub project: String,
    pub confidence: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PatternResponse {
    pub id: String,
    pub category: PatternCategory,
    pub pattern: String,
    pub confidence: f32,
    pub verification: PatternVerification,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SecurityScanRequest {
    pub target: String,
    pub level: Option<ThreatLevel>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SecurityScanResponse {
    pub id: String,
    pub threats_count: usize,
    pub scan_duration_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryAddRequest {
    pub content: String,
    pub namespace: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MemorySearchRequest {
    pub query: String,
    pub namespace: Option<String>,
    pub limit: Option<usize>,
}
