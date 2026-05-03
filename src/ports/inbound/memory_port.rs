use crate::memory::schema::MemoryQueryFilters;
use crate::memory::surreal_store::MemoryRecord;
use async_trait::async_trait;

#[async_trait]
pub trait MemoryQueryPort: Send + Sync {
    async fn search(
        &self,
        query: &str,
        filters: Option<MemoryQueryFilters>,
    ) -> anyhow::Result<Vec<MemoryRecord>>;
    async fn add(&self, record: MemoryRecord) -> anyhow::Result<String>;
    async fn delete(&self, id: &str) -> anyhow::Result<Option<MemoryRecord>>;
    async fn get(&self, id: &str) -> anyhow::Result<Option<MemoryRecord>>;
    async fn list(
        &self,
        workspace_id: &str,
        limit: usize,
    ) -> anyhow::Result<Vec<MemoryRecord>>;
}
