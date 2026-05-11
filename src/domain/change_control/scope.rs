use serde::{Deserialize, Serialize};

/// Defines the boundary of a change — what an agent may touch, read, or must avoid.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeScope {
    pub allowed_write: Vec<String>,
    pub read_only: Vec<String>,
    pub blocked: Vec<String>,
    pub contracts_affected: Vec<String>,
    pub layers_affected: Vec<String>,
}
