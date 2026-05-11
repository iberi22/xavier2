use serde::{Deserialize, Serialize};

/// Access mode for a file lease.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeaseMode {
    Read,
    Write,
    Block,
}

/// Lifecycle status of a file lease.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeaseStatus {
    Active,
    Expired,
    Released,
}

/// A lease grants an agent temporary rights over a set of file patterns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileLease {
    pub id: String,
    pub task_id: String,
    pub agent_id: String,
    pub patterns: Vec<String>,
    pub mode: LeaseMode,
    pub expires_at: i64,
    pub status: LeaseStatus,
}
