use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseRequest {
    pub agent_id: String,
    pub files_affected: Vec<String>,
    pub task_description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseResponse {
    pub lease_id: String,
    pub memory_context: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCompletionRequest {
    pub agent_id: String,
    pub task_id: String,
    pub path: String,
    pub content: String,
    pub files_changed: Vec<String>,
    pub contracts_affected: Vec<String>,
    pub risk_level: String,
    pub checks_passed: Vec<String>,
    pub pr_link: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentChangeSummary {
    pub task_id: String,
    pub agent_id: String,
    pub files_changed: Vec<String>,
    pub contracts_affected: Vec<String>,
    pub risk_level: String,
    pub checks_passed: Vec<String>,
    pub pr: Option<String>,
}
