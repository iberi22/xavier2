use crate::domain::memory::MemoryRecord;
use crate::ports::outbound::StoragePort;
use async_trait::async_trait;

pub struct VecStorageAdapter {
    // VecSqliteMemoryStore placeholder
}

impl VecStorageAdapter {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for VecStorageAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StoragePort for VecStorageAdapter {
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
