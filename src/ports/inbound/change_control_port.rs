use crate::domain::change_control::{LeaseRequest, LeaseResponse, TaskCompletionRequest};
use async_trait::async_trait;

#[async_trait]
pub trait ChangeControlPort: Send + Sync {
    async fn claim_lease(&self, request: LeaseRequest) -> anyhow::Result<LeaseResponse>;
    async fn complete_task(&self, request: TaskCompletionRequest) -> anyhow::Result<String>;
}
