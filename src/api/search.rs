use axum::{extract::Extension, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};

use crate::{
    memory::{qmd_memory::MemoryDocument, schema::MemoryQueryFilters},
    search::hybrid::HybridSearcher,
    workspace::WorkspaceContext,
};

#[derive(Debug, Deserialize)]
pub struct HybridSearchRequest {
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub rrf_k: Option<u32>,
    #[serde(default = "default_keyword_weight")]
    pub keyword_weight: f32,
    #[serde(default = "default_vector_weight")]
    pub vector_weight: f32,
    #[serde(default)]
    pub filters: Option<MemoryQueryFilters>,
    #[serde(default)]
    pub include_embedding: Option<bool>,
}

fn default_keyword_weight() -> f32 {
    0.5
}

fn default_vector_weight() -> f32 {
    0.5
}

fn default_limit() -> usize {
    10
}

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub id: String,
    pub content: String,
    pub score: f32,
    pub source: String,
    pub path: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub query_vector: Option<Vec<f32>>,
    pub total_available: usize,
    pub search_type: String,
}

pub async fn hybrid_search(
    Extension(workspace): Extension<WorkspaceContext>,
    Json(request): Json<HybridSearchRequest>,
) -> impl IntoResponse {
    let searcher = HybridSearcher {
        keyword_weight: request.keyword_weight,
        vector_weight: request.vector_weight,
        rrf_k: request.rrf_k.unwrap_or(60),
        hooks: crate::search::hooks::HookRegistry::new(),
    };

    let results = searcher
        .search(
            &workspace.workspace.memory,
            &request.query,
            request.limit,
            request.filters.as_ref(),
        )
        .await
        .unwrap_or_default();

    let query_vector = if request.include_embedding.unwrap_or(false) {
        match crate::embedding::build_embedder_from_env().await {
            Ok(embedder) => crate::embedding::Embedder::encode(embedder.as_ref(), &request.query)
                .await
                .ok()
                .filter(|vector| !vector.is_empty()),
            Err(_) => None,
        }
    } else {
        None
    };

    let response = SearchResponse {
        results: build_results(&workspace.workspace.memory, results).await,
        query_vector,
        total_available: workspace.workspace.memory.all_documents().await.len(),
        search_type: "hybrid".to_string(),
    };

    Json(response)
}

async fn build_results(
    memory: &crate::memory::qmd_memory::QmdMemory,
    results: Vec<crate::search::rrf::ScoredResult>,
) -> Vec<SearchResult> {
    let mut output = Vec::with_capacity(results.len());
    for result in results {
        let id = result.id.clone();
        let document = find_document(memory, &id).await;
        output.push(SearchResult {
            id,
            content: result.content,
            score: result.score,
            source: result.source,
            path: document
                .as_ref()
                .map(|doc| doc.path.clone())
                .unwrap_or_default(),
            metadata: document
                .map(|doc| doc.metadata)
                .unwrap_or_else(|| serde_json::json!({})),
        });
    }
    output
}

async fn find_document(
    memory: &crate::memory::qmd_memory::QmdMemory,
    id: &str,
) -> Option<MemoryDocument> {
    memory.get(id).await.ok().flatten()
}
