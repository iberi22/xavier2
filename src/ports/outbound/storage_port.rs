use crate::domain::memory::MemoryRecord;
use async_trait::async_trait;

#[async_trait]
pub trait StoragePort: Send + Sync {
    async fn put(&self, record: MemoryRecord) -> anyhow::Result<()>;
    async fn get(&self, id: &str) -> anyhow::Result<Option<MemoryRecord>>;
    async fn list(&self, namespace: &str, limit: usize) -> anyhow::Result<Vec<MemoryRecord>>;
    async fn search(&self, query: &str, limit: usize) -> anyhow::Result<Vec<MemoryRecord>>;
    async fn delete(&self, id: &str) -> anyhow::Result<Option<MemoryRecord>>;
}
