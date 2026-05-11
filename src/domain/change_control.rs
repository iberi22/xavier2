use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeTask {
    pub id: String,
    pub agent_id: String,
    pub title: String,
    pub intent: String,
    pub scope: Vec<String>,
    pub status: ChangeTaskStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub result: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ChangeTaskStatus {
    Open,
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeLease {
    pub id: String,
    pub agent_id: String,
    pub task_id: String,
    pub patterns: Vec<String>,
    pub mode: LeaseMode,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LeaseMode {
    Read,
    Write,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseClaimResponse {
    pub status: String,
    pub lease_id: Option<String>,
    pub conflicts: Vec<String>,
    pub memory_context: Vec<String>,
    pub required_checks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictReport {
    pub has_conflicts: bool,
    pub conflicts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergePlan {
    pub ready: bool,
    pub strategies: Vec<String>,
}
