use serde::{Deserialize, Serialize};

/// Defines the operational boundaries for an agent's change task.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChangeScope {
    /// List of file patterns (globs) that the agent is allowed to modify.
    pub allowed_write: Vec<String>,
    /// List of file patterns (globs) that the agent can only read.
    pub read_only: Vec<String>,
    /// List of file patterns (globs) that the agent is strictly prohibited from accessing.
    pub blocked: Vec<String>,
    /// List of system contracts or interfaces that might be affected by the changes.
    pub contracts_affected: Vec<String>,
    /// List of architectural layers affected by the changes.
    pub layers_affected: Vec<String>,
}
