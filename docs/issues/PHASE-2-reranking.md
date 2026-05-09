# PHASE 2: Reranking with Reciprocal Rank Fusion (RRF)

## Context

After Phase 1, we have both keyword search (FTS5) and vector search (sqlite-vec). But each search type operates independently, and we need a way to combine them intelligently.

**Why this matters:** A user query like "What does BELA know about ManteniApp?" should return results where:
1. Exact matches rank highest
2. Semantic matches ("sales", "product", "software") are also included
3. Results that appear in BOTH search types rank even higher

## Problem Statement

Current search is siloed:
- Keyword search returns results based on word frequency
- Vector search returns results based on embedding similarity
- No fusion of the two ranking systems
- No way to prefer results that appear in both

We need **hybrid search with Reciprocal Rank Fusion (RRF)** to combine both approaches.

## Technical Approach

### 1. RRF Algorithm

**File:** `src/search/rrf.rs` (NEW)

```rust
use std::collections::HashMap;

/// Reciprocal Rank Fusion
///
/// Combines multiple ranked result sets into a single ranked list.
/// Formula: score(d) = Σ 1/(k + rank(d))
///
/// where:
/// - k = 60 (default, tunable constant)
/// - rank(d) = position of document d in result set
///
/// Reference: "Reciprocal Rank Fusion for Retrieving" (Cormack et al., 2009)
pub fn reciprocal_rank_fusion(
    result_sets: Vec<Vec<ScoredResult>>,
    k: u32,
) -> Vec<ScoredResult> {
    let mut scores: HashMap<String, FusedScore> = HashMap::new();

    for result_set in result_sets {
        for (rank, result) in result_set.into_iter().enumerate() {
            let rrf_score = 1.0 / (k + rank as u32);

            scores.entry(result.id.clone())
                .or_insert_with(|| FusedScore {
                    id: result.id,
                    content: result.content,
                    scores: HashMap::new(),
                    total_rrf: 0.0,
                })
                .add_score(result.source.clone(), result.score, rrf_score);
        }
    }

    // Sort by total RRF score descending
    let mut ranked: Vec<_> = scores.into_values().collect();
    ranked.sort_by(|a, b| b.total_rrf.partial_cmp(&a.total_rrf).unwrap());

    ranked.into_iter().map(|s| s.into_result()).collect()
}

#[derive(Clone)]
pub struct ScoredResult {
    pub id: String,
    pub content: String,
    pub score: f32,
    pub source: String,  // "keyword" | "vector" | "hybrid"
}

struct FusedScore {
    id: String,
    content: String,
    scores: HashMap<String, (f32, f32)>,  // source -> (original_score, rrf_score)
    total_rrf: f32,
}

impl FusedScore {
    fn add_score(&mut self, source: String, original_score: f32, rrf_score: f32) {
        self.scores.insert(source, (original_score, rrf_score));
        self.total_rrf += rrf_score;
    }

    fn into_result(self) -> ScoredResult {
        // Use content from highest-scoring source
        let (content, _) = self.scores.values()
            .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
            .cloned().unwrap_or_default();

        ScoredResult {
            id: self.id,
            content,
            score: self.total_rrf,
            source: "hybrid".to_string(),
        }
    }
}
```

### 2. Hybrid Search Module

**File:** `src/search/hybrid.rs` (NEW)

```rust
use crate::state::AppState;
use crate::search::rrf::{self, ScoredResult};

pub struct HybridSearcher {
    keyword_weight: f32,  // 0.0 - 1.0, default 0.5
    vector_weight: f32,  // 0.0 - 1.0, default 0.5
    rrf_k: u32,         // RRF k parameter, default 60
}

impl Default for HybridSearcher {
    fn default() -> Self {
        Self {
            keyword_weight: 0.5,
            vector_weight: 0.5,
            rrf_k: 60,
        }
    }
}

impl HybridSearcher {
    pub async fn search(
        &self,
        state: &AppState,
        query: &str,
        limit: usize,
    ) -> Result<Vec<ScoredResult>, SearchError> {
        // 1. Keyword search (FTS5)
        let keyword_results = self.keyword_search(state, query, limit * 2).await?;

        // 2. Vector search (sqlite-vec)
        let vector_results = self.vector_search(state, query, limit * 2).await?;

        // 3. RRF fusion
        let fused = rrf::reciprocal_rank_fusion(
            vec![keyword_results, vector_results],
            self.rrf_k,
        );

        // 4. Return top results
        Ok(fused.into_iter().take(limit).collect())
    }

    async fn keyword_search(
        &self,
        state: &AppState,
        query: &str,
        limit: usize,
    ) -> Result<Vec<ScoredResult>, SearchError> {
        let memories = state.db.fts_search(query, limit).await
            .map_err(|e| SearchError::DatabaseError(e.to_string()))?;

        Ok(memories.into_iter().enumerate()
            .map(|(i, m)| ScoredResult {
                id: m.id,
                content: m.content,
                score: 1.0 - (i as f32 / limit as f32),  // Normalize 0-1
                source: "keyword".to_string(),
            })
            .collect())
    }

    async fn vector_search(
        &self,
        state: &AppState,
        query: &str,
        limit: usize,
    ) -> Result<Vec<ScoredResult>, SearchError> {
        // Generate query embedding
        let query_vector = state.embedder.encode(query).await
            .map_err(|e| SearchError::EmbeddingError(e.to_string()))?;

        // Search sqlite-vec
        let results = state.vec_store.search(&query_vector, limit).await
            .map_err(|e| SearchError::VectorSearchError(e.to_string()))?;

        Ok(results.into_iter().enumerate()
            .map(|(i, r)| ScoredResult {
                id: r.id,
                content: r.content,
                score: 1.0 - (i as f32 / limit as f32),
                source: "vector".to_string(),
            })
            .collect())
    }
}

#[derive(thiserror::Error, Debug)]
pub enum SearchError {
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Embedding error: {0}")]
    EmbeddingError(String),

    #[error("Vector search error: {0}")]
    VectorSearchError(String),
}
```

### 3. Modify Search API

**File:** `src/api/search.rs` (MODIFY)

Add new endpoint `POST /memory/hybrid`:

```rust
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct HybridSearchRequest {
    pub query: String,
    pub limit: Option<usize>,
    pub rrf_k: Option<u32>,  // Optional RRF k parameter
    pub filters: Option<SearchFilters>,
    pub include_embedding: Option<bool>,
}

#[derive(Deserialize)]
pub struct SearchFilters {
    pub path: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Serialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub query_vector: Option<Vec<f32>>,  // Only if include_embedding=true
    pub total_available: usize,
    pub search_type: String,  // "keyword" | "vector" | "hybrid"
}

#[derive(Serialize)]
pub struct SearchResult {
    pub id: String,
    pub content: String,
    pub score: f32,
    pub source: String,
    pub path: String,
    pub metadata: Option<serde_json::Value>,
}

pub async fn hybrid_search(
    State(state): State<AppState>,
    Json(request): Json<HybridSearchRequest>,
) -> Result<Json<SearchResponse>, StatusCode> {
    let limit = request.limit.unwrap_or(10);
    let k = request.rrf_k.unwrap_or(60);

    let searcher = HybridSearcher {
        keyword_weight: 0.5,
        vector_weight: 0.5,
        rrf_k: k,
    };

    let results = searcher.search(&state, &request.query, limit).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get total count for pagination info
    let total = state.db.get_memory_count().await
        .unwrap_or(0);

    let search_type = if results.first().map(|r| r.source == "hybrid").unwrap_or(false) {
        "hybrid"
    } else {
        "hybrid"  // Always hybrid since we're fusing
    };

    let response = SearchResponse {
        results: results.into_iter().map(|r| SearchResult {
            id: r.id,
            content: r.content,
            score: r.score,
            source: r.source,
            path: String::new(),  // TODO: fetch from DB
            metadata: None,
        }).collect(),
        query_vector: None,  // TODO: return if include_embedding
        total_available: total,
        search_type: search_type.to_string(),
    };

    Ok(Json(response))
}
```

### 4. Update Existing Search Endpoint

**File:** `src/api/search.rs` (MODIFY)

Modify existing `POST /memory/search` to support hybrid mode:

```rust
#[derive(Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub limit: Option<usize>,
    pub search_type: Option<String>,  // "keyword" | "vector" | "hybrid"
    pub rrf_k: Option<u32>,
}

// Modify the search handler
pub async fn search(
    State(state): State<AppState>,
    Json(request): Json<SearchRequest>,
) -> Result<Json<SearchResponse>, StatusCode> {
    let search_type = request.search_type.as_deref().unwrap_or("hybrid");

    match search_type {
        "keyword" => keyword_search_handler(state, request).await,
        "vector" => vector_search_handler(state, request).await,
        "hybrid" | _ => hybrid_search(State(state), Json(HybridSearchRequest {
            query: request.query,
            limit: request.limit,
            rrf_k: request.rrf_k,
            filters: None,
            include_embedding: Some(false),
        })).await,
    }
}
```

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `src/search/mod.rs` | CREATE | Search module barrel |
| `src/search/rrf.rs` | CREATE | RRF algorithm |
| `src/search/hybrid.rs` | CREATE | Hybrid search implementation |
| `src/api/search.rs` | MODIFY | Add hybrid endpoint + modify search |
| `Cargo.toml` | MODIFY | Add thiserror if not present |

## RRF Formula Explained

```
score(d) = Σ 1/(k + rank(d))

For document appearing at rank 1 in keyword AND rank 2 in vector:
keyword contribution: 1/(60+1) = 0.0164
vector contribution: 1/(60+2) = 0.0161
total RRF = 0.0325

vs document appearing rank 1 ONLY in keyword:
total RRF = 1/(60+1) = 0.0164
```

**Key insight:** Documents appearing in multiple result sets get a multiplicative boost, making RRF excellent for hybrid search.

## Acceptance Criteria

1. **RRF fusion:** Results from keyword + vector are fused using RRF formula
2. **Configurable k:** `rrf_k` parameter allows tuning (60 default)
3. **New endpoint:** `POST /memory/hybrid` explicitly calls hybrid search
4. **Backward compatible:** `POST /memory/search` defaults to hybrid mode
5. **Fallback:** If vector search fails, fallback to keyword only
6. **Tests:** `cargo test --lib test_rrf*` passes

## Verification Commands

```bash
# Test RRF algorithm
cargo test --lib test_rrf

# Test hybrid search
curl -X POST http://localhost:8003/memory/hybrid \
  -H "Content-Type: application/json" \
  -d '{"query":"BELA SWAL ManteniApp","limit":5}'

# Compare with keyword-only
curl -X POST http://localhost:8003/memory/search \
  -H "Content-Type: application/json" \
  -d '{"query":"BELA SWAL ManteniApp","search_type":"keyword"}'

# Compare with vector-only
curl -X POST http://localhost:8003/memory/search \
  -H "Content-Type: application/json" \
  -d '{"query":"BELA SWAL ManteniApp","search_type":"vector"}'
```

## Test Cases for RRF

```rust
#[test]
fn test_rrf_fusion_two_result_sets() {
    let results = vec![
        // Result set 1: keyword search
        vec![
            ScoredResult { id: "a".into(), content: "".into(), score: 1.0, source: "keyword".into() },
            ScoredResult { id: "b".into(), content: "".into(), score: 0.9, source: "keyword".into() },
            ScoredResult { id: "c".into(), content: "".into(), score: 0.8, source: "keyword".into() },
        ],
        // Result set 2: vector search
        vec![
            ScoredResult { id: "b".into(), content: "".into(), score: 1.0, source: "vector".into() },
            ScoredResult { id: "d".into(), content: "".into(), score: 0.9, source: "vector".into() },
            ScoredResult { id: "a".into(), content: "".into(), score: 0.8, source: "vector".into() },
        ],
    ];

    let fused = reciprocal_rank_fusion(results, 60);
    let ids: Vec<_> = fused.iter().map(|r| r.id.clone()).collect();

    // "a" and "b" appear in both, should rank highest
    assert_eq!(ids[0], "b");  // b appears at rank 1 in vector, rank 2 in keyword
    assert_eq!(ids[1], "a");  // a appears at rank 1 in keyword, rank 3 in vector
}

#[test]
fn test_rrf_with_empty_result_set() {
    let results = vec![
        vec![
            ScoredResult { id: "a".into(), content: "".into(), score: 1.0, source: "keyword".into() },
        ],
        vec![],  // Empty vector results
    ];

    let fused = reciprocal_rank_fusion(results, 60);
    assert_eq!(fused.len(), 1);
    assert_eq!(fused[0].id, "a");
}
```

## Priority

**🔴 CRITICAL** - Depends on Phase 1 (Embeddings)

---

*Issue created: 2026-04-15*
