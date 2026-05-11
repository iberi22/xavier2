use async_trait::async_trait;
use crate::domain::change_control::*;
use anyhow::Result;

#[async_trait]
pub trait ChangeControlPort: Send + Sync {
    async fn create_task(&self, task: AgentTask) -> Result<String>;
    async fn get_task(&self, id: &str) -> Result<Option<AgentTask>>;
    async fn claim_lease(&self, request: LeaseRequest) -> Result<LeaseResponse>;
    async fn release_lease(&self, lease_id: &str) -> Result<()>;
    async fn active_leases(&self) -> Result<Vec<FileLease>>;
    async fn check_conflicts(&self, task_id: &str) -> Result<Vec<ConflictReport>>;
    async fn validate_change(&self, task_id: &str) -> Result<ValidationReport>;
    async fn complete_task(&self, task_id: &str) -> Result<TaskCompletionReport>;
    async fn merge_plan(&self) -> Result<MergePlan>;
}
