use serde::{Deserialize, Serialize};

/// Represents a detected conflict between multiple agent tasks or system policies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictReport {
    pub task_id: String,
    pub conflicting_task_id: Option<String>,
    pub conflict_type: ConflictType,
    pub files: Vec<String>,
    pub contracts: Vec<String>,
    pub severity: String,
    pub recommendation: String,
}

/// The type of conflict detected.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConflictType {
    DirectFileOverlap,
    ContractBreach,
    DependencyConflict,
    BlockedByPolicy,
}
