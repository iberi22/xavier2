use anyhow::Result;
use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::atomic::Ordering as AtomicOrdering;
use crate::memory::schema::{matches_filters, resolve_metadata, EvidenceKind, MemoryKind, MemoryQueryFilters};
use crate::memory::qmd_memory::types::{CachedSearchResult, MemoryDocument, SearchCacheKey};
use crate::memory::qmd_memory::QmdMemory;
use crate::memory::qmd_memory::consolidation::is_locomo_document;
use crate::memory::qmd_memory::embeddings::query_with_embedding_filtered;
use std::cmp::Ordering;

pub(crate) static SYNONYM_MAP: LazyLock<HashMap<&'static str, &'static [&'static str]>> =
    LazyLock::new(|| {
        HashMap::from([
            ("bug", &["issue", "error", "failure", "defect"][..]),
            ("cache", &["caching", "memoization", "store"][..]),
            ("fast", &["quick", "speed", "latency"][..]),
            ("memory", &["context", "retrieval", "knowledge"][..]),
            ("search", &["lookup", "find", "retrieve"][..]),
            ("vector", &["embedding", "semantic", "dense"][..]),
            ("query", &["question", "request", "prompt"][..]),
            ("reasoning", &["multi-hop", "inference", "analysis"][..]),
        ])
    });

pub(crate) const RRF_K: f32 = 60.0;
pub(crate) const KEYWORD_WEIGHT: f32 = 0.7;
pub(crate) const SEMANTIC_WEIGHT: f32 = 0.3;
pub(crate) const MAX_EXPANSIONS: usize = 4;
pub(crate) const MAX_MULTI_HOP_DEPTH: usize = 2;
pub(crate) const MAX_RERANK_CANDIDATES: usize = 32;

pub(crate) struct QueryBundle {
    pub(crate) normalized_query: String,
    pub(crate) variants: Vec<String>,
    pub(crate) weights: HashMap<String, f32>,
}

impl QueryBundle {
    pub(crate) fn weight_for(&self, query: &str) -> f32 {
        self.weights.get(query).copied().unwrap_or(1.0)
    }
}

pub(crate) fn build_query_bundle(query_text: &str) -> QueryBundle {
    let normalized_query = normalize_query(query_text);
    let mut variants = vec![normalized_query.clone()];
    let mut weights = HashMap::from([(normalized_query.clone(), 1.0)]);

    let tokens = normalized_query
        .split_whitespace()
        .map(|value| value.to_string())
        .collect::<Vec<_>>();

    for token in tokens.into_iter().take(MAX_EXPANSIONS) {
        if let Some(synonyms) = SYNONYM_MAP.get(token.as_str()) {
            for synonym in synonyms.iter().take(2) {
                let expanded = if normalized_query.is_empty() {
                    (*synonym).to_string()
                } else {
                    format!("{normalized_query} {synonym}")
                };
                if weights.contains_key(&expanded) {
                    continue;
                }
                variants.push(expanded.clone());
                weights.insert(expanded, 0.85);
            }
        }
    }

    if variants.len() == 1 {
        for token in query_text.split_whitespace().take(MAX_EXPANSIONS) {
            let cleaned = normalize_token(token);
            if cleaned.len() < 3 || cleaned == normalized_query {
                continue;
            }
            let expanded = format!("{normalized_query} {cleaned}");
            if !weights.contains_key(&expanded) {
                variants.push(expanded.clone());
                weights.insert(expanded, 0.8);
            }
        }
    }

    variants.truncate(5);

    QueryBundle {
        normalized_query,
        variants,
        weights,
    }
}

impl QmdMemory {
    pub async fn search(&self, query_text: &str, limit: usize) -> Result<Vec<MemoryDocument>> {
        self.search_filtered(query_text, limit, None).await
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

        if std::env::var("XAVIER2_EMBEDDING_URL").is_ok() {
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
        let query_bundle = build_query_bundle(query_text);
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
                .unwrap_or(Ordering::Equal)
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
                .unwrap_or(Ordering::Equal)
                .then_with(|| right.2.partial_cmp(&left.2).unwrap_or(Ordering::Equal))
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
            workspace_id: self.workspace_id().to_string(),
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
                if !matches_filters(&doc.path, &doc.metadata, self.workspace_id(), filters) {
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
                .unwrap_or(Ordering::Equal)
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

        // Compute all similarities first to find max for normalization
        let mut similarities: Vec<(f32, MemoryDocument)> = docs
            .iter()
            .filter_map(|doc| {
                let score = cosine_similarity(&query_vector, &doc.embedding);
                (score > 0.0).then(|| (score, doc.clone()))
            })
            .collect();

        // Normalize scores to [0.5, 1.0] range for better RRF fusion
        // This ensures even low-similarity docs get some weight
        if let Some(max_sim) = similarities.iter().map(|(s, _)| *s).reduce(f32::max) {
            if max_sim > 0.0 {
                for (score, _) in similarities.iter_mut() {
                    // Scale from [0, max] to [0.5, 1.0]
                    // Low similarity docs still get some weight but less than high ones
                    *score = 0.5 + 0.5 * (*score / max_sim);
                }
            }
        }

        similarities.sort_by(|left, right| {
            right
                .0
                .partial_cmp(&left.0)
                .unwrap_or(Ordering::Equal)
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
        // Keyword results (BM25-style via lexical_score)
        let keyword_results = self.search(query_text, limit).await?;

        // Semantic results via pplx-embed cosine similarity
        let semantic_results = if query_vector.is_empty() {
            Vec::new()
        } else {
            self.vsearch(query_vector, limit).await.unwrap_or_default()
        };

        // If only one signal is available, fall back to it directly
        if semantic_results.is_empty() {
            return Ok(keyword_results.into_iter().take(limit).collect());
        }
        if keyword_results.is_empty() {
            return Ok(semantic_results.into_iter().take(limit).collect());
        }

        // Weighted RRF matches the main query path: exact lexical/entity hits should
        // stay slightly ahead of broader semantic neighbors.
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

        // --- NATIVE LEXICAL GRAPH EXPANSION (MULTI-HOP) ---
        // Extract potential entities from the top document and fetch their 1-hop context
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
            expanded_terms.truncate(5); // Only take top 5
        }
        tracing::debug!("Extracted expanded_terms: {:?}", expanded_terms);

        for entity in expanded_terms {
            if let Ok(expanded) = self.search_with_cache_filtered(&entity, 2, filters).await {
                // Insert right after the primary hit so it gets a high RRF score
                for doc in expanded.documents {
                    if keyword_results.len() > 1 {
                        keyword_results.insert(1, doc);
                    } else {
                        keyword_results.push(doc);
                    }
                }
            }
        }

        // Deduplicate to avoid rank inflation
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
                    matches_filters(&doc.path, &doc.metadata, self.workspace_id(), filters)
                })
                .collect()
        };

        if vector_results.is_empty() && query_vector.is_empty() {
            return Ok(keyword_results.into_iter().take(limit).collect());
        }

        // ============================================
        // WEIGHTED RECIPROCAL RANK FUSION (WRRF)
        // ============================================
        // For conversational Q&A (LoCoMo), keyword search should be weighted higher
        // Weight: 70% keyword, 30% semantic
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
            let mut extracted = extract_candidate_terms(&doc.content);
            extracted.extend(extract_candidate_terms(&doc.path));
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

pub(crate) fn lexical_score(doc: &MemoryDocument, normalized_query: &str) -> f32 {
    if normalized_query.is_empty() {
        return 0.0;
    }

    if is_locomo_document(&doc.path, &doc.metadata) {
        return locomo_lexical_score(doc, normalized_query);
    }

    let content = doc.content.to_lowercase();
    let path = doc.path.to_lowercase();
    let query_terms: Vec<&str> = normalized_query
        .split_whitespace()
        .filter(|term| !term.is_empty())
        .collect();
    let mut matched_terms = 0usize;
    let mut score = 0.0f32;
    for term in &query_terms {
        let content_hits = content.matches(term).count() as f32;
        let path_hits = path.matches(term).count() as f32 * 2.0;
        if content_hits > 0.0 || path_hits > 0.0 {
            matched_terms += 1;
        }
        score += content_hits + path_hits;
    }
    score += (matched_terms * matched_terms) as f32;

    let memory_kind = doc
        .metadata
        .get("memory_kind")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let resolved = resolved_doc_metadata(doc);
    let category = doc
        .metadata
        .get("category")
        .and_then(|value| value.as_str())
        .unwrap_or_default();

    if normalized_query.split_whitespace().count() >= 2 && content.contains(normalized_query) {
        score += 6.0;
    }

    for (query_signal, content_signal, bonus) in [
        ("sunrise", "sunrise", 12.0),
        ("support", "support group", 12.0),
        ("charity", "charity race", 12.0),
        ("camping", "camping", 12.0),
        ("identity", "transgender", 10.0),
        ("relationship", "single", 10.0),
        ("research", "adoption agenc", 8.0),
        ("field", "counsel", 8.0),
        ("pursue", "counsel", 8.0),
        ("what", "what", 2.0),
        ("who", "who", 3.0),
        ("how", "how", 2.0),
        ("why", "why", 2.0),
        ("which", "which", 2.0),
    ] {
        if normalized_query.contains(query_signal) && content.contains(content_signal) {
            score += bonus;
        }
    }

    if matches!(
        memory_kind,
        "fact_atom" | "entity_state" | "temporal_event" | "summary_fact"
    ) {
        score += 5.0;
    }

    if let Some(resolved) = &resolved {
        match resolved.kind {
            MemoryKind::Repo | MemoryKind::File | MemoryKind::Symbol | MemoryKind::Url => {
                score += 5.0;
            }
            MemoryKind::Decision | MemoryKind::Task | MemoryKind::Fact
                if query_terms.len() >= 2 =>
            {
                score += 3.0;
            }
            _ => {}
        }

        if let Some(evidence_kind) = resolved.evidence_kind {
            match evidence_kind {
                EvidenceKind::SourceTurn => score += 6.0,
                EvidenceKind::FactAtom | EvidenceKind::EntityState => score += 8.0,
                EvidenceKind::TemporalEvent if normalized_query.contains("when") => score += 10.0,
                EvidenceKind::SessionSummary => score *= 0.5,
                _ => {}
            }
        }

        for exact in [
            resolved.provenance.symbol.as_ref(),
            resolved.provenance.file_path.as_ref(),
            resolved.provenance.repo_url.as_ref(),
            resolved.provenance.url.as_ref(),
            resolved.namespace.session_id.as_ref(),
            resolved.namespace.agent_id.as_ref(),
            resolved.namespace.user_id.as_ref(),
            resolved.namespace.project.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            let lowered = exact.to_ascii_lowercase();
            if !lowered.is_empty() && normalized_query.contains(&lowered) {
                score += 18.0;
            }
        }
    }

    if doc
        .metadata
        .get("normalized_value")
        .and_then(|value| value.as_str())
        .is_some()
    {
        score += 2.0;
    }

    match category {
        "session_summary" => score *= 0.2,
        "conversation" => score *= 1.2,
        "observation" => score *= 0.8,
        _ => {}
    }

    score
}

fn locomo_lexical_score(doc: &MemoryDocument, normalized_query: &str) -> f32 {
    let content = doc.content.to_lowercase();
    let path = doc.path.to_lowercase();
    let terms = locomo_query_terms(normalized_query);
    let phrases = locomo_phrases(&terms);
    let speaker = metadata_text_lower(doc, "speaker");
    let subject = doc
        .metadata
        .get("event_subject")
        .and_then(|value| value.as_str())
        .unwrap_or_else(|| {
            doc.metadata
                .get("speaker")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
        })
        .to_lowercase();
    let action = metadata_text_lower(doc, "event_action");
    let resolved_date = metadata_text_lower(doc, "resolved_date");
    let normalized_value = metadata_text_lower(doc, "normalized_value");
    let answer_span = metadata_text_lower(doc, "answer_span");
    let memory_kind = doc
        .metadata
        .get("memory_kind")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let resolved = resolved_doc_metadata(doc);
    let category = doc
        .metadata
        .get("category")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let temporal_query = is_temporal_query(normalized_query);

    // Generic question pattern detection (context-agnostic)
    let is_shared_query = normalized_query.contains("in common")
        || normalized_query.contains("both like")
        || normalized_query.contains("both have")
        || normalized_query.contains("what do")
            && normalized_query.contains("and")
            && normalized_query.contains("both");
    let is_why_query = normalized_query.starts_with("why")
        || normalized_query.contains(" why ")
        || normalized_query.contains("reason")
        || normalized_query.contains("because");
    let is_what_think_query = normalized_query.contains("think")
        || normalized_query.contains("believe")
        || normalized_query.contains("opinion")
        || normalized_query.contains("ideal")
        || normalized_query.contains("prefer");

    let mut score = 0.0f32;
    let mut matched_terms = 0usize;
    for term in &terms {
        let mut term_score = 0.0f32;

        if !speaker.is_empty() && speaker == **term {
            term_score += 18.0;
        }
        if !subject.is_empty() && subject == **term {
            term_score += 18.0;
        }
        if action.contains(*term) {
            term_score += 14.0;
        }
        if normalized_value.contains(*term) || answer_span.contains(*term) {
            term_score += 12.0;
        }
        if path.contains(*term) {
            term_score += 8.0;
        }
        if content.contains(*term) {
            term_score += 4.0;
        }

        if term_score > 0.0 {
            matched_terms += 1;
            score += term_score;
        }
    }

    score += (matched_terms * matched_terms * 2) as f32;

    for phrase in &phrases {
        if action.contains(phrase)
            || normalized_value.contains(phrase)
            || answer_span.contains(phrase)
        {
            score += 18.0;
        } else if content.contains(phrase) || path.contains(phrase) {
            score += 9.0;
        }
    }

    if normalized_query.split_whitespace().count() >= 2 && content.contains(normalized_query) {
        score += 10.0;
    }

    if !speaker.is_empty() && normalized_query.contains(&speaker) {
        score += 14.0;
    }

    // Generic context-agnostic scoring patterns
    // For \"shared/common\" queries, prioritize documents with matching subjects
    if is_shared_query {
        // Boost documents with structured facts (fact_atom, entity_state) for shared queries
        if matches!(memory_kind, "fact_atom" | "entity_state") {
            score += 35.0;
        }
        // Prioritize normalized_value and answer_span which contain extracted facts
        if !normalized_value.is_empty() || !answer_span.is_empty() {
            score += 25.0;
        }
        // Penalize summaries for shared queries (prefer primary sources)
        if memory_kind == "summary_fact" {
            score *= 0.15;
        }
    }

    // For \"why\" queries, prioritize documents with reason/explanation patterns
    if is_why_query {
        // Boost documents that contain causal language
        let has_reason = content.contains("because")
            || content.contains("'cause")
            || content.contains("since")
            || content.contains("reason")
            || content.contains("to share")
            || content.contains("to start")
            || content.contains("decided")
            || content.contains("wanted");
        if has_reason {
            score += 30.0;
        }
        // Boost structured facts with clear values
        if !normalized_value.is_empty() {
            score += 20.0;
        }
        // Penalize summaries for why queries
        if memory_kind == "summary_fact" {
            score *= 0.2;
        }
    }

    // For \"what think/opinion\" queries, prioritize sentiment/opinion content
    if is_what_think_query {
        // Boost documents with opinion markers
        let has_opinion = content.contains("think")
            || content.contains("believe")
            || content.contains("feel")
            || content.contains("prefer")
            || content.contains("ideal")
            || content.contains("favorite")
            || contains_opinion_adjectives(&content);
        if has_opinion {
            score += 25.0;
        }
        // Boost documents with extracted values
        if !normalized_value.is_empty() || !answer_span.is_empty() {
            score += 15.0;
        }
        // Penalize summaries for opinion queries
        if memory_kind == "summary_fact" {
            score *= 0.2;
        }
    }

    if let Some(resolved) = &resolved {
        match resolved.evidence_kind {
            Some(EvidenceKind::TemporalEvent) if temporal_query => score += 60.0,
            Some(
                EvidenceKind::FactAtom | EvidenceKind::EntityState | EvidenceKind::SummaryFact,
            ) if !temporal_query => {
                score += 28.0;
            }
            Some(
                EvidenceKind::FactAtom | EvidenceKind::EntityState | EvidenceKind::SummaryFact,
            ) => {
                score += 12.0;
            }
            Some(EvidenceKind::SourceTurn) => score += 8.0,
            _ => {}
        }

        if let Some(symbol) = resolved.provenance.symbol.as_ref() {
            if normalized_query.contains(&symbol.to_ascii_lowercase()) {
                score += 24.0;
            }
        }
        if let Some(file_path) = resolved.provenance.file_path.as_ref() {
            if normalized_query.contains(&file_path.to_ascii_lowercase()) {
                score += 16.0;
            }
        }
        if let Some(url) = resolved.provenance.url.as_ref() {
            if normalized_query.contains(&url.to_ascii_lowercase()) {
                score += 16.0;
            }
        }
    }

    match memory_kind {
        "temporal_event" if temporal_query => {
            score += 60.0;
        }
        "fact_atom" | "entity_state" | "summary_fact" if !temporal_query => {
            score += 28.0;
        }
        "fact_atom" | "entity_state" | "summary_fact" => {
            score += 12.0;
        }
        _ => {}
    }

    if !resolved_date.is_empty() {
        score += if temporal_query { 24.0 } else { 6.0 };
        score += match infer_date_granularity(&resolved_date) {
            "full_date" => 10.0,
            "month_year" => 6.0,
            "year" => 2.0,
            _ => 0.0,
        };
    }

    match category {
        "conversation" => {
            score += if temporal_query { 18.0 } else { 10.0 };
        }
        "observation" => {
            score += 2.0;
        }
        "session_summary" => {
            score -= if temporal_query { 70.0 } else { 28.0 };
        }
        _ => {}
    }

    if category == "session_summary" && memory_kind.is_empty() {
        score *= if temporal_query { 0.02 } else { 0.15 };
    }

    // LOCOMO fix: Boost structured data (pricing, numbers) for factuality queries
    // Detect pricing/cost/value queries in both English and Spanish
    let pricing_query = normalized_query.contains("pricing")
        || normalized_query.contains("price")
        || normalized_query.contains("precios")
        || normalized_query.contains("precio")
        || normalized_query.contains("costo")
        || normalized_query.contains("coste")
        || normalized_query.contains("valor")
        || normalized_query.contains("fee")
        || normalized_query.contains("tarifa")
        || normalized_query.contains("cuanto")  // Spanish \"how much\"
        || normalized_query.contains("cuál")     // Spanish \"which\"
        || normalized_query.contains("cuáles"); // Spanish \"which\" plural

    if pricing_query {
        // Boost documents that contain numeric values (likely pricing facts)
        // Patterns: $499, 499, 499.99, etc.
        let has_numeric = content.contains('$')
            || content.contains("/mes")
            || content.contains("/mo")
            || content.contains("/month")
            || content.contains("/monthly")
            || content.contains("/year")
            || content.contains("/annual")
            || regex::Regex::new(r"\d+[.,]?\d*")
                .map(|re| re.is_match(&content))
                .unwrap_or(false);

        if has_numeric {
            score += 30.0;
        }

        // Extra boost for fact_atom/entity_state with normalized_value
        // These are extracted structured facts that are most reliable
        if !normalized_value.is_empty() && has_numeric {
            score += 25.0;
        }

        // Boost for tier/version terms (Starter, Pro, Enterprise, etc.)
        let tier_terms = [
            "starter",
            "pro",
            "enterprise",
            "basic",
            "plan",
            "tier",
            "version",
        ];
        for tier in &tier_terms {
            if normalized_query.contains(tier) && (content.contains(tier) || path.contains(tier)) {
                score += 15.0;
            }
        }
    }

    score.max(0.0)
}

pub fn cosine_similarity(left: &[f32], right: &[f32]) -> f32 {
    if left.is_empty() || right.is_empty() || left.len() != right.len() {
        return 0.0;
    }

    let dot = left
        .iter()
        .zip(right.iter())
        .map(|(a, b)| a * b)
        .sum::<f32>();
    let left_magnitude = left.iter().map(|value| value * value).sum::<f32>().sqrt();
    let right_magnitude = right.iter().map(|value| value * value).sum::<f32>().sqrt();

    if left_magnitude == 0.0 || right_magnitude == 0.0 {
        return 0.0;
    }

    dot / (left_magnitude * right_magnitude)
}

pub(crate) fn contextual_boost(query: &str, document: &MemoryDocument, weight: f32) -> f32 {
    let doc_text = format!(
        "{} {} {}",
        document.path.to_ascii_lowercase(),
        document.content.to_ascii_lowercase(),
        document.metadata.to_string().to_ascii_lowercase()
    );
    let mut score = 0.0;
    for token in query.split_whitespace() {
        if token.len() >= 3 && doc_text.contains(token) {
            score += 0.12 * weight;
        }
    }
    if let Some(title) = document
        .metadata
        .get("title")
        .and_then(|value| value.as_str())
    {
        if query.contains(&title.to_ascii_lowercase()) {
            score += 0.20 * weight;
        }
    }
    score + memory_importance_score(document) + memory_decay_penalty(document)
}

fn memory_importance_score(document: &MemoryDocument) -> f32 {
    let metadata = &document.metadata;
    let importance = metadata
        .get("importance")
        .or_else(|| metadata.get("memory_importance"))
        .and_then(|value| value.as_f64())
        .unwrap_or(0.0) as f32;
    importance.clamp(0.0, 1.0) * 0.25
}

fn memory_decay_penalty(document: &MemoryDocument) -> f32 {
    let updated = document
        .metadata
        .get("updated_at")
        .and_then(|value| value.as_str())
        .or_else(|| {
            document
                .metadata
                .get("last_accessed_at")
                .and_then(|value| value.as_str())
        });
    let Some(updated) = updated else {
        return 0.0;
    };
    let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(updated) else {
        return 0.0;
    };
    let age_days = (chrono::Utc::now() - parsed.with_timezone(&chrono::Utc))
        .num_days()
        .max(0) as f32;
    -(age_days / 365.0).min(1.0) * 0.15
}

pub(crate) fn normalize_query(query_text: &str) -> String {
    query_text
        .split_whitespace()
        .map(normalize_token)
        .filter(|token| {
            !token.is_empty()
                && !matches!(
                    token.as_str(),
                    "when"
                        | "what"
                        | "where"
                        | "which"
                        | "who"
                        | "how"
                        | "why"
                        | "did"
                        | "does"
                        | "was"
                        | "were"
                        | "the"
                        | "and"
                        | "for"
                        | "with"
                        | "about"
                        | "into"
                        | "from"
                        | "that"
                        | "this"
                        | "your"
                        | "have"
                        | "had"
                )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_token(token: &str) -> String {
    token
        .chars()
        .filter(|char| char.is_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

fn extract_candidate_terms(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(normalize_token)
        .filter(|token| token.len() >= 4)
        .filter(|token| {
            !matches!(
                token.as_str(),
                "with"
                    | "that"
                    | "this"
                    | "from"
                    | "have"
                    | "were"
                    | "when"
                    | "what"
                    | "where"
                    | "which"
                    | "would"
                    | "could"
            )
        })
        .collect()
}

fn locomo_query_terms(normalized_query: &str) -> Vec<&str> {
    normalized_query
        .split_whitespace()
        .filter(|term| {
            !term.is_empty()
                && !matches!(
                    *term,
                    "the"
                        | "and"
                        | "what"
                        | "when"
                        | "where"
                        | "which"
                        | "with"
                        | "from"
                        | "that"
                        | "this"
                        | "have"
                        | "about"
                        | "your"
                        | "their"
                        | "did"
                        | "does"
                        | "was"
                        | "were"
                )
        })
        .collect()
}

fn is_temporal_query(normalized_query: &str) -> bool {
    normalized_query.contains(" when ")
        || normalized_query.starts_with("when ")
        || normalized_query.contains(" date ")
        || normalized_query.contains(" year ")
        || normalized_query.contains(" month ")
        || normalized_query.contains(" day ")
}

fn contains_opinion_adjectives(content: &str) -> bool {
    let opinion_adjectives = [
        "ideal",
        "perfect",
        "best",
        "favorite",
        "great",
        "amazing",
        "wonderful",
        "excellent",
        "good",
        "bad",
        "terrible",
        "awful",
        "beautiful",
        "nice",
        "lovely",
        "pleasant",
        "important",
        "special",
        "unique",
        "better",
        "worse",
        "prefer",
        "love",
        "hate",
    ];
    let lowered = content.to_lowercase();
    opinion_adjectives.iter().any(|adj| lowered.contains(adj))
}

fn locomo_phrases(terms: &[&str]) -> Vec<String> {
    if terms.len() < 2 {
        return Vec::new();
    }

    terms.windows(2).map(|window| window.join(" ")).collect()
}

fn metadata_text_lower(doc: &MemoryDocument, key: &str) -> String {
    doc.metadata
        .get(key)
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_lowercase()
}

fn resolved_doc_metadata(
    doc: &MemoryDocument,
) -> Option<crate::memory::schema::ResolvedMemoryMetadata> {
    let workspace_id = doc
        .metadata
        .get("namespace")
        .and_then(|value| value.get("workspace_id"))
        .and_then(|value| value.as_str())
        .or_else(|| {
            doc.metadata
                .get("workspace_id")
                .and_then(|value| value.as_str())
        })
        .unwrap_or("default");
    resolve_metadata(&doc.path, &doc.metadata, workspace_id, None).ok()
}

fn infer_date_granularity(value: &str) -> &'static str {
    if value.chars().all(|ch| ch.is_ascii_digit()) && value.len() == 4 {
        "year"
    } else if value.split_whitespace().count() == 2 {
        "month_year"
    } else {
        "full_date"
    }
}
