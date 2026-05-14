use std::sync::Arc;
use async_trait::async_trait;
use crate::domain::memory::{MemoryQueryFilters, MemoryRecord};
use crate::ports::inbound::MemoryQueryPort;
use crate::ports::outbound::ThreatDetectionPort;
use tracing::warn;

pub struct MemoryUseCase {
    inner: Arc<dyn MemoryQueryPort>,
    threat_detector: Option<Arc<dyn ThreatDetectionPort>>,
}

impl MemoryUseCase {
    pub fn new(inner: Arc<dyn MemoryQueryPort>, threat_detector: Option<Arc<dyn ThreatDetectionPort>>) -> Self {
        Self {
            inner,
            threat_detector,
        }
    }
}

#[async_trait]
impl MemoryQueryPort for MemoryUseCase {
    async fn search(
        &self,
        query: &str,
        filters: Option<MemoryQueryFilters>,
    ) -> anyhow::Result<Vec<MemoryRecord>> {
        if let Some(ref detector) = self.threat_detector {
            let clean = detector.scan_and_log(query, "memory_search").await?;
            if !clean {
                warn!("Memory search blocked: security threat detected in query");
                return Err(anyhow::anyhow!("Security policy violation detected in search query"));
            }
        }
        self.inner.search(query, filters).await
    }

    async fn add(&self, record: MemoryRecord) -> anyhow::Result<String> {
        if let Some(ref detector) = self.threat_detector {
            let clean = detector.scan_and_log(&record.content, "memory_add").await?;
            if !clean {
                warn!("Memory add blocked: security threat detected in content");
                return Err(anyhow::anyhow!("Security policy violation detected in memory content"));
            }
        }
        self.inner.add(record).await
    }

    async fn delete(&self, id: &str) -> anyhow::Result<Option<MemoryRecord>> {
        self.inner.delete(id).await
    }

    async fn get(&self, id: &str) -> anyhow::Result<Option<MemoryRecord>> {
        self.inner.get(id).await
    }

    async fn list(&self, workspace_id: &str, limit: usize) -> anyhow::Result<Vec<MemoryRecord>> {
        self.inner.list(workspace_id, limit).await
    }
}
