use crate::domain::memory::MemoryRecord;
use crate::ports::outbound::StoragePort;
use async_trait::async_trait;

pub struct SqliteStorageAdapter {
    // SqliteMemoryStore placeholder
}

impl SqliteStorageAdapter {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for SqliteStorageAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StoragePort for SqliteStorageAdapter {
    async fn put(&self, _record: MemoryRecord) -> anyhow::Result<()> {
        todo!()
    }

    async fn get(&self, _id: &str) -> anyhow::Result<Option<MemoryRecord>> {
        todo!()
    }

    async fn list(&self, _namespace: &str, _limit: usize) -> anyhow::Result<Vec<MemoryRecord>> {
        todo!()
    }

    async fn search(&self, _query: &str, _limit: usize) -> anyhow::Result<Vec<MemoryRecord>> {
        todo!()
    }

    async fn delete(&self, _id: &str) -> anyhow::Result<Option<MemoryRecord>> {
        todo!()
    }
}
