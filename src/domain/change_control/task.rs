use serde::{Deserialize, Serialize};

/// Status of an agent task in the change-control lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentTaskStatus {
    Draft,
    Claimed,
    Active,
    Completed,
    Failed,
    Cancelled,
}

/// Severity / criticality of a proposed change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// A task describing an autonomous agent change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    pub id: String,
    pub title: String,
    pub capability: String,
    pub agent_id: String,
    pub status: AgentTaskStatus,
    pub intent: String,
    pub scope: super::scope::ChangeScope,
    pub risk_level: RiskLevel,
    pub dependencies: Vec<String>,
    pub memory_refs: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
}
