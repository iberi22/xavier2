//! QmdMemory adapter that implements MemoryQueryPort.
//! Wraps QmdMemory (the domain) behind the inbound port interface.
// TODO: HexArch - depends on concrete crate::memory::qmd_memory, should use a port abstraction

use crate::memory::qmd_memory::QmdMemory;
use crate::memory::schema::MemoryQueryFilters;
use crate::memory::store::MemoryRecord;
use crate::ports::inbound::MemoryQueryPort;
use async_trait::async_trait;
use std::sync::Arc;

#[derive(Clone)]
pub struct QmdMemoryAdapter {
    inner: Arc<QmdMemory>,
}

impl QmdMemoryAdapter {
    pub fn new(inner: Arc<QmdMemory>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl MemoryQueryPort for QmdMemoryAdapter {
    async fn search(
        &self,
        query: &str,
        filters: Option<MemoryQueryFilters>,
    ) -> anyhow::Result<Vec<MemoryRecord>> {
        let limit = 100; // Default limit for search
        let results = self
            .inner
            .search_filtered(query, limit, filters.as_ref())
            .await?;

        let workspace_id = self.inner.workspace_id();
        Ok(results
            .into_iter()
            .map(|doc| MemoryRecord::from_document(&workspace_id, &doc, true, None))
            .collect())
    }

    async fn add(&self, record: MemoryRecord) -> anyhow::Result<String> {
        let doc = record.to_document();
        self.inner
            .add_document(doc.path, doc.content, doc.metadata)
            .await
    }

    async fn delete(&self, id: &str) -> anyhow::Result<Option<MemoryRecord>> {
        let workspace_id = self.inner.workspace_id();
        let result = self.inner.delete(id).await?;
        Ok(result.map(|doc| MemoryRecord::from_document(&workspace_id, &doc, true, None)))
    }

    async fn get(&self, id: &str) -> anyhow::Result<Option<MemoryRecord>> {
        let workspace_id = self.inner.workspace_id();
        let result = self.inner.get(id).await?;
        Ok(result.map(|doc| MemoryRecord::from_document(&workspace_id, &doc, true, None)))
    }

    async fn list(&self, workspace_id: &str, limit: usize) -> anyhow::Result<Vec<MemoryRecord>> {
        let limit = limit.max(1).min(100);
        let results = self.inner.all_documents().await;

        Ok(results
            .into_iter()
            .take(limit)
            .map(|doc| MemoryRecord::from_document(workspace_id, &doc, true, None))
            .collect())
    }
}
