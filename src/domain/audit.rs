use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub component: String,
    pub event_type: String,
    pub severity: AuditSeverity,
    pub message: String,
    pub metadata: serde_json::Value,
    pub hash: String,
    pub prev_hash: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AuditSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

impl AuditSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuditSeverity::Critical => "critical",
            AuditSeverity::High => "high",
            AuditSeverity::Medium => "medium",
            AuditSeverity::Low => "low",
            AuditSeverity::Info => "info",
        }
    }
}
