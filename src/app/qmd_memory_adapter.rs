//! QmdMemory adapter that implements MemoryQueryPort.
//! Wraps QmdMemory (the domain) behind the inbound port interface.

use crate::domain::memory::{
    EvidenceKind, MemoryKind, MemoryNamespace, MemoryProvenance, MemoryQueryFilters,
    MemoryRecord,
};
use crate::memory::qmd_memory::{MemoryDocument, QmdMemory};
use crate::ports::inbound::MemoryQueryPort;
use async_trait::async_trait;
use chrono::Utc;
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

fn doc_to_record(doc: MemoryDocument) -> MemoryRecord {
    MemoryRecord {
        id: doc.id.unwrap_or_default(),
        content: doc.content,
        kind: MemoryKind::Context,
        namespace: MemoryNamespace::Global,
        provenance: MemoryProvenance {
            source: doc.path,
            evidence_kind: EvidenceKind::Direct,
            confidence: 1.0,
        },
        embedding: Some(doc.embedding),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

#[async_trait]
impl MemoryQueryPort for QmdMemoryAdapter {
    async fn search(
        &self,
        query: &str,
        _filters: Option<MemoryQueryFilters>,
    ) -> anyhow::Result<Vec<MemoryRecord>> {
        // Note: we use QmdMemory::search directly to avoid type mismatches
        // between domain::MemoryQueryFilters and schema::MemoryQueryFilters.
        let results = self.inner.search(query, 100).await?;
        Ok(results.into_iter().map(doc_to_record).collect())
    }

    async fn add(&self, record: MemoryRecord) -> anyhow::Result<String> {
        let doc = MemoryDocument {
            id: None,
            path: record.provenance.source.clone(),
            content: record.content,
            metadata: serde_json::Value::Object(serde_json::Map::new()),
            content_vector: None,
            embedding: record.embedding.unwrap_or_default(),
        };
        self.inner
            .add_document(doc.path, doc.content, doc.metadata)
            .await
    }

    async fn delete(&self, id: &str) -> anyhow::Result<Option<MemoryRecord>> {
        let result = self.inner.delete(id).await?;
        Ok(result.map(doc_to_record))
    }

    async fn get(&self, id: &str) -> anyhow::Result<Option<MemoryRecord>> {
        let result = self.inner.get(id).await?;
        Ok(result.map(doc_to_record))
    }

    async fn list(
        &self,
        _namespace: MemoryNamespace,
        _limit: usize,
    ) -> anyhow::Result<Vec<MemoryRecord>> {
        // Not used in current codebase; search is the primary query method.
        Ok(vec![])
    }
}
