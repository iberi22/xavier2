use crate::domain::change_control::{AgentTask, ChangeScope, ConflictReport, FileLease, LeaseMode};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseResponse {
    pub status: String,
    pub lease_id: String,
    pub conflicts: Vec<ConflictReport>,
    pub memory_context: Vec<String>,
    pub required_checks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub passed: bool,
    pub violations: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCompletionResult {
    pub task_id: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergePlan {
    pub safe_parallel_groups: Vec<Vec<String>>,
    pub sequential: Vec<String>,
    pub blocked: Vec<BlockedTask>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockedTask {
    pub task: String,
    pub reason: String,
    pub blocked_by: Vec<String>,
}

// ---------------------------------------------------------------------------
// Port trait
// ---------------------------------------------------------------------------

#[async_trait]
pub trait ChangeControlPort: Send + Sync {
    /// Create a new agent task (starts in Draft status).
    async fn create_task(&self, task: AgentTask) -> anyhow::Result<String>;

    /// Retrieve a task by ID.
    async fn get_task(&self, id: &str) -> anyhow::Result<Option<AgentTask>>;

    /// List all tasks.
    async fn list_tasks(&self) -> anyhow::Result<Vec<AgentTask>>;

    /// Claim a file lease for an agent working on a task.
    async fn claim_lease(
        &self,
        agent_id: &str,
        task_id: &str,
        patterns: Vec<String>,
        mode: LeaseMode,
        ttl_seconds: i64,
    ) -> anyhow::Result<LeaseResponse>;

    /// Release a previously claimed lease.
    async fn release_lease(&self, lease_id: &str) -> anyhow::Result<()>;

    /// Return all currently active (non-expired) leases.
    async fn active_leases(&self) -> anyhow::Result<Vec<FileLease>>;

    /// Check for conflicts between the given task and all other active tasks.
    async fn check_conflicts(&self, task_id: &str) -> anyhow::Result<Vec<ConflictReport>>;

    /// Validate a change scope against the current change-control rules.
    async fn validate_change(&self, scope: &ChangeScope) -> anyhow::Result<ValidationResult>;

    /// Mark a task as complete with an optional JSON result payload.
    async fn complete_task(
        &self,
        task_id: &str,
        result: serde_json::Value,
    ) -> anyhow::Result<TaskCompletionResult>;

    /// Build a merge plan that groups tasks into safe parallel / sequential groups.
    async fn merge_plan(&self) -> anyhow::Result<MergePlan>;
}
