use crate::domain::memory::{MemoryNamespace, MemoryQueryFilters, MemoryRecord};
use crate::ports::inbound::MemoryQueryPort;
use crate::ports::outbound::{EmbeddingPort, StoragePort};
use async_trait::async_trait;

#[allow(dead_code)]
pub struct MemoryService<S: StoragePort, E: EmbeddingPort> {
    storage: S,
    embedding: E,
}

impl<S: StoragePort, E: EmbeddingPort> MemoryService<S, E> {
    pub fn new(storage: S, embedding: E) -> Self {
        Self { storage, embedding }
    }
}

#[async_trait]
impl<S: StoragePort + Send + Sync, E: EmbeddingPort + Send + Sync> MemoryQueryPort
    for MemoryService<S, E>
{
    async fn search(
        &self,
        query: &str,
        filters: Option<MemoryQueryFilters>,
    ) -> anyhow::Result<Vec<MemoryRecord>> {
        let _ = query;
        let _ = filters;
        todo!()
    }

    async fn add(&self, record: MemoryRecord) -> anyhow::Result<String> {
        let _ = record;
        todo!()
    }

    async fn delete(&self, id: &str) -> anyhow::Result<Option<MemoryRecord>> {
        let _ = id;
        todo!()
    }

    async fn get(&self, id: &str) -> anyhow::Result<Option<MemoryRecord>> {
        let _ = id;
        todo!()
    }

    async fn list(
        &self,
        namespace: MemoryNamespace,
        limit: usize,
    ) -> anyhow::Result<Vec<MemoryRecord>> {
        let _ = namespace;
        let _ = limit;
        todo!()
    }
}
