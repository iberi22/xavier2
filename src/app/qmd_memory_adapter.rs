//! QmdMemory adapter that implements MemoryQueryPort.
//! Wraps QmdMemory (the domain) behind the inbound port interface.

use crate::domain::memory::{
    EvidenceKind, MemoryKind, MemoryNamespace, MemoryProvenance, MemoryQueryFilters, MemoryRecord,
};
use crate::memory::qmd_memory::{MemoryDocument, QmdMemory};
use crate::memory::schema as qmd_schema;
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
    let resolved = qmd_schema::resolve_metadata(&doc.path, &doc.metadata, "", None).ok();
    MemoryRecord {
        id: doc.id.unwrap_or_default(),
        content: doc.content,
        kind: resolved
            .as_ref()
            .map(|metadata| match metadata.kind {
                qmd_schema::MemoryKind::Fact => MemoryKind::Fact,
                qmd_schema::MemoryKind::Task => MemoryKind::Task,
                qmd_schema::MemoryKind::Session => MemoryKind::Conversation,
                _ => MemoryKind::Context,
            })
            .unwrap_or(MemoryKind::Context),
        namespace: resolved
            .as_ref()
            .and_then(|metadata| {
                let namespace = &metadata.namespace;
                if namespace.session_id.is_some() {
                    Some(MemoryNamespace::Session)
                } else if namespace.project.is_some() {
                    Some(MemoryNamespace::Project)
                } else if namespace.scope.as_deref() == Some("ephemeral") {
                    Some(MemoryNamespace::Ephemeral)
                } else {
                    None
                }
            })
            .unwrap_or(MemoryNamespace::Global),
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

fn domain_filters_to_qmd(filters: &MemoryQueryFilters) -> Option<qmd_schema::MemoryQueryFilters> {
    let mut qmd_filters = qmd_schema::MemoryQueryFilters::default();

    if let Some(kinds) = &filters.kinds {
        let kinds = kinds
            .iter()
            .filter_map(|kind| match kind {
                MemoryKind::Fact => Some(qmd_schema::MemoryKind::Fact),
                MemoryKind::Task => Some(qmd_schema::MemoryKind::Task),
                MemoryKind::Conversation => Some(qmd_schema::MemoryKind::Session),
                MemoryKind::Context => Some(qmd_schema::MemoryKind::Document),
                MemoryKind::Preference => None,
            })
            .collect::<Vec<_>>();
        if !kinds.is_empty() {
            qmd_filters.kinds = Some(kinds);
        }
    }

    if matches!(filters.namespace, Some(MemoryNamespace::Ephemeral)) {
        qmd_filters.scope = Some("ephemeral".to_string());
    }

    (qmd_filters != qmd_schema::MemoryQueryFilters::default()).then_some(qmd_filters)
}

fn matches_domain_filters(record: &MemoryRecord, filters: Option<&MemoryQueryFilters>) -> bool {
    let Some(filters) = filters else {
        return true;
    };

    if let Some(namespace) = filters.namespace {
        if record.namespace != namespace {
            return false;
        }
    }
    if let Some(kinds) = &filters.kinds {
        if !kinds.contains(&record.kind) {
            return false;
        }
    }
    if let Some(min_confidence) = filters.min_confidence {
        if record.provenance.confidence < min_confidence {
            return false;
        }
    }

    true
}

#[async_trait]
impl MemoryQueryPort for QmdMemoryAdapter {
    async fn search(
        &self,
        query: &str,
        filters: Option<MemoryQueryFilters>,
    ) -> anyhow::Result<Vec<MemoryRecord>> {
        let limit = filters
            .as_ref()
            .and_then(|filters| filters.limit)
            .unwrap_or(100)
            .max(1)
            .min(100);
        let qmd_filters = filters.as_ref().and_then(domain_filters_to_qmd);
        let results = self
            .inner
            .search_filtered(query, limit, qmd_filters.as_ref())
            .await?;
        Ok(results
            .into_iter()
            .map(doc_to_record)
            .filter(|record| matches_domain_filters(record, filters.as_ref()))
            .take(limit)
            .collect())
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
        namespace: MemoryNamespace,
        limit: usize,
    ) -> anyhow::Result<Vec<MemoryRecord>> {
        let limit = limit.max(1).min(100);
        let filters = MemoryQueryFilters {
            namespace: Some(namespace),
            kinds: None,
            limit: Some(limit),
            min_confidence: None,
        };
        Ok(self
            .inner
            .all_documents()
            .await
            .into_iter()
            .map(doc_to_record)
            .filter(|record| matches_domain_filters(record, Some(&filters)))
            .take(limit)
            .collect())
    }
}
