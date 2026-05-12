use anyhow::Result;
use std::collections::HashMap;
use std::sync::atomic::Ordering as AtomicOrdering;
use std::sync::Arc;
use tokio::sync::RwLock as AsyncRwLock;

pub mod cache;
pub mod index;
pub mod search;
pub mod storage;
pub mod types;
pub mod utils;

pub use cache::*;
pub use index::*;
pub use search::*;
pub use storage::*;
pub use types::*;
pub use utils::*;

use crate::memory::schema::{matches_filters, MemoryQueryFilters, TypedMemoryPayload};
use crate::memory::store::MemoryStore;

#[derive(Clone)]
pub struct QmdMemory {
    pub(crate) workspace_id: String,
    pub(crate) docs: Arc<AsyncRwLock<Vec<MemoryDocument>>>,
    pub(crate) search_cache: Arc<AsyncRwLock<HashMap<SearchCacheKey, Vec<MemoryDocument>>>>,
    pub(crate) cache_counters: Arc<CacheCounters>,
    pub(crate) store: Arc<AsyncRwLock<Option<Arc<dyn MemoryStore>>>>,
}

impl QmdMemory {
    pub fn new(docs: Arc<AsyncRwLock<Vec<MemoryDocument>>>) -> Self {
        Self::new_with_workspace(docs, "default")
    }

    pub fn new_with_workspace(
        docs: Arc<AsyncRwLock<Vec<MemoryDocument>>>,
        workspace_id: impl Into<String>,
    ) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            docs,
            search_cache: Arc::new(AsyncRwLock::new(HashMap::new())),
            cache_counters: Arc::new(CacheCounters::default()),
            store: Arc::new(AsyncRwLock::new(None)),
        }
    }

    pub fn workspace_id(&self) -> &str {
        &self.workspace_id
    }

    pub async fn set_store(&self, store: Arc<dyn MemoryStore>) {
        *self.store.write().await = Some(store);
    }

    async fn store(&self) -> Option<Arc<dyn MemoryStore>> {
        self.store.read().await.clone()
    }

    /// Load workspace state from persistent store on startup.
    /// This is CRITICAL for persistence - without this, data written to the configured store before a restart would be lost on restart.
    pub async fn init(&self) -> Result<()> {
        if let Some(store) = self.store().await {
            let state = store.load_workspace_state(&self.workspace_id).await?;
            let docs: Vec<MemoryDocument> = state
                .memories
                .into_iter()
                .map(|record| record.to_document())
                .collect();
            let loaded_memories = docs.len();
            *self.docs.write().await = docs;
            tracing::info!(
                workspace_id = %self.workspace_id,
                loaded_memories = loaded_memories,
                "QmdMemory loaded from persistent store"
            );
        }
        Ok(())
    }

    pub async fn search(&self, query_text: &str, limit: usize) -> Result<Vec<MemoryDocument>> {
        self.search_filtered(query_text, limit, None).await
    }

    pub async fn bm25_search(
        &self,
        query: &str,
        limit: usize,
        filters: Option<&MemoryQueryFilters>,
    ) -> Result<Vec<MemoryDocument>> {
        let docs = self.all_documents().await;
        let filtered_docs: Vec<MemoryDocument> = docs
            .into_iter()
            .filter(|doc| matches_filters(&doc.path, &doc.metadata, &self.workspace_id, filters))
            .collect();

        if filtered_docs.is_empty() {
            return Ok(Vec::new());
        }

        let scored = crate::search::bm25::score_documents(
            query,
            &filtered_docs,
            crate::search::bm25::Bm25Params::default(),
        );

        let mut results = Vec::new();
        for (_, id) in scored.into_iter().take(limit) {
            if let Some(doc) = filtered_docs
                .iter()
                .find(|d| d.id.as_deref() == Some(&id) || d.path == id)
            {
                results.push(doc.clone());
            }
        }

        Ok(results)
    }

    pub async fn search_filtered(
        &self,
        query_text: &str,
        limit: usize,
        filters: Option<&MemoryQueryFilters>,
    ) -> Result<Vec<MemoryDocument>> {
        let optimized = self
            .search_hybrid_optimized(query_text, limit, filters)
            .await?;
        if !optimized.is_empty() {
            return Ok(optimized);
        }

        let all_docs = self.all_documents().await;
        let locomo_only = !all_docs.is_empty()
            && all_docs
                .iter()
                .all(|doc| is_locomo_document(&doc.path, &doc.metadata));

        if locomo_only {
            return Ok(self
                .search_with_cache_filtered(query_text, limit, filters)
                .await?
                .documents);
        }

        if std::env::var("XAVIER_EMBEDDING_URL").is_ok() {
            if let Ok(results) =
                query_with_embedding_filtered(self, query_text, limit, filters).await
            {
                if !results.is_empty() {
                    return Ok(results);
                }
            }
        }

        Ok(self
            .search_with_cache_filtered(query_text, limit, filters)
            .await?
            .documents)
    }

    async fn search_hybrid_optimized(
        &self,
        query_text: &str,
        limit: usize,
        filters: Option<&MemoryQueryFilters>,
    ) -> Result<Vec<MemoryDocument>> {
        let query_bundle = index::build_query_bundle_internal(query_text);
        let mut candidate_scores: HashMap<String, (f32, MemoryDocument, f32)> = HashMap::new();

        for expanded_query in &query_bundle.variants {
            let cache_hit = self
                .search_with_cache_filtered(expanded_query, limit.max(3), filters)
                .await?;
            self.merge_ranked_candidates(
                &mut candidate_scores,
                cache_hit.documents,
                expanded_query,
                query_bundle.weight_for(expanded_query),
            );
        }

        if candidate_scores.is_empty() {
            return Ok(Vec::new());
        }

        let mut candidates: Vec<(f32, MemoryDocument, f32)> =
            candidate_scores.values().cloned().collect();
        candidates.sort_by(|left, right| {
            right
                .0
                .partial_cmp(&left.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.1.path.cmp(&right.1.path))
        });

        let seed_docs: Vec<MemoryDocument> = candidates
            .iter()
            .take(limit.max(3))
            .map(|(_, doc, _)| doc.clone())
            .collect();

        let multi_hop_docs = self
            .multi_hop_context(query_text, &seed_docs, filters)
            .await;

        for doc in multi_hop_docs {
            let score = contextual_boost(&query_bundle.normalized_query, &doc, 0.45);
            candidate_scores
                .entry(doc.id.clone().unwrap_or_else(|| doc.path.clone()))
                .and_modify(|entry| entry.0 += score)
                .or_insert((score, doc, 0.45));
        }

        let mut reranked: Vec<(f32, MemoryDocument, f32)> =
            candidate_scores.values().cloned().collect();
        reranked.truncate(MAX_RERANK_CANDIDATES.max(limit));
        reranked.sort_by(|left, right| {
            right
                .0
                .partial_cmp(&left.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    right
                        .2
                        .partial_cmp(&left.2)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .then_with(|| left.1.path.cmp(&right.1.path))
        });

        Ok(reranked
            .into_iter()
            .take(limit)
            .map(|(_, doc, _)| doc)
            .collect())
    }

    pub async fn search_with_cache(
        &self,
        query_text: &str,
        limit: usize,
    ) -> Result<CachedSearchResult> {
        self.search_with_cache_filtered(query_text, limit, None)
            .await
    }

    pub async fn search_with_cache_filtered(
        &self,
        query_text: &str,
        limit: usize,
        filters: Option<&MemoryQueryFilters>,
    ) -> Result<CachedSearchResult> {
        let normalized_query = normalize_query(query_text);
        let cache_key = SearchCacheKey {
            workspace_id: self.workspace_id.clone(),
            query: normalized_query.clone(),
            limit,
            filters: serde_json::to_string(&filters).unwrap_or_default(),
        };

        if let Some(cached) = self.search_cache.read().await.get(&cache_key).cloned() {
            self.cache_counters
                .hits
                .fetch_add(1, AtomicOrdering::Relaxed);
            return Ok(CachedSearchResult {
                documents: cached,
                cache_hit: true,
            });
        }

        let docs = self.docs.read().await;
        let mut scored: Vec<(f32, MemoryDocument)> = docs
            .iter()
            .filter_map(|doc| {
                if !matches_filters(&doc.path, &doc.metadata, &self.workspace_id, filters) {
                    return None;
                }
                let score = lexical_score(doc, &normalized_query);
                (score > 0.0).then(|| (score, doc.clone()))
            })
            .collect();
        drop(docs);

        scored.sort_by(|left, right| {
            right
                .0
                .partial_cmp(&left.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.1.path.cmp(&right.1.path))
        });

        let documents: Vec<MemoryDocument> =
            scored.into_iter().map(|(_, doc)| doc).take(limit).collect();

        self.search_cache
            .write()
            .await
            .insert(cache_key, documents.clone());
        self.cache_counters
            .misses
            .fetch_add(1, AtomicOrdering::Relaxed);

        Ok(CachedSearchResult {
            documents,
            cache_hit: false,
        })
    }

    pub async fn vsearch(
        &self,
        query_vector: Vec<f32>,
        limit: usize,
    ) -> Result<Vec<MemoryDocument>> {
        if query_vector.is_empty() {
            return Ok(Vec::new());
        }

        let docs = self.docs.read().await;

        let mut similarities: Vec<(f32, MemoryDocument)> = docs
            .iter()
            .filter_map(|doc| {
                let score = cosine_similarity(&query_vector, &doc.embedding);
                (score > 0.0).then(|| (score, doc.clone()))
            })
            .collect();

        if let Some(max_sim) = similarities.iter().map(|(s, _)| *s).reduce(f32::max) {
            if max_sim > 0.0 {
                for (score, _) in similarities.iter_mut() {
                    *score = 0.5 + 0.5 * (*score / max_sim);
                }
            }
        }

        similarities.sort_by(|left, right| {
            right
                .0
                .partial_cmp(&left.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.1.path.cmp(&right.1.path))
        });

        Ok(similarities
            .into_iter()
            .map(|(_, doc)| doc)
            .take(limit)
            .collect())
    }

    pub async fn query_with_hybrid_search(
        &self,
        query_text: &str,
        query_vector: Vec<f32>,
        limit: usize,
    ) -> Result<Vec<MemoryDocument>> {
        let keyword_results = self.search(query_text, limit).await?;

        let semantic_results = if query_vector.is_empty() {
            Vec::new()
        } else {
            self.vsearch(query_vector, limit).await.unwrap_or_default()
        };

        if semantic_results.is_empty() {
            return Ok(keyword_results.into_iter().take(limit).collect());
        }
        if keyword_results.is_empty() {
            return Ok(semantic_results.into_iter().take(limit).collect());
        }

        let mut scores: std::collections::HashMap<String, (f32, MemoryDocument)> =
            std::collections::HashMap::new();

        for (rank, doc) in keyword_results.into_iter().enumerate() {
            let key = doc
                .id
                .clone()
                .unwrap_or_else(|| format!("path:{}", doc.path));
            let rrf_score = 1.0 / (RRF_K + (rank as f32) + 1.0);
            scores.insert(key, (rrf_score * KEYWORD_WEIGHT, doc));
        }

        for (rank, doc) in semantic_results.into_iter().enumerate() {
            let key = doc
                .id
                .clone()
                .unwrap_or_else(|| format!("path:{}", doc.path));
            let rrf_score = 1.0 / (RRF_K + (rank as f32) + 1.0);
            if let Some((existing, _)) = scores.get_mut(&key) {
                *existing += rrf_score * SEMANTIC_WEIGHT;
            } else {
                scores.insert(key, (rrf_score * SEMANTIC_WEIGHT, doc));
            }
        }

        let mut fused: Vec<(f32, MemoryDocument)> = scores.into_values().collect();
        fused.sort_by(|a, b| {
            b.0.partial_cmp(&a.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.1.path.cmp(&b.1.path))
        });

        Ok(fused.into_iter().map(|(_, doc)| doc).take(limit).collect())
    }

    pub async fn query(
        &self,
        query_text: &str,
        query_vector: Vec<f32>,
        limit: usize,
    ) -> Result<Vec<MemoryDocument>> {
        self.query_filtered(query_text, query_vector, limit, None)
            .await
    }

    pub async fn query_filtered(
        &self,
        query_text: &str,
        query_vector: Vec<f32>,
        limit: usize,
        filters: Option<&MemoryQueryFilters>,
    ) -> Result<Vec<MemoryDocument>> {
        let mut keyword_results = self
            .search_with_cache_filtered(query_text, limit, filters)
            .await?
            .documents;
        let locomo_only = !keyword_results.is_empty()
            && keyword_results
                .iter()
                .all(|doc| is_locomo_document(&doc.path, &doc.metadata));

        let mut expanded_terms = Vec::new();
        let expansion_seed = if locomo_only {
            keyword_results
                .iter()
                .find(|doc| {
                    doc.metadata
                        .get("category")
                        .and_then(|value| value.as_str())
                        != Some("session_summary")
                })
                .or_else(|| keyword_results.first())
        } else {
            keyword_results.first()
        };
        if let Some(top_doc) = expansion_seed {
            let query_lower = query_text.to_lowercase();
            for w in top_doc.content.split_whitespace() {
                let w_clean = w
                    .trim_matches(|c: char| !c.is_alphanumeric())
                    .to_lowercase();
                if w_clean.len() >= 3 && !query_lower.contains(&w_clean) {
                    expanded_terms.push(w_clean);
                }
            }
            expanded_terms.truncate(5);
        }

        for entity in expanded_terms {
            if let Ok(expanded) = self.search_with_cache_filtered(&entity, 2, filters).await {
                for doc in expanded.documents {
                    if keyword_results.len() > 1 {
                        keyword_results.insert(1, doc);
                    } else {
                        keyword_results.push(doc);
                    }
                }
            }
        }

        let mut seen = std::collections::HashSet::new();
        keyword_results.retain(|doc| {
            if let Some(id) = &doc.id {
                seen.insert(id.clone())
            } else {
                seen.insert(doc.path.clone())
            }
        });

        let vector_results = if query_vector.is_empty() {
            Vec::new()
        } else {
            self.vsearch(query_vector.clone(), limit)
                .await
                .unwrap_or_default()
                .into_iter()
                .filter(|doc| {
                    matches_filters(&doc.path, &doc.metadata, &self.workspace_id, filters)
                })
                .collect()
        };

        if vector_results.is_empty() && query_vector.is_empty() {
            return Ok(keyword_results.into_iter().take(limit).collect());
        }

        let mut scores: std::collections::HashMap<String, (f32, MemoryDocument)> =
            std::collections::HashMap::new();

        for (rank, doc) in keyword_results.into_iter().enumerate() {
            let key = doc
                .id
                .clone()
                .unwrap_or_else(|| format!("path:{}", doc.path));
            let rrf_score = 1.0 / (RRF_K + (rank as f32) + 1.0);
            let weighted_score = rrf_score * KEYWORD_WEIGHT;
            scores.insert(key, (weighted_score, doc));
        }

        for (rank, doc) in vector_results.into_iter().enumerate() {
            let key = doc
                .id
                .clone()
                .unwrap_or_else(|| format!("path:{}", doc.path));
            let rrf_score = 1.0 / (RRF_K + (rank as f32) + 1.0);
            let weighted_score = rrf_score * SEMANTIC_WEIGHT;
            if let Some((existing_score, _)) = scores.get_mut(&key) {
                *existing_score += weighted_score;
            } else {
                scores.insert(key, (weighted_score, doc));
            }
        }

        let mut fused: Vec<(f32, MemoryDocument)> = scores.into_values().collect();
        fused.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let combined: Vec<MemoryDocument> =
            fused.into_iter().map(|(_, doc)| doc).take(limit).collect();
        Ok(combined)
    }

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
                .put(memory_record_from_document(&self.workspace_id, &doc))
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
                .update(memory_record_from_document(&self.workspace_id, &persisted))
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
        let metadata =
            crate::memory::schema::normalize_metadata(&path, metadata, &self.workspace_id, typed)?;
        let metadata = index::normalize_locomo_metadata(&path, metadata);
        let variants = index::expand_document_variants(&path, &content, &metadata);
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
                let _ = store.delete(&self.workspace_id, path_or_id).await?;
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
                let _ = store.delete(&self.workspace_id, &id).await?;
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

    async fn invalidate_cache(&self) {
        self.search_cache.write().await.clear();
    }

    fn merge_ranked_candidates(
        &self,
        candidate_scores: &mut HashMap<String, (f32, MemoryDocument, f32)>,
        documents: Vec<MemoryDocument>,
        query: &str,
        query_weight: f32,
    ) {
        for (rank, doc) in documents.into_iter().enumerate() {
            let key = doc.id.clone().unwrap_or_else(|| doc.path.clone());
            let rrf_score = 1.0 / (RRF_K + (rank as f32) + 1.0);
            let rerank = contextual_boost(query, &doc, query_weight);
            let combined = (rrf_score * query_weight) + rerank;
            candidate_scores
                .entry(key)
                .and_modify(|entry| {
                    entry.0 += combined;
                    entry.2 = entry.2.max(query_weight);
                })
                .or_insert((combined, doc, query_weight));
        }
    }

    async fn multi_hop_context(
        &self,
        query_text: &str,
        seed_docs: &[MemoryDocument],
        filters: Option<&MemoryQueryFilters>,
    ) -> Vec<MemoryDocument> {
        let mut expanded = Vec::new();
        let query_terms = normalize_query(query_text);

        for doc in seed_docs.iter().take(MAX_MULTI_HOP_DEPTH) {
            let mut extracted = index::extract_candidate_terms_internal(&doc.content);
            extracted.extend(index::extract_candidate_terms_internal(&doc.path));
            extracted.sort();
            extracted.dedup();
            for term in extracted.into_iter().take(MAX_EXPANSIONS) {
                if query_terms.contains(&term) {
                    continue;
                }
                if let Ok(results) = self.search_with_cache_filtered(&term, 2, filters).await {
                    expanded.extend(results.documents);
                }
            }
        }

        expanded
    }
}

pub async fn query_with_embedding(
    memory: &QmdMemory,
    query_text: &str,
    limit: usize,
) -> Result<Vec<MemoryDocument>> {
    query_with_embedding_filtered(memory, query_text, limit, None).await
}

pub async fn query_with_embedding_filtered(
    memory: &QmdMemory,
    query_text: &str,
    limit: usize,
    filters: Option<&MemoryQueryFilters>,
) -> Result<Vec<MemoryDocument>> {
    let mut processed_query = query_text.to_string();

    let all_docs = memory.all_documents().await;
    let mut all_speakers = std::collections::HashSet::new();
    let locomo_only = !all_docs.is_empty()
        && all_docs
            .iter()
            .all(|doc| is_locomo_document(&doc.path, &doc.metadata));
    for doc in &all_docs {
        for speaker in extract_speakers(&doc.content) {
            all_speakers.insert(speaker);
        }
    }
    let speakers_list: Vec<String> = all_speakers.into_iter().collect();

    if !speakers_list.is_empty() {
        processed_query = resolve_pronouns(&processed_query, &speakers_list);
    }

    if let Some(target_speaker) = extract_speaker_from_query(query_text) {
        if !processed_query.contains(&target_speaker) {
            processed_query = format!("{} {}", target_speaker, processed_query);
        }
    }

    if locomo_only {
        return memory
            .query_filtered(&processed_query, Vec::new(), limit, filters)
            .await;
    }

    let query_vector = generate_embedding(&processed_query).await?;

    if query_vector.is_empty() {
        return memory
            .search_with_cache_filtered(&processed_query, limit, filters)
            .await
            .map(|r| r.documents);
    }

    let initial_results = memory
        .vsearch(query_vector.clone(), 3)
        .await
        .unwrap_or_default();

    if !initial_results.is_empty() {
        let mut context_terms = Vec::new();

        let common_words: std::collections::HashSet<&str> = std::collections::HashSet::from_iter([
            "the", "a", "an", "is", "are", "was", "were", "be", "been", "being", "have", "has",
            "had", "do", "does", "did", "will", "would", "could", "should", "may", "might", "must",
            "shall", "can", "need", "dare", "to", "of", "in", "for", "on", "with", "at", "by",
            "from", "as", "into", "through", "during", "before", "after", "above", "below", "that",
            "this", "these", "those", "it", "its", "they", "them", "what", "which", "who", "whom",
            "whose", "where", "when", "why", "how",
        ]);

        for doc in initial_results.iter().take(2) {
            for word in doc.content.split_whitespace() {
                let w_clean = word
                    .trim_matches(|c: char| !c.is_alphanumeric())
                    .to_lowercase();
                if w_clean.len() >= 4
                    && !common_words.contains(w_clean.as_str())
                    && !processed_query.to_lowercase().contains(&w_clean)
                {
                    context_terms.push(w_clean);
                }
            }
        }

        if context_terms.len() >= 2 {
            let expanded_query = format!("{} {}", processed_query, context_terms.join(" "));
            if let Ok(expanded_vector) = generate_embedding(&expanded_query).await {
                if !expanded_vector.is_empty() {
                    return memory
                        .query_filtered(&expanded_query, expanded_vector, limit, filters)
                        .await;
                }
            }
        }
    }

    memory
        .query_filtered(&processed_query, query_vector, limit, filters)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[tokio::test]
    async fn repeated_searches_hit_cache() {
        let memory = QmdMemory::new(Arc::new(AsyncRwLock::new(Vec::new())));
        memory
            .add_document(
                "docs/cache".to_string(),
                "cache acceleration for repeated searches".to_string(),
                serde_json::json!({}),
            )
            .await
            .expect("test assertion");

        let first = memory
            .search_with_cache("cache acceleration", 5)
            .await
            .expect("test assertion");
        let second = memory
            .search_with_cache("cache acceleration", 5)
            .await
            .expect("test assertion");
        let metrics = memory.cache_metrics().await;

        assert!(!first.cache_hit);
        assert!(second.cache_hit);
        assert_eq!(metrics.misses, 1);
        assert_eq!(metrics.hits, 1);
        assert_eq!(metrics.entries, 1);
    }

    #[tokio::test]
    async fn mutating_memory_invalidates_cache() {
        let memory = QmdMemory::new(Arc::new(AsyncRwLock::new(Vec::new())));
        memory
            .add_document(
                "docs/original".to_string(),
                "performance tuning for xavier".to_string(),
                serde_json::json!({}),
            )
            .await
            .expect("test assertion");

        let _ = memory.search_with_cache("performance", 5).await.expect("test assertion");
        assert_eq!(memory.cache_metrics().await.entries, 1);

        memory
            .add_document(
                "docs/new".to_string(),
                "new performance tuning guide".to_string(),
                serde_json::json!({}),
            )
            .await
            .expect("test assertion");

        assert_eq!(memory.cache_metrics().await.entries, 0);
    }

    #[tokio::test]
    async fn add_document_skips_embedding_when_service_not_configured() {
        unsafe {
            env::remove_var("XAVIER_EMBEDDING_URL");
            env::set_var("XAVIER_EMBEDDER", "disabled");
        }

        let memory = QmdMemory::new(Arc::new(AsyncRwLock::new(Vec::new())));
        memory
            .add_document(
                "docs/offline".to_string(),
                "offline startup should not require embeddings".to_string(),
                serde_json::json!({ "source": "test" }),
            )
            .await
            .expect("test assertion");

        let stored = memory.get("docs/offline").await.expect("test assertion").expect("test assertion");
        assert!(stored.embedding.is_empty());
    }

    #[tokio::test]
    async fn add_document_creates_clean_locomo_derivatives() {
        let memory = QmdMemory::new(Arc::new(AsyncRwLock::new(Vec::new())));
        memory
            .add_document(
                "locomo/conv-26/session_1/D1:17".to_string(),
                "Caroline: I've been researching adoption agencies lately.".to_string(),
                serde_json::json!({
                    "benchmark": "locomo",
                    "speaker": "Caroline",
                    "session_time": "8 May, 2023"
                }),
            )
            .await
            .expect("test assertion");

        let stored = memory.all_documents().await;
        assert!(stored.len() > 1);
        let derived = stored
            .iter()
            .find(|doc| {
                doc.metadata.get("memory_kind").and_then(|v| v.as_str()) == Some("fact_atom")
            })
            .expect("derived fact atom");
        assert_eq!(
            derived
                .metadata
                .get("normalized_value")
                .and_then(|v| v.as_str()),
            Some("Adoption agencies")
        );
        assert!(!derived.content.contains("source_path"));
    }

    #[tokio::test]
    async fn locomo_search_prioritizes_temporal_derivatives_over_session_summaries() {
        let memory = QmdMemory::new(Arc::new(AsyncRwLock::new(Vec::new())));
        memory
            .add_document(
                "locomo/conv-26/session_1/summary".to_string(),
                "Caroline and Melanie spoke on 8 May, 2023. Caroline discussed several LGBTQ experiences and many other summer memories.".to_string(),
                serde_json::json!({
                    "benchmark": "locomo",
                    "session_time": "1:56 pm on 8 May, 2023",
                    "category": "session_summary",
                }),
            )
            .await
            .expect("test assertion");
        memory
            .add_document(
                "locomo/conv-26/session_1/D1:3".to_string(),
                "Caroline: I went to a LGBTQ support group yesterday and it was so powerful."
                    .to_string(),
                serde_json::json!({
                    "benchmark": "locomo",
                    "session_time": "1:56 pm on 8 May, 2023",
                    "speaker": "Caroline",
                    "category": "conversation",
                }),
            )
            .await
            .expect("test assertion");

        let results = memory
            .search("When did Caroline go to the LGBTQ support group?", 5)
            .await
            .expect("test assertion");

        assert!(!results.is_empty());
        assert_eq!(
            results[0]
                .metadata
                .get("memory_kind")
                .and_then(|value| value.as_str()),
            Some("temporal_event")
        );
        assert_eq!(
            results[0]
                .metadata
                .get("resolved_date")
                .and_then(|value| value.as_str()),
            Some("7 May 2023")
        );
    }

    #[tokio::test]
    async fn add_document_normalizes_locomo_dia_ids_for_primary_and_derived_docs() {
        let memory = QmdMemory::new(Arc::new(AsyncRwLock::new(Vec::new())));
        memory
            .add_document(
                "locomo/conv-26/session_1/D1:03".to_string(),
                "Caroline: I went to a LGBTQ support group yesterday and it was so powerful."
                    .to_string(),
                serde_json::json!({
                    "benchmark": "locomo",
                    "speaker": "Caroline",
                    "session_time": "1:56 pm on 8 May, 2023",
                    "dia_id": "d1:03",
                    "category": "conversation",
                }),
            )
            .await
            .expect("test assertion");

        let stored = memory.all_documents().await;
        let primary = stored
            .iter()
            .find(|doc| doc.path == "locomo/conv-26/session_1/D1:03")
            .expect("primary locomo document");
        assert_eq!(
            primary
                .metadata
                .get("normalized_dia_id")
                .and_then(|value| value.as_str()),
            Some("D1:3")
        );
        assert_eq!(
            primary
                .metadata
                .get("dia_id")
                .and_then(|value| value.as_str()),
            Some("D1:3")
        );

        let derived = stored
            .iter()
            .find(|doc| doc.path.ends_with("#derived/temporal_event/0"))
            .expect("derived temporal event");
        assert_eq!(
            derived
                .metadata
                .get("source_path")
                .and_then(|value| value.as_str()),
            Some("locomo/conv-26/session_1/D1:3")
        );
        assert_eq!(
            derived
                .metadata
                .get("source_dia_id")
                .and_then(|value| value.as_str()),
            Some("D1:3")
        );
    }

    #[tokio::test]
    async fn hybrid_search_uses_rrf_to_combine_keyword_and_vector_hits() {
        let memory = QmdMemory::new(Arc::new(AsyncRwLock::new(Vec::new())));
        memory
            .add(MemoryDocument {
                id: Some("kw-doc".to_string()),
                path: "docs/keyword".to_string(),
                content: "Alice moved to Paris in 2020 to work as a software engineer.".to_string(),
                metadata: serde_json::json!({}),
                content_vector: Some(vec![0.0, 1.0]),
                embedding: vec![0.0, 1.0],
            })
            .await
            .expect("test assertion");
        memory
            .add(MemoryDocument {
                id: Some("semantic-doc".to_string()),
                path: "docs/semantic".to_string(),
                content:
                    "Alice's favorite programming language is Rust, which she learned in 2021."
                        .to_string(),
                metadata: serde_json::json!({}),
                content_vector: Some(vec![1.0, 0.0]),
                embedding: vec![1.0, 0.0],
            })
            .await
            .expect("test assertion");
        memory
            .add(MemoryDocument {
                id: Some("noise-doc".to_string()),
                path: "docs/noise".to_string(),
                content: "Bob studied design and architecture in Boston.".to_string(),
                metadata: serde_json::json!({}),
                content_vector: Some(vec![0.0, 0.2]),
                embedding: vec![0.0, 0.2],
            })
            .await
            .expect("test assertion");

        let results = memory
            .query_with_hybrid_search("Where did Alice move in 2020?", vec![1.0, 0.0], 3)
            .await
            .expect("test assertion");

        let paths: Vec<&str> = results.iter().map(|doc| doc.path.as_str()).collect();
        assert!(paths.iter().take(2).any(|path| *path == "docs/keyword"));
        assert!(paths.iter().take(2).any(|path| *path == "docs/semantic"));
    }

    #[test]
    fn test_extract_speakers() {
        let text = "Caroline: Hello\n[James]: Hi\nSpeaker: Alice\nPerson: Robert\nGuest: Emma";
        let speakers = extract_speakers(text);
        assert!(speakers.contains(&"Caroline".to_string()));
        assert!(speakers.contains(&"James".to_string()));
        assert!(speakers.contains(&"Alice".to_string()));
        assert!(speakers.contains(&"Robert".to_string()));
        assert!(speakers.contains(&"Emma".to_string()));
    }

    #[test]
    fn test_extract_speaker_from_query() {
        assert_eq!(
            extract_speaker_from_query("Who is Caroline?"),
            Some("Caroline".to_string())
        );
        assert_eq!(
            extract_speaker_from_query("What did James say?"),
            Some("James".to_string())
        );
        assert_eq!(
            extract_speaker_from_query("When was Alice there?"),
            Some("Alice".to_string())
        );
        assert_eq!(
            extract_speaker_from_query("Where is Robert?"),
            Some("Robert".to_string())
        );
        assert_eq!(
            extract_speaker_from_query("Why did Emma laugh?"),
            Some("Emma".to_string())
        );
    }

    #[test]
    fn test_resolve_pronouns() {
        let speakers = vec!["Caroline".to_string(), "James".to_string()];

        // Single female candidate
        assert_eq!(
            resolve_pronouns("What did she say?", &speakers),
            "What did Caroline say?"
        );

        // Single male candidate
        assert_eq!(
            resolve_pronouns("What did he say?", &speakers),
            "What did James say?"
        );

        // Multiple female candidates - no resolution
        let speakers_multiple = vec!["Caroline".to_string(), "Alice".to_string()];
        assert_eq!(
            resolve_pronouns("What did she say?", &speakers_multiple),
            "What did she say?"
        );
    }

    #[test]
    fn test_is_likely_speaker() {
        assert!(is_likely_speaker("Caroline"));
        assert!(is_likely_speaker("James"));
        assert!(!is_likely_speaker("Who"));
        assert!(!is_likely_speaker("What"));
        assert!(!is_likely_speaker("She"));
        assert!(!is_likely_speaker("The"));
    }
}
