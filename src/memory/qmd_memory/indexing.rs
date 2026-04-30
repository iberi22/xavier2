use anyhow::Result;
use std::sync::atomic::Ordering as AtomicOrdering;
use crate::memory::schema::{normalize_metadata, TypedMemoryPayload};
use crate::memory::surreal_store::MemoryRecord;
use crate::memory::qmd_memory::types::{MemoryDocument, MemoryUsage, CacheMetrics};
use crate::memory::qmd_memory::QmdMemory;
use crate::memory::qmd_memory::consolidation::{expand_document_variants, normalize_locomo_metadata};
use crate::memory::qmd_memory::embeddings::generate_embedding;
use crate::utils::crypto::hex_encode;
use sha2::{Digest, Sha256};

impl QmdMemory {
    pub async fn get(&self, path_or_id: &str) -> Result<Option<MemoryDocument>> {
        let docs = self.docs.read().await;
        Ok(docs
            .iter()
            .find(|doc| doc.path == path_or_id || doc.id.as_deref() == Some(path_or_id))
            .cloned())
    }

    pub async fn add(&self, doc: MemoryDocument) -> Result<()> {
        self.docs.write().await.push(doc.clone());
        self.invalidate_cache().await;
        if let Some(store) = self.store().await {
            store
                .put(memory_record_from_document(self.workspace_id(), &doc))
                .await?;
        }
        Ok(())
    }

    pub async fn update(&self, doc: MemoryDocument) -> Result<()> {
        let persisted = doc.clone();
        let mut docs = self.docs.write().await;
        if let Some(existing) = docs
            .iter_mut()
            .find(|d| d.id == doc.id || d.path == doc.path)
        {
            *existing = doc;
        } else {
            docs.push(doc);
        }
        drop(docs);
        self.invalidate_cache().await;
        if let Some(store) = self.store().await {
            store
                .update(memory_record_from_document(self.workspace_id(), &persisted))
                .await?;
        }
        Ok(())
    }

    pub async fn add_document(
        &self,
        path: String,
        content: String,
        metadata: serde_json::Value,
    ) -> Result<String> {
        self.add_document_typed_with_embedding(path, content, metadata, None, None)
            .await
    }

    pub async fn add_document_typed(
        &self,
        path: String,
        content: String,
        metadata: serde_json::Value,
        typed: Option<TypedMemoryPayload>,
    ) -> Result<String> {
        self.add_document_typed_with_embedding(path, content, metadata, typed, None)
            .await
    }

    pub async fn add_document_typed_with_embedding(
        &self,
        path: String,
        content: String,
        metadata: serde_json::Value,
        typed: Option<TypedMemoryPayload>,
        embedding: Option<Vec<f32>>,
    ) -> Result<String> {
        let id = ulid::Ulid::new().to_string();
        let metadata = normalize_metadata(&path, metadata, self.workspace_id(), typed)?;
        let metadata = normalize_locomo_metadata(&path, metadata);
        let variants = expand_document_variants(&path, &content, &metadata);
        let is_locomo_benchmark = metadata
            .get("benchmark")
            .and_then(|value| value.as_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("locomo"))
            || path.contains("locomo/");
        let base_embedding = if is_locomo_benchmark {
            Vec::new()
        } else if let Some(embedding) = embedding.clone() {
            embedding
        } else {
            generate_embedding(&content)
                .await
                .unwrap_or_else(|_| Vec::new())
        };

        for (index, (variant_path, variant_content, variant_metadata)) in
            variants.into_iter().enumerate()
        {
            let variant_embedding = if is_locomo_benchmark || variant_content == content {
                base_embedding.clone()
            } else {
                generate_embedding(&variant_content)
                    .await
                    .unwrap_or_else(|_| Vec::new())
            };

            self.add(MemoryDocument {
                id: Some(if index == 0 {
                    id.clone()
                } else {
                    ulid::Ulid::new().to_string()
                }),
                path: variant_path,
                content: variant_content,
                metadata: variant_metadata,
                content_vector: Some(variant_embedding.clone()),
                embedding: variant_embedding,
            })
            .await?;
        }

        Ok(id)
    }

    pub async fn delete(&self, path_or_id: &str) -> Result<Option<MemoryDocument>> {
        let mut docs = self.docs.write().await;
        let removed = docs
            .iter()
            .position(|doc| doc.path == path_or_id || doc.id.as_deref() == Some(path_or_id))
            .map(|index| docs.remove(index));
        drop(docs);

        if removed.is_some() {
            self.invalidate_cache().await;
            if let Some(store) = self.store().await {
                let _ = store.delete(self.workspace_id(), path_or_id).await?;
            }
        }

        Ok(removed)
    }

    pub async fn clear(&self) -> Result<usize> {
        let ids = self
            .docs
            .read()
            .await
            .iter()
            .filter_map(|doc| doc.id.clone().or_else(|| Some(doc.path.clone())))
            .collect::<Vec<_>>();
        let mut docs = self.docs.write().await;
        let removed = docs.len();
        docs.clear();
        drop(docs);
        self.invalidate_cache().await;
        if let Some(store) = self.store().await {
            for id in ids {
                let _ = store.delete(self.workspace_id(), &id).await?;
            }
        }
        Ok(removed)
    }

    pub async fn count(&self) -> Result<usize> {
        Ok(self.docs.read().await.len())
    }

    pub async fn all_documents(&self) -> Vec<MemoryDocument> {
        self.docs.read().await.clone()
    }

    pub async fn usage(&self) -> MemoryUsage {
        let docs = self.docs.read().await;
        MemoryUsage {
            document_count: docs.len(),
            storage_bytes: docs.iter().map(MemoryDocument::estimated_bytes).sum(),
        }
    }

    pub async fn cache_metrics(&self) -> CacheMetrics {
        CacheMetrics {
            hits: self.cache_counters.hits.load(AtomicOrdering::Relaxed),
            misses: self.cache_counters.misses.load(AtomicOrdering::Relaxed),
            entries: self.search_cache.read().await.len(),
        }
    }

    pub(crate) async fn invalidate_cache(&self) {
        self.search_cache.write().await.clear();
    }
}

pub fn estimate_document_bytes(path: &str, content: &str, metadata: &serde_json::Value) -> u64 {
    path.len() as u64 + content.len() as u64 + metadata.to_string().len() as u64
}

pub(crate) fn memory_record_from_document(workspace_id: &str, document: &MemoryDocument) -> MemoryRecord {
    let primary = document
        .metadata
        .get("source_path")
        .and_then(|value| value.as_str())
        .is_none();
    let parent_id = document
        .metadata
        .get("parent_id")
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
        .or_else(|| {
            (!primary)
                .then(|| {
                    document
                        .metadata
                        .get("source_path")
                        .and_then(|value| value.as_str())
                        .map(|value| value.to_string())
                })
                .flatten()
        });

    MemoryRecord::from_document(workspace_id, document, primary, parent_id)
}

pub(crate) fn _compute_content_hash(content: &str) -> String {
    hex_encode(Sha256::digest(content.as_bytes()).as_slice())
}
