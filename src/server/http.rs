//! HTTP handlers for the minimal Xavier2 vertical slice.

use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{
    agents::provider::ModelProviderClient,
    agents::runtime::AgentRunTrace,
    agents::runtime::System3Mode,
    consolidation::ConsolidationTask,
    consistency::regularization::{CoherenceReport, RetentionRegularizer},
    embedding,
    memory::belief_graph::{BeliefNode, BeliefRelation},
    memory::entity_graph::EntityRecord,
    memory::qmd_memory::MemoryDocument,
    memory::schema::{MemoryQueryFilters, TypedMemoryPayload},
    memory::semantic::SemanticMemoryExt,
    memory::sqlite_vec_store::VecSqliteMemoryStore,
    memory::surreal_store::{GraphHopResult, HybridSearchMode},
    retrieval::gating::{AdaptiveGating, LayerWeights, SessionSummary},
    utils::crypto::sha256_hex,
    workspace::WorkspaceContext,
    AppState,
};

#[derive(Debug, Clone)]
pub struct HttpConfig {
    pub host: String,
    pub port: u16,
    pub tls_enabled: bool,
    pub tls_cert_path: Option<String>,
    pub tls_key_path: Option<String>,
}

impl HttpConfig {
    pub fn new(host: String, port: u16) -> Self {
        Self {
            host,
            port,
            tls_enabled: false,
            tls_cert_path: None,
            tls_key_path: None,
        }
    }

    pub fn with_tls(mut self, cert_path: String, key_path: String) -> Self {
        self.tls_enabled = true;
        self.tls_cert_path = Some(cert_path);
        self.tls_key_path = Some(key_path);
        self
    }
}

pub struct HttpServer {
    #[allow(dead_code)]
    config: HttpConfig,
}

impl HttpServer {
    pub fn new(config: HttpConfig) -> Self {
        Self { config }
    }

    pub async fn serve(&self) {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}

#[derive(Debug, Deserialize)]
pub struct CodeScanRequest {
    pub path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CodeFindRequest {
    #[serde(default)]
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub pattern: Option<String>,
}

fn default_limit() -> usize {
    10
}

fn query_fingerprint(query: &str) -> String {
    sha256_hex(query.as_bytes())[..12].to_string()
}

async fn record_optimization_trace(
    workspace: &WorkspaceContext,
    trace: &AgentRunTrace,
) -> anyhow::Result<()> {
    workspace
        .workspace
        .record_optimization(
            trace.optimization.route_category,
            trace.optimization.semantic_cache_hit,
            trace.optimization.llm_used,
            trace.optimization.model.as_deref(),
        )
        .await
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CodeScanResponse {
    pub status: String,
    pub indexed_files: usize,
    pub indexed_chunks: usize,
    pub paths: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CodeFindResponse {
    pub status: String,
    pub results: Vec<CodeSymbol>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CodeSymbol {
    pub path: String,
    pub symbol: String,
    pub symbol_type: String,
    pub line: usize,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CodeStatsResponse {
    pub status: String,
    pub total_files: usize,
    pub total_chunks: usize,
}

#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub filters: Option<MemoryQueryFilters>,
    #[serde(default)]
    pub system3_mode: Option<System3Mode>,
}

#[derive(Debug, Deserialize)]
pub struct HybridSearchRequest {
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default, rename = "type")]
    pub search_type: Option<HybridSearchMode>,
    #[serde(default)]
    pub filters: Option<MemoryQueryFilters>,
}

#[derive(Debug, Deserialize)]
pub struct AddMemoryRequest {
    pub content: String,
    pub path: Option<String>,
    pub metadata: Option<serde_json::Value>,
    #[serde(default)]
    pub kind: Option<crate::memory::schema::MemoryKind>,
    #[serde(default)]
    pub evidence_kind: Option<crate::memory::schema::EvidenceKind>,
    #[serde(default)]
    pub namespace: Option<crate::memory::schema::MemoryNamespace>,
    #[serde(default)]
    pub provenance: Option<crate::memory::schema::MemoryProvenance>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteMemoryRequest {
    pub id: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AgentRunRequest {
    pub query: String,
    pub session_id: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub filters: Option<MemoryQueryFilters>,
    #[serde(default)]
    pub system3_mode: Option<System3Mode>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResponse {
    pub status: String,
    pub results: Vec<serde_json::Value>,
    pub query: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HybridSearchResponse {
    pub status: String,
    pub results: Vec<serde_json::Value>,
    pub query: String,
    pub mode: HybridSearchMode,
}

/// Request for multi-layer memory retrieval
#[derive(Debug, Deserialize)]
pub struct MultiLayerRetrieveRequest {
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Layer weights as JSON string: {"working":0.3,"episodic":0.3,"semantic":0.4}
    #[serde(default)]
    pub layer_weights: Option<LayerWeights>,
    /// Relevance threshold (0.0-1.0)
    #[serde(default = "default_relevance_threshold")]
    pub relevance_threshold: f32,
    /// RRF k parameter
    #[serde(default = "default_rrf_k")]
    pub rrf_k: u32,
    /// Include coherence report
    #[serde(default)]
    pub include_coherence: bool,
}

fn default_relevance_threshold() -> f32 {
    0.3
}

fn default_rrf_k() -> u32 {
    60
}

/// Response for multi-layer memory retrieval
#[derive(Debug, Serialize)]
pub struct MultiLayerRetrieveResponse {
    pub status: String,
    pub results: Vec<RetrievedMemory>,
    pub query: String,
    pub layers_used: LayerStatsJson,
    pub coherence_report: Option<CoherenceReport>,
}

#[derive(Debug, Serialize)]
pub struct RetrievedMemory {
    pub id: String,
    pub content: String,
    pub score: f32,
    pub source_layer: String,
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct LayerStatsJson {
    pub working_count: usize,
    pub episodic_count: usize,
    pub semantic_count: usize,
    pub total_results: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryResponse {
    pub status: String,
    pub response: String,
    pub confidence: f32,
    pub session_id: String,
}

#[derive(Debug, Serialize)]
pub struct GraphResponse {
    pub status: String,
    pub nodes: Vec<BeliefNode>,
    pub edges: Vec<BeliefRelation>,
}

#[derive(Debug, Deserialize)]
pub struct GraphHopsRequest {
    pub path: String,
    pub hops: usize,
    #[serde(default)]
    pub query: String,
}

#[derive(Debug, Serialize)]
pub struct GraphHopsResponse {
    pub status: String,
    pub result: GraphHopResult,
}

#[derive(Debug, Serialize)]
pub struct AgentResponse {
    pub status: String,
    pub session_id: String,
    pub response: String,
    pub confidence: f32,
}

#[derive(Debug, Serialize)]
pub struct SyncResponse {
    pub status: String,
    pub synced: usize,
}

#[derive(Debug, Deserialize)]
pub struct BridgeImportRequest {
    pub source: crate::memory::bridge::BridgeSource,
    pub path: String,
    #[serde(default)]
    pub project: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub agent_id: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BridgeImportResponse {
    pub status: String,
    pub source: String,
    pub imported: usize,
    pub skipped: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteMemoryResponse {
    pub status: String,
    pub deleted: bool,
    pub id: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResetMemoryResponse {
    pub status: String,
    pub removed: usize,
}

pub async fn health() -> impl IntoResponse {
    const HEALTH_JSON: &str = concat!(
        "{\"status\":\"ok\",\"service\":\"xavier2\",\"version\":\"",
        env!("CARGO_PKG_VERSION"),
        "\"}"
    );
    axum::response::Response::builder()
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(HEALTH_JSON))
        .unwrap()
}

#[derive(Debug, Serialize)]
pub struct MemoryStoreBuildInfo {
    pub selected_backend: String,
    pub backend: String,
    pub migrated_from_file: bool,
    pub migration_detail: String,
    pub rrf_k: usize,
    pub entity_extraction_enabled: bool,
    pub qjl_threshold: usize,
    pub audit_chain_enabled: bool,
}

#[derive(Debug, Serialize)]
pub struct BuildInfoResponse {
    pub service: String,
    pub version: String,
    pub rust_log: Option<String>,
    pub xavier2_log_level: Option<String>,
    pub model_provider: crate::agents::provider::ModelProviderStatus,
    pub memory_store: MemoryStoreBuildInfo,
}

pub async fn build_info(State(state): State<AppState>) -> impl IntoResponse {
    let workspace = state.workspace_registry.default_context().await;

    Json(BuildInfoResponse {
        service: "xavier2".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        rust_log: std::env::var("RUST_LOG").ok(),
        xavier2_log_level: std::env::var("XAVIER2_LOG_LEVEL").ok(),
        model_provider: ModelProviderClient::from_env().status(),
        memory_store: workspace
            .map(|workspace| MemoryStoreBuildInfo {
                selected_backend: workspace
                    .workspace
                    .config()
                    .memory_backend
                    .as_str()
                    .to_string(),
                backend: workspace.workspace.durable_store_backend().to_string(),
                migrated_from_file: workspace.workspace.durable_store_migrated_from_file(),
                migration_detail: workspace
                    .workspace
                    .durable_store_migration_detail()
                    .to_string(),
                rrf_k: VecSqliteMemoryStore::configured_rrf_k(),
                entity_extraction_enabled: VecSqliteMemoryStore::entity_extraction_enabled(),
                qjl_threshold: VecSqliteMemoryStore::configured_qjl_threshold(),
                audit_chain_enabled: VecSqliteMemoryStore::audit_chain_enabled(),
            })
            .unwrap_or(MemoryStoreBuildInfo {
                selected_backend: std::env::var("XAVIER2_MEMORY_BACKEND")
                    .map(|value| {
                        crate::memory::surreal_store::MemoryBackend::from_env(&value)
                            .as_str()
                            .to_string()
                    })
                    .unwrap_or_else(|_| "vec".to_string()),
                backend: "unavailable".to_string(),
                migrated_from_file: false,
                migration_detail: "default workspace is not available".to_string(),
                rrf_k: VecSqliteMemoryStore::configured_rrf_k(),
                entity_extraction_enabled: VecSqliteMemoryStore::entity_extraction_enabled(),
                qjl_threshold: VecSqliteMemoryStore::configured_qjl_threshold(),
                audit_chain_enabled: VecSqliteMemoryStore::audit_chain_enabled(),
            }),
    })
}

#[derive(Debug, Serialize)]
pub struct ReadinessComponent {
    pub configured: bool,
    pub ready: bool,
    pub detail: String,
}

#[derive(Debug, Serialize)]
pub struct ReadinessResponse {
    pub status: String,
    pub service: String,
    pub version: String,
    pub workspace: ReadinessComponent,
    pub memory_store: ReadinessComponent,
    pub code_graph: ReadinessComponent,
    pub embeddings: ReadinessComponent,
    pub llm: ReadinessComponent,
}

pub async fn readiness(State(state): State<AppState>) -> impl IntoResponse {
    let workspace_context = state.workspace_registry.default_context().await;
    let workspace_ready = workspace_context.is_some();
    let embedding_configured = std::env::var("XAVIER2_EMBEDDING_URL").is_ok();
    let embeddings = match crate::memory::embedder::EmbeddingClient::from_env() {
        Ok(client) if embedding_configured => match client.health().await {
            Ok(true) => ReadinessComponent {
                configured: true,
                ready: true,
                detail: "embedding service reachable".to_string(),
            },
            Ok(false) => ReadinessComponent {
                configured: true,
                ready: false,
                detail: "embedding service responded without vectors".to_string(),
            },
            Err(error) => ReadinessComponent {
                configured: true,
                ready: false,
                detail: error.to_string(),
            },
        },
        Ok(_) => ReadinessComponent {
            configured: false,
            ready: true,
            detail: "embedding service not configured".to_string(),
        },
        Err(error) if embedding_configured => ReadinessComponent {
            configured: true,
            ready: false,
            detail: error.to_string(),
        },
        Err(_) => ReadinessComponent {
            configured: false,
            ready: true,
            detail: "embedding service not configured".to_string(),
        },
    };

    let llm_status = ModelProviderClient::from_env().status();
    let llm = ReadinessComponent {
        configured: llm_status.configured,
        ready: llm_status.configured,
        detail: format!(
            "provider={} model={}",
            llm_status.provider, llm_status.model
        ),
    };

    let workspace = ReadinessComponent {
        configured: true,
        ready: workspace_ready,
        detail: if workspace_ready {
            "default workspace loaded".to_string()
        } else {
            "default workspace is not available".to_string()
        },
    };

    let memory_store = match workspace_context {
        Some(workspace) => match workspace.workspace.durable_store_health().await {
            Ok(detail) => ReadinessComponent {
                configured: true,
                ready: true,
                detail: format!(
                    "{detail}; migration={}",
                    workspace.workspace.durable_store_migration_detail()
                ),
            },
            Err(error) => ReadinessComponent {
                configured: true,
                ready: false,
                detail: error.to_string(),
            },
        },
        None => ReadinessComponent {
            configured: true,
            ready: false,
            detail: "default workspace is not available".to_string(),
        },
    };

    let code_graph = match state.code_db.stats() {
        Ok(stats) => ReadinessComponent {
            configured: true,
            ready: true,
            detail: format!(
                "code graph reachable (files={}, symbols={})",
                stats.total_files, stats.total_symbols
            ),
        },
        Err(error) => ReadinessComponent {
            configured: true,
            ready: false,
            detail: error.to_string(),
        },
    };

    let ready = workspace.ready
        && memory_store.ready
        && code_graph.ready
        && (!embeddings.configured || embeddings.ready)
        && (!llm.configured || llm.ready);

    Json(ReadinessResponse {
        status: if ready { "ok" } else { "degraded" }.to_string(),
        service: "xavier2".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        workspace,
        memory_store,
        code_graph,
        embeddings,
        llm,
    })
}

pub async fn memory_add(
    Extension(workspace): Extension<WorkspaceContext>,
    Json(payload): Json<AddMemoryRequest>,
) -> impl IntoResponse {
    info!(
        path = payload.path.as_deref().unwrap_or("default"),
        "memory_add"
    );

    let path = payload.path.unwrap_or_else(|| "default".to_string());
    let content = payload.content;
    let mut metadata = payload.metadata.unwrap_or(serde_json::json!({}));
    if let Some(object) = metadata.as_object_mut() {
        let agent_id = object
            .get("agent_id")
            .and_then(|value| value.as_str())
            .unwrap_or("http");
        object.insert(
            "_audit".to_string(),
            serde_json::json!({
                "agent_id": agent_id,
                "operation": "memory.add"
            }),
        );
    }
    let typed = TypedMemoryPayload {
        kind: payload.kind,
        evidence_kind: payload.evidence_kind,
        namespace: payload.namespace,
        provenance: payload.provenance,
    };
    let content_vector = match embedding::build_embedder_from_env().await {
        Ok(embedder) => match embedder.encode(&content).await {
            Ok(vector) if !vector.is_empty() => Some(vector),
            Ok(_) => None,
            Err(error) => {
                tracing::warn!(%error, "embedding generation failed; storing memory without vector");
                None
            }
        },
        Err(error) => {
            tracing::warn!(%error, "embedding provider unavailable; storing memory without vector");
            None
        }
    };

    if let Err(error) = workspace
        .workspace
        .ensure_within_storage_limit(&path, &content, &metadata)
        .await
    {
        return Json(serde_json::json!({
            "status": "error",
            "message": error.to_string(),
            "workspace_id": workspace.workspace_id,
        }));
    }

    let memory_id = match workspace
        .workspace
        .memory
        .add_document_typed_with_embedding(path, content.clone(), metadata.clone(), Some(typed), content_vector)
        .await
    {
        Ok(id) => id,
        Err(error) => {
            tracing::error!(%error, workspace_id = %workspace.workspace_id, "failed to add memory document");
            return Json(serde_json::json!({
                "status": "error",
                "message": format!("failed to add memory: {}", error),
                "workspace_id": workspace.workspace_id,
            }));
        }
    };

    if let Err(error) = workspace
        .workspace
        .index_memory_entities(&memory_id, &content, &metadata)
        .await
    {
        tracing::warn!(%error, memory_id = %memory_id, "failed to index entity graph from memory_add");
    }

    // Also index into semantic memory (NER-style entity extraction)
    if let Err(error) = workspace
        .workspace
        .semantic_memory
        .index_memory(&memory_id, &content)
        .await
    {
        tracing::warn!(%error, memory_id = %memory_id, "failed to index semantic memory from memory_add");
    }

    Json(serde_json::json!({
        "status": "ok",
        "message": "Document added to memory",
        "workspace_id": workspace.workspace_id,
    }))
}

pub async fn memory_search(
    Extension(workspace): Extension<WorkspaceContext>,
    Json(payload): Json<SearchRequest>,
) -> impl IntoResponse {
    info!(
        query_fingerprint = %query_fingerprint(&payload.query),
        "memory_search"
    );

    let results = workspace
        .workspace
        .memory
        .search_filtered(&payload.query, payload.limit, payload.filters.as_ref())
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|doc: MemoryDocument| {
            serde_json::json!({
                "id": doc.id,
                "path": doc.path,
                "content": doc.content,
                "metadata": doc.metadata,
            })
        })
        .collect();

    Json(SearchResponse {
        status: "ok".to_string(),
        results,
        query: payload.query,
    })
}

pub async fn memory_hybrid_search(
    Extension(workspace): Extension<WorkspaceContext>,
    Json(payload): Json<HybridSearchRequest>,
) -> impl IntoResponse {
    let mode = payload.search_type.unwrap_or_default();
    info!(
        query_fingerprint = %query_fingerprint(&payload.query),
        mode = ?mode,
        "memory_hybrid_search"
    );

    match workspace
        .workspace
        .durable_store()
        .hybrid_search(
            &workspace.workspace_id,
            &payload.query,
            mode,
            payload.filters.as_ref(),
            payload.limit,
        )
        .await
    {
        Ok(results) => Json(HybridSearchResponse {
            status: "ok".to_string(),
            results: results
                .into_iter()
                .map(|result| {
                    serde_json::json!({
                        "id": result.record.id,
                        "path": result.record.path,
                        "content": result.record.content,
                        "metadata": result.record.metadata,
                        "score": result.score,
                        "vector_score": result.vector_score,
                        "lexical_score": result.lexical_score,
                        "kg_score": result.kg_score,
                        "bm25": result.bm25,
                    })
                })
                .collect(),
            query: payload.query,
            mode,
        })
        .into_response(),
        Err(error) => Json(serde_json::json!({
            "status": "error",
            "message": error.to_string(),
            "query": payload.query,
            "mode": mode,
        }))
        .into_response(),
    }
}

/// Multi-layer memory retrieval with adaptive gating
pub async fn memory_retrieve(
    Extension(workspace): Extension<WorkspaceContext>,
    Json(payload): Json<MultiLayerRetrieveRequest>,
) -> impl IntoResponse {
    info!(
        query_fingerprint = %query_fingerprint(&payload.query),
        limit = payload.limit,
        "memory_retrieve"
    );

    let weights = payload.layer_weights.unwrap_or_else(LayerWeights::default);

    // Configure adaptive gating with the caller's requested limit so we do not
    // truncate results at the module default before applying the HTTP limit.
    let gating = AdaptiveGating::new(crate::retrieval::gating::GatingConfig {
        layer_weights: weights,
        relevance_threshold: payload.relevance_threshold.clamp(0.0, 1.0),
        rrf_k: payload.rrf_k,
        max_results: payload.limit.max(1),
    });

    // Collect working memory documents
    let working_docs = workspace.workspace.memory.all_documents().await;
    let working_count = working_docs.len();

    // Collect episodic summaries from panel store (session history)
    let episodic_summaries = workspace
        .workspace
        .panel_store
        .list_threads()
        .await
        .into_iter()
        .map(|s| SessionSummary {
            session_id: s.id.clone(),
            start_time: s.created_at,
            summary: s.last_preview.clone(),
            key_events: vec![],
            sentiment_timeline: vec![],
        })
        .collect::<Vec<_>>();
    let episodic_count = episodic_summaries.len();

    // Collect semantic entities
    let semantic_entities: Vec<EntityRecord> = workspace
        .workspace
        .entity_graph
        .all_entities()
        .await;
    let semantic_count = semantic_entities.len();

    // Perform multi-layer retrieval
    let results = gating.retrieve(
        &working_docs,
        &episodic_summaries,
        &semantic_entities,
        &payload.query,
    );

    let total_results = results.len();

    // Optionally compute coherence report
    let coherence_report = if payload.include_coherence {
        let regularizer = RetentionRegularizer::with_defaults();
        Some(regularizer.check_coherence_with_entities(
            &working_docs,
            &semantic_entities,
        ))
    } else {
        None
    };

    // Build response
    let retrieved: Vec<RetrievedMemory> = results
        .into_iter()
        .take(payload.limit)
        .map(|r| {
            let crate::search::rrf::ScoredResult {
                id,
                content,
                score,
                source,
            } = r;

            RetrievedMemory {
                path: retrieved_path_for_result(&working_docs, &id, &source),
                id,
                content,
                score,
                source_layer: source,
            }
        })
        .collect();

    Json(MultiLayerRetrieveResponse {
        status: "ok".to_string(),
        results: retrieved,
        query: payload.query.clone(),
        layers_used: LayerStatsJson {
            working_count,
            episodic_count,
            semantic_count,
            total_results,
        },
        coherence_report,
    })
}

fn retrieved_path_for_result(
    working_docs: &[MemoryDocument],
    result_id: &str,
    source_layer: &str,
) -> String {
    if let Some(path) = working_docs
        .iter()
        .find(|document| document.id.as_deref() == Some(result_id))
        .map(|document| document.path.clone())
    {
        return path;
    }

    match source_layer {
        "episodic" => format!("panel/threads/{result_id}"),
        "semantic" => format!("semantic/entities/{result_id}"),
        _ => result_id.to_string(),
    }
}

pub async fn memory_curate(
    Extension(workspace): Extension<WorkspaceContext>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let doc_id = payload
        .get("id")
        .and_then(|id| id.as_str())
        .map(|s| s.to_string());

    if let Some(id) = doc_id {
        info!("🧠 Manual curation request for doc: {}", id);
        let action = crate::memory::manager::MemoryAction::Curate { doc_id: id };
        match workspace
            .workspace
            .memory_manager
            .execute_actions(vec![action])
            .await
        {
            Ok(_) => {
                if let Err(error) = workspace.workspace.persist_beliefs().await {
                    return Json(serde_json::json!({
                        "status": "error",
                        "message": error.to_string(),
                    }));
                }
                Json(serde_json::json!({ "status": "ok", "message": "Curation completed" }))
            }
            Err(e) => Json(serde_json::json!({ "status": "error", "message": e.to_string() })),
        }
    } else {
        Json(serde_json::json!({ "status": "error", "message": "Missing 'id' in request body" }))
    }
}

pub async fn memory_manage(Extension(workspace): Extension<WorkspaceContext>) -> impl IntoResponse {
    info!("⚙️ Memory management auto-run requested");
    match workspace.workspace.memory_manager.auto_manage().await {
        Ok(count) => {
            if let Err(error) = workspace.workspace.persist_beliefs().await {
                return Json(serde_json::json!({
                    "status": "error",
                    "message": error.to_string(),
                }));
            }
            Json(serde_json::json!({ "status": "ok", "actions_executed": count }))
        }
        Err(e) => Json(serde_json::json!({ "status": "error", "message": e.to_string() })),
    }
}

pub async fn memory_decay(Extension(workspace): Extension<WorkspaceContext>) -> impl IntoResponse {
    info!("📉 Memory decay request");
    match workspace.workspace.memory_manager.decay_memories().await {
        Ok(result) => {
            if let Err(error) = workspace.workspace.persist_beliefs().await {
                return Json(serde_json::json!({
                    "status": "error",
                    "message": error.to_string(),
                }));
            }
            Json(serde_json::json!({
                "status": "ok",
                "documents_affected": result.documents_affected,
                "actions": result.actions.len(),
                "bytes_freed": result.bytes_freed,
            }))
        }
        Err(e) => Json(serde_json::json!({"status": "error", "message": e.to_string() })),
    }
}

pub async fn memory_consolidate(
    Extension(workspace): Extension<WorkspaceContext>,
) -> impl IntoResponse {
    info!("🔗 Memory consolidation request");
    let task = ConsolidationTask::default();
    match task.consolidate(&workspace).await {
        Ok(stats) => Json(serde_json::json!({
            "status": "ok",
            "stats": stats,
        })),
        Err(e) => Json(serde_json::json!({"status": "error", "message": e.to_string() })),
    }
}

pub async fn memory_reflect(Extension(workspace): Extension<WorkspaceContext>) -> impl IntoResponse {
    info!("🪞 Memory reflection request");
    let task = ConsolidationTask::default();
    match task.reflect(&workspace).await {
        Ok(stats) => Json(serde_json::json!({
            "status": "ok",
            "stats": stats,
        })),
        Err(e) => Json(serde_json::json!({"status": "error", "message": e.to_string() })),
    }
}

#[derive(Debug, Deserialize)]
pub struct MemoryQualityQuery {
    pub threshold: Option<f32>,
}

pub async fn memory_quality(
    Extension(workspace): Extension<WorkspaceContext>,
    Query(params): Query<MemoryQualityQuery>,
) -> impl IntoResponse {
    info!("📊 Memory quality request");
    let threshold = params.threshold.unwrap_or(0.3);

    match workspace
        .workspace
        .memory_manager
        .get_low_quality_memories(threshold)
        .await
    {
        Ok(memories) => {
            let results: Vec<serde_json::Value> = memories
                .iter()
                .map(|m| {
                    serde_json::json!({
                        "id": m.doc.id,
                        "path": m.doc.path,
                        "priority": m.priority.as_str(),
                        "quality": {
                            "overall": m.quality.overall,
                            "relevance": m.quality.relevance_score,
                            "accuracy": m.quality.accuracy_score,
                            "freshness": m.quality.freshness_score,
                            "completeness": m.quality.completeness_score,
                        },
                        "access_count": m.access_count,
                        "last_access": m.last_access.map(|t| t.to_rfc3339()),
                    })
                })
                .collect();

            Json(serde_json::json!({
                "status": "ok",
                "threshold": threshold,
                "count": memories.len(),
                "memories": results,
            }))
        }
        Err(e) => Json(serde_json::json!({"status": "error", "message": e.to_string() })),
    }
}

#[derive(Debug, Deserialize)]
pub struct MemoryEvictQuery {
    pub priority: Option<String>,
    pub threshold: Option<f32>,
}

pub async fn memory_evict(
    Extension(workspace): Extension<WorkspaceContext>,
    Query(params): Query<MemoryEvictQuery>,
) -> impl IntoResponse {
    info!("🗑️ Memory eviction request");

    let result = if let Some(priority_str) = &params.priority {
        let priority = match priority_str.to_lowercase().as_str() {
            "critical" => crate::memory::manager::MemoryPriority::Critical,
            "high" => crate::memory::manager::MemoryPriority::High,
            "medium" => crate::memory::manager::MemoryPriority::Medium,
            "low" => crate::memory::manager::MemoryPriority::Low,
            "ephemeral" => crate::memory::manager::MemoryPriority::Ephemeral,
            _ => {
                return Json(serde_json::json!({
                    "status": "error",
                    "message": format!("Unknown priority: {}", priority_str),
                }));
            }
        };
        workspace
            .workspace
            .memory_manager
            .evict_by_priority(priority)
            .await
    } else {
        workspace.workspace.memory_manager.evict_low_quality().await
    };

    match result {
        Ok(r) => {
            if let Err(error) = workspace.workspace.persist_beliefs().await {
                return Json(serde_json::json!({
                    "status": "error",
                    "message": error.to_string(),
                }));
            }
            Json(serde_json::json!({
                "status": "ok",
                "documents_affected": r.documents_affected,
                "actions": r.actions.len(),
                "bytes_freed": r.bytes_freed,
            }))
        }
        Err(e) => Json(serde_json::json!({"status": "error", "message": e.to_string() })),
    }
}

pub async fn memory_stats(Extension(workspace): Extension<WorkspaceContext>) -> impl IntoResponse {
    info!("📈 Memory stats request");
    match workspace.workspace.memory_manager.get_stats().await {
        Ok(stats) => Json(serde_json::json!({
            "status": "ok",
            "total_documents": stats.total_documents,
            "total_size_bytes": stats.total_size_bytes,
            "by_priority": stats.by_priority,
            "by_quality_bucket": stats.by_quality_bucket,
            "low_quality_count": stats.low_quality_count,
            "ephemeral_count": stats.ephemeral_count,
        })),
        Err(e) => Json(serde_json::json!({"status": "error", "message": e.to_string() })),
    }
}

pub async fn memory_delete(
    Extension(workspace): Extension<WorkspaceContext>,
    Json(payload): Json<DeleteMemoryRequest>,
) -> impl IntoResponse {
    let target = payload.id.clone().or(payload.path.clone());

    let Some(target) = target else {
        return Json(DeleteMemoryResponse {
            status: "error: missing id or path".to_string(),
            deleted: false,
            id: payload.id,
            path: payload.path,
        });
    };

    info!("🗑️ Delete request: {}", target);

    match workspace.workspace.memory.delete(&target).await {
        Ok(Some(doc)) => {
            if let Some(memory_id) = doc.id.clone().or_else(|| Some(doc.path.clone())) {
                if let Err(error) = workspace.workspace.remove_memory_entities(&memory_id).await {
                    tracing::warn!(%error, memory_id = %memory_id, "failed to remove entity graph memory index");
                }
            }
            Json(DeleteMemoryResponse {
                status: "ok".to_string(),
                deleted: true,
                id: doc.id,
                path: Some(doc.path),
            })
        }
        Ok(None) => Json(DeleteMemoryResponse {
            status: "not_found".to_string(),
            deleted: false,
            id: payload.id,
            path: payload.path,
        }),
        Err(error) => Json(DeleteMemoryResponse {
            status: format!("error: {}", error),
            deleted: false,
            id: payload.id,
            path: payload.path,
        }),
    }
}

pub async fn memory_query(
    Extension(workspace): Extension<WorkspaceContext>,
    Json(payload): Json<SearchRequest>,
) -> impl IntoResponse {
    info!(
        query_fingerprint = %query_fingerprint(&payload.query),
        "memory_query"
    );

    let response = workspace
        .workspace
        .runtime
        .run_with_trace_filtered(
            &payload.query,
            None,
            payload.category,
            payload.filters,
            payload.system3_mode.unwrap_or_default(),
        )
        .await;

    match response {
        Ok(trace) => {
            if let Err(error) = record_optimization_trace(&workspace, &trace).await {
                return Json(QueryResponse {
                    status: format!("error: {}", error),
                    response: String::new(),
                    confidence: 0.0,
                    session_id: String::new(),
                });
            }

            Json(QueryResponse {
                status: "ok".to_string(),
                response: trace.agent.response,
                confidence: trace.agent.confidence,
                session_id: trace.agent.session_id,
            })
        }
        Err(error) => Json(QueryResponse {
            status: format!("error: {}", error),
            response: String::new(),
            confidence: 0.0,
            session_id: String::new(),
        }),
    }
}

pub async fn memory_reset(Extension(workspace): Extension<WorkspaceContext>) -> impl IntoResponse {
    info!("♻️ Reset memory request");

    match workspace.workspace.memory.clear().await {
        Ok(removed) => Json(ResetMemoryResponse {
            status: "ok".to_string(),
            removed,
        }),
        Err(error) => Json(ResetMemoryResponse {
            status: format!("error: {}", error),
            removed: 0,
        }),
    }
}

pub async fn memory_graph(Extension(workspace): Extension<WorkspaceContext>) -> impl IntoResponse {
    info!("🔗 Graph request");

    let graph = workspace.workspace.belief_graph.read().await;

    Json(GraphResponse {
        status: "ok".to_string(),
        nodes: graph.list_nodes(),
        edges: graph.get_relations(),
    })
}

pub async fn memory_graph_hops(
    Extension(workspace): Extension<WorkspaceContext>,
    Json(payload): Json<GraphHopsRequest>,
) -> impl IntoResponse {
    info!(path = %payload.path, hops = payload.hops, "memory_graph_hops");

    match workspace
        .workspace
        .durable_store()
        .graph_hops(
            &workspace.workspace_id,
            &payload.path,
            payload.hops.max(1),
            &payload.query,
        )
        .await
    {
        Ok(result) => Json(GraphHopsResponse {
            status: "ok".to_string(),
            result,
        })
        .into_response(),
        Err(error) => Json(serde_json::json!({
            "status": "error",
            "message": error.to_string(),
            "path": payload.path,
            "hops": payload.hops,
            "query": payload.query,
        }))
        .into_response(),
    }
}

pub async fn agents_run(
    Extension(workspace): Extension<WorkspaceContext>,
    Json(payload): Json<AgentRunRequest>,
) -> impl IntoResponse {
    info!(
        query_fingerprint = %query_fingerprint(&payload.query),
        "agents_run"
    );

    let response = workspace
        .workspace
        .runtime
        .run_with_trace_filtered(
            &payload.query,
            payload.session_id,
            payload.category,
            payload.filters,
            payload.system3_mode.unwrap_or_default(),
        )
        .await;

    match response {
        Ok(trace) => {
            if let Err(error) = record_optimization_trace(&workspace, &trace).await {
                return Json(AgentResponse {
                    status: format!("error: {}", error),
                    session_id: String::new(),
                    response: String::new(),
                    confidence: 0.0,
                });
            }
            if let Err(error) = workspace
                .workspace
                .record_session_exchange(
                    &trace.agent.session_id,
                    "http_agents_run",
                    &payload.query,
                    &trace.agent.response,
                )
                .await
            {
                return Json(AgentResponse {
                    status: format!("error: {}", error),
                    session_id: String::new(),
                    response: String::new(),
                    confidence: 0.0,
                });
            }

            Json(AgentResponse {
                status: "ok".to_string(),
                session_id: trace.agent.session_id,
                response: trace.agent.response,
                confidence: trace.agent.confidence,
            })
        }
        Err(error) => Json(AgentResponse {
            status: format!("error: {}", error),
            session_id: String::new(),
            response: String::new(),
            confidence: 0.0,
        }),
    }
}

pub async fn sync_tier1(Extension(workspace): Extension<WorkspaceContext>) -> impl IntoResponse {
    info!("🔄 Tier 1 sync request");

    let synced = workspace.workspace.memory.count().await.unwrap_or(0);

    Json(SyncResponse {
        status: "ok".to_string(),
        synced,
    })
}

pub async fn bridge_import(
    Extension(workspace): Extension<WorkspaceContext>,
    Json(payload): Json<BridgeImportRequest>,
) -> impl IntoResponse {
    info!(path = %payload.path, source = ?payload.source, "bridge_import");

    match crate::memory::bridge::import_from_path(
        &workspace.workspace.memory,
        payload.source,
        &payload.path,
        crate::memory::bridge::BridgeImportOptions {
            project: payload.project,
            scope: payload.scope,
            agent_id: payload.agent_id,
            session_id: payload.session_id,
        },
    )
    .await
    {
        Ok(stats) => Json(BridgeImportResponse {
            status: "ok".to_string(),
            source: stats.source,
            imported: stats.imported,
            skipped: stats.skipped,
        }),
        Err(error) => Json(BridgeImportResponse {
            status: format!("error: {}", error),
            source: String::new(),
            imported: 0,
            skipped: 0,
        }),
    }
}

#[derive(Debug, Serialize)]
pub struct AccountUsageResponse {
    pub status: String,
    #[serde(flatten)]
    pub usage: crate::workspace::WorkspaceUsageSnapshot,
}

#[derive(Debug, Serialize)]
pub struct AccountLimitsResponse {
    pub status: String,
    #[serde(flatten)]
    pub limits: crate::workspace::WorkspaceLimitsSnapshot,
}

#[derive(Debug, Serialize)]
pub struct SyncPoliciesResponse {
    pub status: String,
    #[serde(flatten)]
    pub sync: crate::workspace::SyncPolicySnapshot,
}

#[derive(Debug, Serialize)]
pub struct EmbeddingProviderStatusResponse {
    pub status: String,
    #[serde(flatten)]
    pub provider: crate::workspace::EmbeddingProviderSnapshot,
}

pub async fn account_usage(Extension(workspace): Extension<WorkspaceContext>) -> impl IntoResponse {
    Json(AccountUsageResponse {
        status: "ok".to_string(),
        usage: workspace.workspace.usage_snapshot().await,
    })
}

pub async fn account_limits(
    Extension(workspace): Extension<WorkspaceContext>,
) -> impl IntoResponse {
    Json(AccountLimitsResponse {
        status: "ok".to_string(),
        limits: workspace.workspace.limits_snapshot(),
    })
}

pub async fn sync_policies(Extension(workspace): Extension<WorkspaceContext>) -> impl IntoResponse {
    Json(SyncPoliciesResponse {
        status: "ok".to_string(),
        sync: workspace.workspace.sync_policy_snapshot(),
    })
}

pub async fn embedding_provider_status(
    Extension(workspace): Extension<WorkspaceContext>,
) -> impl IntoResponse {
    Json(EmbeddingProviderStatusResponse {
        status: "ok".to_string(),
        provider: workspace.workspace.embedding_provider_snapshot().await,
    })
}

pub async fn code_scan(
    State(state): State<AppState>,
    Json(payload): Json<CodeScanRequest>,
) -> impl IntoResponse {
    info!("📂 Code scan request: {:?}", payload.path);

    let path_str = payload.path.unwrap_or_else(|| ".".to_string());
    let path = std::path::Path::new(&path_str);

    match state.code_indexer.index(path).await {
        Ok(stats) => {
            Json(CodeScanResponse {
                status: "ok".to_string(),
                indexed_files: stats.total_files as usize,
                indexed_chunks: stats.total_symbols as usize, // Mapping symbols to chunks for compatibility
                paths: vec![path_str],
            })
        }
        Err(e) => Json(CodeScanResponse {
            status: format!("error: {}", e),
            indexed_files: 0,
            indexed_chunks: 0,
            paths: vec![],
        }),
    }
}

pub async fn code_find(
    State(state): State<AppState>,
    Json(payload): Json<CodeFindRequest>,
) -> impl IntoResponse {
    info!(
        "🔎 Code find request: {} (kind: {:?}, pattern: {:?})",
        payload.query, payload.kind, payload.pattern
    );

    // Filter by AST pattern if specified
    let symbols = if let Some(ref pattern) = payload.pattern {
        state
            .code_query
            .search_by_pattern(pattern, payload.limit)
            .unwrap_or_default()
    } else if let Some(ref kind) = payload.kind {
        match kind.to_lowercase().as_str() {
            "function" => state
                .code_query
                .functions(payload.limit)
                .unwrap_or_default(),
            "struct" => state.code_query.structs(payload.limit).unwrap_or_default(),
            "class" => state.code_query.classes(payload.limit).unwrap_or_default(),
            "enum" => state.code_query.enums(payload.limit).unwrap_or_default(),
            _ => state
                .code_query
                .search(&payload.query, payload.limit)
                .map(|r| r.symbols)
                .unwrap_or_default(),
        }
    } else {
        state
            .code_query
            .search(&payload.query, payload.limit)
            .map(|r| r.symbols)
            .unwrap_or_default()
    };

    let results: Vec<CodeSymbol> = symbols
        .into_iter()
        .map(|s| CodeSymbol {
            path: s.file_path,
            symbol: s.name,
            symbol_type: format!("{:?}", s.kind),
            line: s.start_line as usize,
            content: s.signature.unwrap_or_default(),
        })
        .collect();

    Json(CodeFindResponse {
        status: "ok".to_string(),
        results,
    })
}

pub async fn code_stats(State(state): State<AppState>) -> impl IntoResponse {
    info!("📊 Code stats request");

    // Get stats from code-graph database
    let stats = state
        .code_db
        .stats()
        .unwrap_or_else(|_| code_graph::types::IndexStats {
            total_files: 0,
            total_symbols: 0,
            total_imports: 0,
            languages: vec![],
            duration_ms: 0,
        });

    Json(CodeStatsResponse {
        status: "ok".to_string(),
        total_files: stats.total_files as usize,
        total_chunks: stats.total_symbols as usize,
    })
}

// ==========================================
// Security / Anticipator Handlers
// ==========================================

use crate::security::anticipator::Anticipator;

#[derive(Debug, Deserialize)]
pub struct SecurityScanRequest {
    pub message: String,
    pub context: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SecurityScanResponse {
    pub status: String,
    pub clean: bool,
    pub threats: Vec<serde_json::Value>,
    pub layers_triggered: Vec<String>,
    pub scan_ms: u64,
}

/// Global Anticipator scanner instance
static ANTICIPATOR: std::sync::LazyLock<Anticipator> = std::sync::LazyLock::new(Anticipator::new);

/// POST /security/scan - Scan a message for threats
pub async fn security_scan(Json(payload): Json<SecurityScanRequest>) -> impl IntoResponse {
    let result = ANTICIPATOR.scan(&payload.message);

    let threats: Vec<serde_json::Value> = result
        .threats
        .iter()
        .map(|t| {
            serde_json::json!({
                "severity": t.severity.as_str(),
                "layer": t.layer,
                "category": t.category.as_str(),
                "message": t.message,
                "evidence": t.evidence,
                "detection_method": t.detection_method
            })
        })
        .collect();

    Json(SecurityScanResponse {
        status: "ok".to_string(),
        clean: result.clean,
        threats,
        layers_triggered: result.layers_triggered,
        scan_ms: result.scan_ms,
    })
}

/// GET /security/config - Get security scanner configuration
pub async fn security_config() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "enabled_layers": ANTICIPATOR.enabled_layers(),
        "config": {
            "enable_phrase": ANTICIPATOR.config().enable_phrase,
            "enable_encoding": ANTICIPATOR.config().enable_encoding,
            "enable_entropy": ANTICIPATOR.config().enable_entropy,
            "enable_heuristic": ANTICIPATOR.config().enable_heuristic,
            "enable_canary": ANTICIPATOR.config().enable_canary,
            "enable_homoglyph": ANTICIPATOR.config().enable_homoglyph,
            "enable_path_traversal": ANTICIPATOR.config().enable_path_traversal,
            "enable_tool_alias": ANTICIPATOR.config().enable_tool_alias,
            "enable_threat_categories": ANTICIPATOR.config().enable_threat_categories,
            "enable_config_drift": ANTICIPATOR.config().enable_config_drift,
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
        routing::{delete, get, post},
        Router,
    };
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tower::util::ServiceExt;

    use crate::{
        agents::RuntimeConfig,
        memory::file_indexer::{FileIndexer, FileIndexerConfig},
        workspace::{WorkspaceConfig, WorkspaceContext, WorkspaceRegistry, WorkspaceState},
        AppState,
    };

    fn unique_test_path(prefix: &str, suffix: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{unique}-{suffix}"))
    }

    async fn test_state_with_backend(
        backend: crate::memory::surreal_store::MemoryBackend,
    ) -> (AppState, WorkspaceContext) {
        let db_path = unique_test_path("xavier2-code-http", "code_graph.db");
        let code_db = Arc::new(code_graph::db::CodeGraphDB::new(&db_path).unwrap());
        let code_indexer = Arc::new(code_graph::indexer::Indexer::new(Arc::clone(&code_db)));
        let code_query = Arc::new(code_graph::query::QueryEngine::new(Arc::clone(&code_db)));
        let workspace_registry = Arc::new(WorkspaceRegistry::new());
        let workspace = WorkspaceState::new(
            WorkspaceConfig {
                id: "test".to_string(),
                token: "test-token".to_string(),
                plan: crate::workspace::PlanTier::Personal,
                memory_backend: backend,
                storage_limit_bytes: Some(10 * 1024 * 1024),
                request_limit: Some(10_000),
                request_unit_limit: Some(20_000),
                embedding_provider_mode: crate::workspace::EmbeddingProviderMode::BringYourOwn,
                managed_google_embeddings: false,
                sync_policy: crate::workspace::SyncPolicy::CloudMirror,
            },
            RuntimeConfig::default(),
            unique_test_path("xavier2-panel-store", "threads"),
        )
        .await
        .unwrap();
        workspace_registry.insert(workspace).await.unwrap();
        let workspace = workspace_registry.authenticate("test-token").await.unwrap();

        (
            AppState {
                workspace_registry,
                indexer: FileIndexer::new(FileIndexerConfig::default(), Some(code_indexer.clone())),
                code_indexer,
                code_query,
                code_db,
                pattern_adapter: Arc::new(crate::adapters::outbound::vec::pattern_adapter::PatternAdapter::new()),
                security_service: Arc::new(crate::app::security_service::SecurityService::new()),
            },
            workspace,
        )
    }

    async fn test_state() -> (AppState, WorkspaceContext) {
        test_state_with_backend(crate::memory::surreal_store::MemoryBackend::File).await
    }

    fn test_router(state: AppState, workspace: WorkspaceContext) -> Router {
        Router::new()
            .route("/build", get(build_info))
            .route("/readiness", get(readiness))
            .route("/memory/add", post(memory_add))
            .route("/memory/delete", post(memory_delete))
            .route("/memory/query", post(memory_query))
            .route("/memory/reset", post(memory_reset))
            .route("/memory/search", post(memory_search))
            .route("/memory/hybrid", post(crate::api::search::hybrid_search))
            .route("/memory/hybrid-search", post(memory_hybrid_search))
            .route("/memory/retrieve", post(memory_retrieve))
            .route(
                "/memory/graph/entity/{entity_id}",
                get(crate::api::graph::memory_graph_entity),
            )
            .route(
                "/memory/graph/relations",
                get(crate::api::graph::memory_graph_relations),
            )
            .route("/memory/graph/hops", post(memory_graph_hops))
            .route("/memory/decay", post(memory_decay))
            .route("/memory/consolidate", post(memory_consolidate))
            .route("/memory/reflect", post(memory_reflect))
            .route("/memory/quality", get(memory_quality))
            .route("/memory/evict", delete(memory_evict))
            .route("/memory/stats", get(memory_stats))
            .route("/agents/run", post(agents_run))
            .route("/bridge/import", post(bridge_import))
            .route("/v1/account/usage", get(account_usage))
            .route("/v1/account/limits", get(account_limits))
            .route("/code/scan", post(code_scan))
            .route("/code/find", post(code_find))
            .route("/code/stats", get(code_stats))
            .layer(Extension(workspace))
            .with_state(state)
    }

    #[tokio::test]
    async fn test_runtime_and_http_share_same_memory_instance() {
        let (_state, workspace) = test_state().await;

        assert!(Arc::ptr_eq(
            &workspace.workspace.memory,
            &workspace.workspace.runtime.memory()
        ));
    }

    #[tokio::test]
    async fn test_account_usage_reports_plan_limits() {
        let (state, workspace) = test_state().await;
        let app = test_router(state, workspace);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/v1/account/limits")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["plan"], "personal");
        assert_eq!(payload["storage_limit_bytes"], 10 * 1024 * 1024);
    }

    #[tokio::test]
    async fn test_build_info_exposes_version_and_provider_status() {
        let (state, workspace) = test_state().await;
        let app = test_router(state, workspace);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/build")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["service"], "xavier2");
        assert_eq!(payload["version"], env!("CARGO_PKG_VERSION"));
        assert!(payload.get("model_provider").is_some());
        assert_eq!(payload["memory_store"]["selected_backend"], "file");
        assert_eq!(payload["memory_store"]["backend"], "file");
        assert_eq!(payload["memory_store"]["rrf_k"], 60);
        assert_eq!(payload["memory_store"]["entity_extraction_enabled"], true);
        assert_eq!(payload["memory_store"]["qjl_threshold"], 30000);
        assert_eq!(payload["memory_store"]["audit_chain_enabled"], true);
        assert!(payload["memory_store"].get("migration_detail").is_some());
    }

    #[tokio::test]
    async fn test_build_info_exposes_selected_memory_backend() {
        let (state, workspace) =
            test_state_with_backend(crate::memory::surreal_store::MemoryBackend::Memory).await;
        let app = test_router(state, workspace);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/build")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["memory_store"]["selected_backend"], "memory");
        assert_eq!(payload["memory_store"]["backend"], "memory");
    }

    #[tokio::test]
    async fn test_readiness_reports_memory_store_and_code_graph_status() {
        let (state, workspace) = test_state().await;
        let app = test_router(state, workspace);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/readiness")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["status"].as_str(), Some("ok"));
        assert_eq!(payload["memory_store"]["ready"], true);
        assert_eq!(payload["code_graph"]["ready"], true);
    }

    #[tokio::test]
    async fn test_memory_add_and_query_share_same_memory() {
        let (state, workspace) = test_state().await;
        let app = test_router(state, workspace);

        let add_request = Request::builder()
            .method("POST")
            .uri("/memory/add")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "path": "shared-doc",
                    "content": "shared xavier2 memory document for system query",
                    "metadata": {"source": "http-test"}
                })
                .to_string(),
            ))
            .unwrap();

        let add_response = app.clone().oneshot(add_request).await.unwrap();
        assert_eq!(add_response.status(), StatusCode::OK);

        let query_request = Request::builder()
            .method("POST")
            .uri("/memory/query")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "query": "system query",
                    "limit": 5
                })
                .to_string(),
            ))
            .unwrap();

        let query_response = app.clone().oneshot(query_request).await.unwrap();
        assert_eq!(query_response.status(), StatusCode::OK);

        let query_body = to_bytes(query_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: QueryResponse = serde_json::from_slice(&query_body).unwrap();

        assert_eq!(payload.status, "ok");
        assert!(payload.confidence > 0.0);
        assert!(payload.response.contains("shared xavier2 memory document"));

        let search_request = Request::builder()
            .method("POST")
            .uri("/memory/search")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "query": "shared xavier2",
                    "limit": 5
                })
                .to_string(),
            ))
            .unwrap();

        let search_response = app.oneshot(search_request).await.unwrap();
        assert_eq!(search_response.status(), StatusCode::OK);

        let search_body = to_bytes(search_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: SearchResponse = serde_json::from_slice(&search_body).unwrap();

        assert_eq!(payload.status, "ok");
        assert_eq!(payload.results.len(), 1);
    }

    #[tokio::test]
    async fn test_memory_search_respects_kind_and_project_filters() {
        let (state, workspace) = test_state().await;
        let app = test_router(state, workspace);

        for payload in [
            serde_json::json!({
                "path": "repo/xavier2",
                "content": "Typed schema lives in src/memory/schema.rs",
                "kind": "repo",
                "namespace": {"project": "xavier2"},
                "provenance": {"file_path": "src/memory/schema.rs"}
            }),
            serde_json::json!({
                "path": "task/other",
                "content": "Connect OpenClaw and Engram",
                "kind": "task",
                "namespace": {"project": "xavier2"}
            }),
        ] {
            let request = Request::builder()
                .method("POST")
                .uri("/memory/add")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap();
            let response = app.clone().oneshot(request).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        }

        let search = Request::builder()
            .method("POST")
            .uri("/memory/search")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "query": "Where is the typed schema stored?",
                    "filters": {
                        "kinds": ["repo"],
                        "project": "xavier2"
                    }
                })
                .to_string(),
            ))
            .unwrap();

        let response = app.oneshot(search).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: SearchResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(payload.results.len(), 1);
        assert_eq!(payload.results[0]["path"], "repo/xavier2");
        assert_eq!(payload.results[0]["metadata"]["kind"], "repo");
        assert_eq!(
            payload.results[0]["metadata"]["namespace"]["project"],
            "xavier2"
        );
        assert_eq!(
            payload.results[0]["metadata"]["provenance"]["file_path"],
            "src/memory/schema.rs"
        );
    }

    #[tokio::test]
    async fn test_memory_hybrid_search_returns_scores_for_vec_backend() {
        let (state, workspace) =
            test_state_with_backend(crate::memory::surreal_store::MemoryBackend::Vec).await;
        let store = workspace.workspace.durable_store();
        let workspace_id = workspace.workspace_id.clone();
        let primary_id = crate::memory::surreal_store::stable_key(
            "memory",
            &[&workspace_id, "memory/account-renewal"],
        );
        let mut embedding = vec![0.0; 768];
        embedding[1] = 1.0;

        store
            .put(crate::memory::surreal_store::MemoryRecord {
                id: primary_id.clone(),
                workspace_id: workspace_id.clone(),
                path: "memory/account-renewal".to_string(),
                content: "Customer account ACCT-9F3A renewal approved by Alice Johnson."
                    .to_string(),
                metadata: serde_json::json!({}),
                embedding,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                revision: 1,
                primary: true,
                parent_id: None,
                revisions: Vec::new(),
            })
            .await
            .unwrap();

        store
            .save_beliefs(
                &workspace_id,
                vec![crate::memory::belief_graph::BeliefRelation {
                    id: ulid::Ulid::new().to_string(),
                    source: "ACCT-9F3A".to_string(),
                    target: "Alice Johnson".to_string(),
                    relation_type: "approved_by".to_string(),
                    weight: 0.9,
                    confidence: 0.9,
                    source_memory_id: Some(primary_id.clone()),
                    valid_from: None,
                    valid_until: None,
                    superseded_by: None,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                }],
            )
            .await
            .unwrap();

        let app = test_router(state, workspace);
        let request = Request::builder()
            .method("POST")
            .uri("/memory/hybrid-search")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "query": "ACCT-9F3A renewal",
                    "limit": 3,
                    "type": "both"
                })
                .to_string(),
            ))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(payload["status"], "ok");
        assert_eq!(payload["mode"], "both");
        assert_eq!(payload["results"][0]["id"], primary_id);
        assert!(payload["results"][0]["score"].as_f64().unwrap() > 0.0);
    }

    #[tokio::test]
    async fn test_memory_retrieve_respects_requested_limit() {
        let (state, workspace) = test_state().await;
        let app = test_router(state, workspace);

        for index in 0..25 {
            let request = Request::builder()
                .method("POST")
                .uri("/memory/add")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "path": format!("retrieve-doc-{index}"),
                        "content": format!("retrieve target memory document {index}"),
                        "metadata": {"source": "http-test"}
                    })
                    .to_string(),
                ))
                .unwrap();

            let response = app.clone().oneshot(request).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        }

        let retrieve_request = Request::builder()
            .method("POST")
            .uri("/memory/retrieve")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "query": "retrieve target",
                    "limit": 25,
                    "include_coherence": false
                })
                .to_string(),
            ))
            .unwrap();

        let retrieve_response = app.oneshot(retrieve_request).await.unwrap();
        assert_eq!(retrieve_response.status(), StatusCode::OK);

        let body = to_bytes(retrieve_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: MultiLayerRetrieveResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(payload.status, "ok");
        assert_eq!(payload.results.len(), 25);
        assert_eq!(payload.layers_used.working_count, 25);
        assert!(payload
            .results
            .iter()
            .all(|result| result.source_layer == "working"));
        assert!(payload
            .results
            .iter()
            .all(|result| result.path.starts_with("retrieve-doc-")));
    }

    #[tokio::test]
    async fn test_memory_graph_hops_endpoint_exists_for_vec_backend() {
        let (state, workspace) =
            test_state_with_backend(crate::memory::surreal_store::MemoryBackend::Vec).await;
        let workspace_id = workspace.workspace_id.clone();
        let source_id =
            crate::memory::surreal_store::stable_key("memory", &[&workspace_id, "memory/root"]);

        workspace
            .workspace
            .durable_store()
            .put(crate::memory::surreal_store::MemoryRecord {
                id: source_id,
                workspace_id: workspace_id.clone(),
                path: "memory/root".to_string(),
                content: "ACCT-9F3A was approved by Alice Johnson".to_string(),
                metadata: serde_json::json!({}),
                embedding: vec![0.0; 768],
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                revision: 1,
                primary: true,
                parent_id: None,
                revisions: Vec::new(),
            })
            .await
            .unwrap();

        let app = test_router(state, workspace);
        let request = Request::builder()
            .method("POST")
            .uri("/memory/graph/hops")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "path": "memory/root",
                    "hops": 2,
                    "query": "ACCT-9F3A"
                })
                .to_string(),
            ))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_entity_graph_endpoints_index_memory_additions() {
        let (state, workspace) = test_state().await;
        let app = test_router(state, workspace);

        let add_request = Request::builder()
            .method("POST")
            .uri("/memory/add")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "path": "graph-doc",
                    "content": "BELA works at SWAL and knows Leonardo in Bogota.",
                    "metadata": {"source": "http-test"}
                })
                .to_string(),
            ))
            .unwrap();
        let add_response = app.clone().oneshot(add_request).await.unwrap();
        assert_eq!(add_response.status(), StatusCode::OK);

        let entity_request = Request::builder()
            .method("GET")
            .uri("/memory/graph/entity/BELA?max_depth=2&direction=both")
            .body(Body::empty())
            .unwrap();
        let entity_response = app.clone().oneshot(entity_request).await.unwrap();
        assert_eq!(entity_response.status(), StatusCode::OK);
        let entity_body = to_bytes(entity_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let entity_payload: serde_json::Value = serde_json::from_slice(&entity_body).unwrap();
        assert_eq!(entity_payload["status"], "ok");
        assert!(entity_payload["entity"]["name"].as_str().is_some());

        let relations_request = Request::builder()
            .method("GET")
            .uri("/memory/graph/relations?entity_id=BELA&max_depth=2&direction=both")
            .body(Body::empty())
            .unwrap();
        let relations_response = app.oneshot(relations_request).await.unwrap();
        assert_eq!(relations_response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_bridge_import_openclaw_markdown_supports_filterable_recall() {
        let (state, workspace) = test_state().await;
        let app = test_router(state, workspace);

        let root = unique_test_path("xavier2-openclaw-bridge", "fixtures");
        let file = root.join("memory").join("decision.md");
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(
            &file,
            r#"---
title: "Bridge decision"
date: "2026-03-28T10:00:00Z"
memory_type: "decision"
projects: ["xavier2"]
---
Use token auth for local bridge workflows.
"#,
        )
        .unwrap();

        let import_request = Request::builder()
            .method("POST")
            .uri("/bridge/import")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "source": "openclaw_markdown",
                    "path": root,
                    "scope": "ops"
                })
                .to_string(),
            ))
            .unwrap();

        let import_response = app.clone().oneshot(import_request).await.unwrap();
        assert_eq!(import_response.status(), StatusCode::OK);
        let import_body = to_bytes(import_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let import_payload: serde_json::Value = serde_json::from_slice(&import_body).unwrap();
        assert_eq!(import_payload["status"], "ok");
        assert_eq!(import_payload["source"], "openclaw_markdown");
        assert_eq!(import_payload["imported"], 1);
        assert_eq!(import_payload["skipped"], 0);

        let search_request = Request::builder()
            .method("POST")
            .uri("/memory/search")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "query": "token auth bridge workflows",
                    "filters": {
                        "kinds": ["decision"],
                        "project": "xavier2",
                        "scope": "ops",
                        "source_app": "openclaw"
                    }
                })
                .to_string(),
            ))
            .unwrap();

        let search_response = app.oneshot(search_request).await.unwrap();
        assert_eq!(search_response.status(), StatusCode::OK);
        let search_body = to_bytes(search_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: SearchResponse = serde_json::from_slice(&search_body).unwrap();

        assert_eq!(payload.results.len(), 1);
        assert_eq!(
            payload.results[0]["path"],
            "bridge/openclaw/memory/decision.md"
        );
        assert_eq!(payload.results[0]["metadata"]["kind"], "decision");
        assert_eq!(
            payload.results[0]["metadata"]["namespace"]["project"],
            "xavier2"
        );
        assert_eq!(payload.results[0]["metadata"]["namespace"]["scope"], "ops");
        assert_eq!(
            payload.results[0]["metadata"]["provenance"]["source_app"],
            "openclaw"
        );
        assert_eq!(
            payload.results[0]["metadata"]["provenance"]["file_path"],
            "memory/decision.md"
        );
    }

    #[tokio::test]
    async fn test_bridge_import_engram_export_supports_agents_run_without_system3() {
        let (state, workspace) = test_state().await;
        let app = test_router(state, workspace);

        let export_path = unique_test_path("xavier2-engram-bridge", "export.json");
        fs::write(
            &export_path,
            serde_json::json!({
                "sessions": [{
                    "id": "session-bridge-1",
                    "project": "xavier2",
                    "directory": "E:/scripts-python/xavier2",
                    "started_at": "2026-03-28T18:00:00Z",
                    "summary": "Imported bridge memories"
                }],
                "observations": [{
                    "id": 7,
                    "session_id": "session-bridge-1",
                    "project": "xavier2",
                    "type": "decision",
                    "title": "Bridge rollout decision",
                    "content": "Record typed provenance before any synthesis layer.",
                    "topic_key": "architecture/typed-bridge",
                    "created_at": "2026-03-28T18:05:00Z"
                }]
            })
            .to_string(),
        )
        .unwrap();

        let import_request = Request::builder()
            .method("POST")
            .uri("/bridge/import")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "source": "engram_export",
                    "path": export_path,
                    "project": "xavier2"
                })
                .to_string(),
            ))
            .unwrap();

        let import_response = app.clone().oneshot(import_request).await.unwrap();
        assert_eq!(import_response.status(), StatusCode::OK);
        let import_body = to_bytes(import_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let import_payload: serde_json::Value = serde_json::from_slice(&import_body).unwrap();
        assert_eq!(import_payload["status"], "ok");
        assert_eq!(import_payload["source"], "engram_export");
        assert_eq!(import_payload["imported"], 2);

        let run_request = Request::builder()
            .method("POST")
            .uri("/agents/run")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "query": "What was the bridge rollout decision?",
                    "filters": {
                        "project": "xavier2",
                        "session_id": "session-bridge-1",
                        "source_app": "engram",
                        "topic_key": "architecture/typed-bridge"
                    },
                    "system3_mode": "disabled"
                })
                .to_string(),
            ))
            .unwrap();

        let run_response = app.oneshot(run_request).await.unwrap();
        assert_eq!(run_response.status(), StatusCode::OK);
        let run_body = to_bytes(run_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&run_body).unwrap();

        assert_eq!(payload["status"], "ok");
        assert!(payload["confidence"].as_f64().unwrap_or_default() > 0.0);
        assert!(payload["response"]
            .as_str()
            .unwrap_or_default()
            .to_lowercase()
            .contains("typed provenance before any synthesis layer"));
    }

    #[tokio::test]
    async fn test_memory_delete_removes_document_from_shared_memory() {
        let (state, workspace) = test_state().await;
        let app = test_router(state, workspace);

        let add_request = Request::builder()
            .method("POST")
            .uri("/memory/add")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "path": "delete-doc",
                    "content": "document to delete from xavier2 memory",
                    "metadata": {"source": "http-test"}
                })
                .to_string(),
            ))
            .unwrap();
        let add_response = app.clone().oneshot(add_request).await.unwrap();
        assert_eq!(add_response.status(), StatusCode::OK);

        let delete_request = Request::builder()
            .method("POST")
            .uri("/memory/delete")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "path": "delete-doc"
                })
                .to_string(),
            ))
            .unwrap();
        let delete_response = app.clone().oneshot(delete_request).await.unwrap();
        assert_eq!(delete_response.status(), StatusCode::OK);

        let delete_body = to_bytes(delete_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: DeleteMemoryResponse = serde_json::from_slice(&delete_body).unwrap();
        assert_eq!(payload.status, "ok");
        assert!(payload.deleted);
        assert_eq!(payload.path.as_deref(), Some("delete-doc"));

        let search_request = Request::builder()
            .method("POST")
            .uri("/memory/search")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "query": "delete from xavier2",
                    "limit": 5
                })
                .to_string(),
            ))
            .unwrap();
        let search_response = app.oneshot(search_request).await.unwrap();
        assert_eq!(search_response.status(), StatusCode::OK);

        let search_body = to_bytes(search_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: SearchResponse = serde_json::from_slice(&search_body).unwrap();
        assert!(payload.results.is_empty());
    }

    #[tokio::test]
    async fn test_memory_reset_clears_shared_memory() {
        let (state, workspace) = test_state().await;
        let app = test_router(state, workspace);

        for path in ["reset-doc-1", "reset-doc-2"] {
            let add_request = Request::builder()
                .method("POST")
                .uri("/memory/add")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "path": path,
                        "content": format!("{path} content"),
                        "metadata": {"source": "http-test"}
                    })
                    .to_string(),
                ))
                .unwrap();
            let add_response = app.clone().oneshot(add_request).await.unwrap();
            assert_eq!(add_response.status(), StatusCode::OK);
        }

        let reset_request = Request::builder()
            .method("POST")
            .uri("/memory/reset")
            .body(Body::empty())
            .unwrap();
        let reset_response = app.clone().oneshot(reset_request).await.unwrap();
        assert_eq!(reset_response.status(), StatusCode::OK);

        let reset_body = to_bytes(reset_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: ResetMemoryResponse = serde_json::from_slice(&reset_body).unwrap();
        assert_eq!(payload.status, "ok");
        assert_eq!(payload.removed, 2);

        let search_request = Request::builder()
            .method("POST")
            .uri("/memory/search")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "query": "content",
                    "limit": 10
                })
                .to_string(),
            ))
            .unwrap();
        let search_response = app.oneshot(search_request).await.unwrap();
        assert_eq!(search_response.status(), StatusCode::OK);

        let search_body = to_bytes(search_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: SearchResponse = serde_json::from_slice(&search_body).unwrap();
        assert!(payload.results.is_empty());
    }

    #[tokio::test]
    async fn test_code_scan_and_find_index_symbols() {
        let (state, workspace) = test_state().await;
        let app = test_router(state, workspace);
        let project_dir = unique_test_path("xavier2-code-project", "src");
        std::fs::create_dir_all(&project_dir).unwrap();
        let source_path = project_dir.join("main.rs");
        std::fs::write(
            &source_path,
            "struct TestStruct;\nfn test_handler() -> TestStruct { TestStruct }\n",
        )
        .unwrap();

        let scan_request = Request::builder()
            .method("POST")
            .uri("/code/scan")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "path": project_dir.to_string_lossy()
                })
                .to_string(),
            ))
            .unwrap();
        let scan_response = app.clone().oneshot(scan_request).await.unwrap();
        assert_eq!(scan_response.status(), StatusCode::OK);

        let scan_body = to_bytes(scan_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let scan_payload: CodeScanResponse = serde_json::from_slice(&scan_body).unwrap();
        assert_eq!(scan_payload.status, "ok");
        assert!(scan_payload.indexed_files >= 1);
        assert!(scan_payload.indexed_chunks >= 2);

        let find_request = Request::builder()
            .method("POST")
            .uri("/code/find")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "query": "test_handler",
                    "limit": 10
                })
                .to_string(),
            ))
            .unwrap();
        let find_response = app.clone().oneshot(find_request).await.unwrap();
        assert_eq!(find_response.status(), StatusCode::OK);

        let find_body = to_bytes(find_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let find_payload: CodeFindResponse = serde_json::from_slice(&find_body).unwrap();
        assert_eq!(find_payload.status, "ok");
        assert!(find_payload
            .results
            .iter()
            .any(|result| result.symbol == "test_handler"));
    }

    #[tokio::test]
    async fn test_code_stats_reports_indexed_files() {
        let (state, workspace) = test_state().await;
        let app = test_router(state, workspace);
        let project_dir = unique_test_path("xavier2-code-stats", "src");
        std::fs::create_dir_all(&project_dir).unwrap();
        std::fs::write(project_dir.join("lib.rs"), "fn stats_target() {}\n").unwrap();

        let scan_request = Request::builder()
            .method("POST")
            .uri("/code/scan")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "path": project_dir.to_string_lossy()
                })
                .to_string(),
            ))
            .unwrap();
        let scan_response = app.clone().oneshot(scan_request).await.unwrap();
        assert_eq!(scan_response.status(), StatusCode::OK);

        let stats_request = Request::builder()
            .method("GET")
            .uri("/code/stats")
            .body(Body::empty())
            .unwrap();
        let stats_response = app.oneshot(stats_request).await.unwrap();
        assert_eq!(stats_response.status(), StatusCode::OK);

        let stats_body = to_bytes(stats_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let stats_payload: CodeStatsResponse = serde_json::from_slice(&stats_body).unwrap();
        assert_eq!(stats_payload.status, "ok");
        assert!(stats_payload.total_files >= 1);
        assert!(stats_payload.total_chunks >= 1);
    }

    #[tokio::test]
    async fn test_memory_consolidate_merges_duplicate_memories() {
        let (state, workspace) = test_state().await;
        let app = test_router(state, workspace.clone());

        for path in ["memory/phase4/a", "memory/phase4/b"] {
            let add_request = Request::builder()
                .method("POST")
                .uri("/memory/add")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "content": "Phase four duplicate memory about consolidation and reflection.",
                        "path": path,
                        "metadata": {"memory_priority": "low"}
                    })
                    .to_string(),
                ))
                .unwrap();
            let response = app.clone().oneshot(add_request).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        }

        let consolidate_request = Request::builder()
            .method("POST")
            .uri("/memory/consolidate")
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(consolidate_request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["status"].as_str(), Some("ok"));

        let docs = workspace.workspace.memory.all_documents().await;
        let matching = docs
            .iter()
            .filter(|doc| doc.content.contains("Phase four duplicate memory"))
            .count();
        assert_eq!(matching, 1);
    }

    #[tokio::test]
    async fn test_memory_reflect_creates_summary_memory() {
        let (state, workspace) = test_state().await;
        let app = test_router(state, workspace.clone());

        for (idx, content) in [
            "We should merge similar memories to remove redundant information.",
            "Consolidation should remove duplicate memory content and keep one summary.",
        ]
        .iter()
        .enumerate()
        {
            let add_request = Request::builder()
                .method("POST")
                .uri("/memory/add")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "content": content,
                        "path": format!("memory/reflection/source-{}", idx),
                        "metadata": {"memory_priority": "medium"}
                    })
                    .to_string(),
                ))
                .unwrap();
            let response = app.clone().oneshot(add_request).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        }

        let reflect_request = Request::builder()
            .method("POST")
            .uri("/memory/reflect")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(reflect_request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["status"], "ok");

        let docs = workspace.workspace.memory.all_documents().await;
        assert!(docs.iter().any(|doc| {
            doc.metadata
                .get("memory_reflection")
                .and_then(|value| value.as_bool())
                == Some(true)
        }));
    }
}
