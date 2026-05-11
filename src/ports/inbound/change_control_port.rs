use crate::domain::change_control::{
    ChangeLease, ChangeTask, ConflictReport, LeaseClaimResponse, LeaseMode, MergePlan,
    ValidationResult,
};
use async_trait::async_trait;

#[async_trait]
pub trait ChangeControlPort: Send + Sync {
    async fn create_task(
        &self,
        agent_id: String,
        title: String,
        intent: String,
        scope: Vec<String>,
    ) -> anyhow::Result<ChangeTask>;

    async fn get_task(&self, id: String) -> anyhow::Result<Option<ChangeTask>>;

    async fn claim_lease(
        &self,
        agent_id: String,
        task_id: String,
        patterns: Vec<String>,
        mode: LeaseMode,
        ttl_seconds: u64,
    ) -> anyhow::Result<LeaseClaimResponse>;

    async fn release_lease(&self, lease_id: String) -> anyhow::Result<bool>;

    async fn get_active_leases(&self) -> anyhow::Result<Vec<ChangeLease>>;

    async fn check_conflicts(&self, task_id: String, scope: Vec<String>) -> anyhow::Result<ConflictReport>;

    async fn validate_task(&self, task_id: String) -> anyhow::Result<ValidationResult>;

    async fn complete_task(&self, task_id: String, result: String) -> anyhow::Result<ChangeTask>;

    async fn get_merge_plan(&self) -> anyhow::Result<MergePlan>;
}
