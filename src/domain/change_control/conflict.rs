use serde::{Deserialize, Serialize};

/// The kind of conflict detected between two agent tasks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictType {
    DirectFileOverlap,
    ContractBreach,
    DependencyConflict,
    BlockedByPolicy,
}

/// Severity of a conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictSeverity {
    Warning,
    Blocking,
    Critical,
}

/// Reports a conflict between two agent tasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictReport {
    pub task_id: String,
    pub conflicting_task_id: String,
    pub conflict_type: ConflictType,
    pub files: Vec<String>,
    pub contracts: Vec<String>,
    pub severity: ConflictSeverity,
    pub recommendation: String,
}
