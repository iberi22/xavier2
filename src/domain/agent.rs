use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Active agent entry tracked by the lifecycle registry.
#[derive(Debug, Clone)]
pub struct AgentEntry {
    pub agent_id: String,
    pub session_id: String,
    pub last_heartbeat: DateTime<Utc>,
    pub metadata: AgentMetadata,
}

/// Additional agent metadata accepted during registration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentMetadata {
    pub name: Option<String>,
    pub capabilities: Vec<String>,
    pub role: Option<String>,
    pub endpoint: Option<String>,
}
