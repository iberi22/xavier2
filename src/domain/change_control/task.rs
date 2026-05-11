use serde::{Deserialize, Serialize};
use super::scope::ChangeScope;

/// Represents a unit of work assigned to an agent that involves modifying the codebase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    pub id: String,
    pub title: String,
    pub capability: String,
    pub agent_id: Option<String>,
    pub status: AgentTaskStatus,
    pub intent: String,
    pub scope: ChangeScope,
    pub risk_level: String,
    pub dependencies: Vec<String>,
    pub memory_refs: Vec<String>,
}

/// The lifecycle status of an AgentTask.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AgentTaskStatus {
    Draft,
    Claimed,
    Active,
    Completed,
    Failed,
    Cancelled,
}
