use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Represents a temporary reservation of file patterns by an agent for a specific task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileLease {
    pub id: String,
    pub task_id: String,
    pub agent_id: String,
    pub patterns: Vec<String>,
    pub mode: LeaseMode,
    pub expires_at: DateTime<Utc>,
    pub status: LeaseStatus,
}

/// The mode of the file lease, defining what the agent is allowed to do.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LeaseMode {
    Read,
    Write,
    Block,
}

/// The lifecycle status of a FileLease.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LeaseStatus {
    Active,
    Expired,
    Released,
}
