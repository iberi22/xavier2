use crate::tasks::session_sync_task::SyncCheckResult;
use async_trait::async_trait;

#[async_trait]
pub trait SessionSyncPort: Send + Sync {
    async fn check(&self) -> anyhow::Result<SyncCheckResult>;
    async fn last_result(&self) -> SyncCheckResult;
}
