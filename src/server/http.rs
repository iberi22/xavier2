//! HTTP handlers for the minimal Xavier vertical slice.

use axum::{
    extract::{ws::Message, ws::WebSocket, State, WebSocketUpgrade},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{error, info};

use crate::{
    agents::provider::ModelProviderClient,
    agents::runtime::System3Mode,
    consistency::regularization::{CoherenceReport, RetentionRegularizer},
    consolidation::ConsolidationTask,
    embedding,
    domain::memory::belief::{BeliefNode, BeliefEdge},
    memory::entity_graph::EntityRecord,
    memory::qmd_memory::MemoryDocument,
    memory::schema::{MemoryQueryFilters, TypedMemoryPayload},
    memory::sqlite_vec_store::VecSqliteMemoryStore,
    memory::store::{GraphHopResult, HybridSearchMode},
    retrieval::gating::{AdaptiveGating, LayerWeights, SessionSummary},
    server::events::{WsEvent, WsMessage},
    utils::crypto::sha256_hex,
    workspace::WorkspaceContext,
    AppState,
};

// ============================================================
// Graceful Shutdown Infrastructure
// ============================================================

/// Global shutdown tracker shared across the application.
/// All components that need to cooperate with graceful shutdown
/// (the HTTP server, background tasks, etc.) share this state.
#[derive(Clone)]
pub struct ShutdownState {
    /// Signal received — server should stop accepting new connections.
    pub shutdown_signalled: Arc<AtomicU64>,
    /// Broadcast channel for notifying all subsystems.
    /// All components that hold a Sender can signal shutdown.
    shutdown_tx: Arc<broadcast::Sender<()>>,
}

impl ShutdownState {
    pub fn new() -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        Self {
            shutdown_signalled: Arc::new(AtomicU64::new(0)),
            shutdown_tx: Arc::new(shutdown_tx),
        }
    }

    /// Request graceful shutdown. Idempotent — multiple calls are fine.
    pub fn request_shutdown(&self, reason: &'static str) {
        let prev = self.shutdown_signalled.fetch_add(1, Ordering::SeqCst);
        if prev == 0 {
            info!("Shutdown requested: {}", reason);
            // Ignore send error — receivers may have already dropped.
            let _ = self.shutdown_tx.send(());
        }
    }

    /// Returns true if shutdown has been requested.
    pub fn is_shutdown_requested(&self) -> bool {
        self.shutdown_signalled.load(Ordering::SeqCst) > 0
    }

    /// Subscribe to shutdown signals. The returned Receiver fires
    /// when shutdown is requested.
    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.shutdown_tx.subscribe()
    }

    /// Number of seconds since shutdown was signalled.
    /// Returns 0 if not yet signalled.
    pub fn seconds_since_shutdown(&self) -> u64 {
        let val = self.shutdown_signalled.load(Ordering::SeqCst);
        if val == 0 {
            0
        } else {
            // We store the timestamp in the atomic as seconds-since-epoch.
            val
        }
    }
}

// ============================================================
// Real-time Event Streaming (WebSocket)
// ============================================================

#[derive(Debug, Default, Clone)]
struct WsSubscriptions {
    agent_ids: std::collections::HashSet<String>,
    project_ids: std::collections::HashSet<String>,
    event_types: std::collections::HashSet<String>,
}

impl WsSubscriptions {
    fn matches(&self, event: &crate::server::events::RealtimeEvent) -> bool {
        // If no subscriptions, match nothing
        if self.agent_ids.is_empty() && self.project_ids.is_empty() && self.event_types.is_empty() {
            return false;
        }

        if !self.agent_ids.is_empty() && !self.agent_ids.contains(&event.agent_id) {
            return false;
        }

        if !self.project_ids.is_empty() {
            match &event.project_id {
                Some(p) if self.project_ids.contains(p) => {}
                _ => return false,
            }
        }

        if !self.event_types.is_empty() && !self.event_types.contains(&event.event_type) {
            return false;
        }

        true
    }
}

pub async fn ws_events_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let rx = state
        .workspace_registry
        .default_context_sync()
        .and_then(|ctx| ctx.workspace.event_tx_channel().map(|tx| tx.subscribe()));

    ws.on_upgrade(move |socket| handle_ws_socket(socket, rx))
}

async fn handle_ws_socket(
    mut socket: WebSocket,
    mut event_rx: Option<broadcast::Receiver<crate::server::events::RealtimeEvent>>,
) {
    let mut subscriptions = WsSubscriptions::default();

    loop {
        tokio::select! {
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                            match ws_msg {
                                WsMessage::Subscribe { agent_id, project_id, event_type } => {
                                    if let Some(id) = agent_id { subscriptions.agent_ids.insert(id); }
                                    if let Some(id) = project_id { subscriptions.project_ids.insert(id); }
                                    if let Some(id) = event_type { subscriptions.event_types.insert(id); }

                                    let _ = socket.send(Message::Text(
                                        serde_json::to_string(&WsEvent::SubscriptionConfirmed).unwrap_or_default().into()
                                    )).await;
                                }
                                WsMessage::Unsubscribe { agent_id, project_id, event_type } => {
                                    if let Some(id) = agent_id { subscriptions.agent_ids.remove(&id); }
                                    if let Some(id) = project_id { subscriptions.project_ids.remove(&id); }
                                    if let Some(id) = event_type { subscriptions.event_types.remove(&id); }

                                    let _ = socket.send(Message::Text(
                                        serde_json::to_string(&WsEvent::SubscriptionConfirmed).unwrap_or_default().into()
                                    )).await;
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
            event_res = async {
                if let Some(rx) = &mut event_rx {
                    rx.recv().await.ok()
                } else {
                    None
                }
            } => {
                if let Some(event) = event_res {
                    if subscriptions.matches(&event) {
                        let _ = socket.send(Message::Text(
                            serde_json::to_string(&WsEvent::Event(event)).unwrap_or_default().into()
                        )).await;
                    }
                }
            }
        }
    }
}

impl Default for ShutdownState {
    fn default() -> Self {
        Self::new()
    }
}

/// Starts a background task that listens for OS signals (SIGTERM, SIGINT)
/// and translates them into graceful shutdown requests.
/// On Windows this watches Ctrl+C / console events.
/// Returns a handle to the task; dropping the handle does NOT cancel the task.
pub async fn start_signal_handler(state: ShutdownState) {
    tokio::spawn(async move {
        #[cfg(windows)]
        use tokio::signal::windows::ctrl_c;

        // Unix signals (SIGTERM / SIGINT)
        #[cfg(unix)]
        {
            use tokio::signal::unix::{signal, SignalKind};
            let mut sigterm = match signal(SignalKind::terminate()) {
                Ok(s) => s,
                Err(e) => {
                    error!("Failed to register SIGTERM handler: {}", e);
                    return;
                }
            };
            let mut sigint = match signal(SignalKind::interrupt()) {
                Ok(s) => s,
                Err(e) => {
                    error!("Failed to register SIGINT handler: {}", e);
                    return;
                }
            };

            tokio::select! {
                _ = sigterm.recv() => state.request_shutdown("SIGTERM"),
                _ = sigint.recv() => state.request_shutdown("SIGINT"),
            }
        }

        // Windows console events
        #[cfg(windows)]
        {
            let ctrl_events = async {
                match ctrl_c() {
                    Ok(mut rx) => rx.recv().await,
                    Err(error) => {
                        error!(%error, "Failed to register Ctrl+C handler");
                        None
                    }
                }
            };

            let reason = ctrl_events.await;
            if let Some(()) = reason {
                state.request_shutdown("Ctrl+C / console close");
            }
        }
    });
}

// ============================================================
// Global Panic Hook
// ============================================================

/// Installs a global panic hook that logs panics with stack traces
/// before the process exits. This ensures panics are never silently
/// swallowed by tokio's default handler.
pub fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        // Log to stderr so it appears in logs even if stdout is redirected.
        let msg = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic payload".to_string()
        };

        let location = panic_info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown location".to_string());

        // Format thread name if available
        let thread = std::thread::current();
        let thread_name = thread.name().map(|n| format!("[{n}] ")).unwrap_or_default();

        eprintln!(
            "--------------------------------------------------\n  PANIC in Xavier ({})\n  Thread: {}\n  Location: {}\n--------------------------------------------------\n",
            chrono::Utc::now().to_rfc3339(),
            thread_name,
            location
        );

        // Also emit a structured log event so aggregators catch it.
        error!(
            panic_message = %msg,
            panic_location = %location,
            thread_name = %thread.name().unwrap_or("unknown"),
            "xavier_panic"
        );

        // Call the original hook (default prints to stderr) so we don't
        // suppress the standard panic output.
        default_hook(panic_info);
    }));
}

// ============================================================
// HTTP Config & Server
// ============================================================

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
    config: HttpConfig,
}

impl HttpServer {
    pub fn new(config: HttpConfig) -> Self {
        Self { config }
    }

    pub async fn serve(&self) {
        tracing::warn!("HttpServer::serve() is a stub - does not actually start a server");
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}

// ============================================================
// Request/Response Types
// ============================================================

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
    #[serde(default = "default_weight")]
    pub keyword_weight: f32,
    #[serde(default = "default_weight")]
    pub vector_weight: f32,
}

fn default_weight() -> f32 {
    0.5
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
    #[serde(default)]
    pub cluster_id: Option<String>,
    #[serde(default)]
    pub level: Option<crate::memory::schema::MemoryLevel>,
    #[serde(default)]
    pub relation: Option<crate::memory::schema::RelationKind>,
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
    0.5
}

fn default_rrf_k() -> u32 {
    crate::search::hybrid::configured_rrf_k()
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
    pub edges: Vec<BeliefEdge>,
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

// ============================================================
// Health & Readiness Endpoints
// ============================================================

/// Basic liveness probe — returns 200 if the process is alive.
/// Does NOT check dependencies. Fast and cheap; suitable for
/// Kubernetes liveness probes.
pub async fn health() -> impl IntoResponse {
    const HEALTH_JSON: &str = concat!(
        "{\"status\":\"ok\",\"service\":\"xavier\",\"version\":\"",
        env!("CARGO_PKG_VERSION"),
        "\"}"
    );
    match axum::response::Response::builder()
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(HEALTH_JSON))
    {
        Ok(response) => response,
        Err(error) => {
            error!(%error, "failed to build health response");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "status": "error",
                    "message": "failed to build health response"
                })),
            )
                .into_response()
        }
    }
}

/// Detailed liveness + readiness probe for orchestration systems.
/// Checks workspace, memory store, code graph, embeddings, and LLM.
/// Suitable for Kubernetes readiness probes and load balancer health checks.
pub async fn readiness(State(state): State<AppState>) -> impl IntoResponse {
    let workspace_context = state.workspace_registry.default_context().await;
    let workspace_ready = workspace_context.is_some();
    let embedding_configured = crate::memory::embedder::EmbeddingClient::is_configured_from_env();
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

    let code_graph = ReadinessComponent {
        configured: false,
        ready: true,
        detail: "code graph not available in CLI mode".to_string(),
    };

    let ready = workspace.ready
        && memory_store.ready
        && code_graph.ready
        && (!embeddings.configured || embeddings.ready)
        && (!llm.configured || llm.ready);

    Json(ReadinessResponse {
        status: if ready { "ok" } else { "degraded" }.to_string(),
        service: "xavier".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        workspace,
        memory_store,
        code_graph,
        embeddings,
        llm,
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

// ============================================================
// Build Info
// ============================================================

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
    pub xavier_log_level: Option<String>,
    pub model_provider: crate::agents::provider::ModelProviderStatus,
    pub memory_store: MemoryStoreBuildInfo,
}

pub async fn build_info(State(state): State<AppState>) -> impl IntoResponse {
    let workspace = state.workspace_registry.default_context().await;

    Json(BuildInfoResponse {
        service: "xavier".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        rust_log: std::env::var("RUST_LOG").ok(),
        xavier_log_level: std::env::var("XAVIER_LOG_LEVEL").ok(),
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
                selected_backend: std::env::var("XAVIER_MEMORY_BACKEND")
                    .map(|value| {
                        crate::memory::store::MemoryBackend::from_env(&value)
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

// ============================================================
// Memory Endpoints
// ============================================================

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
        cluster_id: payload.cluster_id,
        level: payload.level,
        relation: payload.relation,
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

    match workspace
        .workspace
        .ingest_typed(
            path,
            content.clone(),
            metadata.clone(),
            Some(typed),
            content_vector,
            false,
        )
        .await
    {
        Ok(_) => {}
        Err(error) => {
            tracing::error!(%error, workspace_id = %workspace.workspace_id, "failed to add memory document");
            return Json(serde_json::json!({
                "status": "error",
                "message": format!("failed to add memory: {}", error),
                "workspace_id": workspace.workspace_id,
            }));
        }
    };

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

pub async fn memory_retrieve(
    Extension(workspace): Extension<WorkspaceContext>,
    Json(payload): Json<MultiLayerRetrieveRequest>,
) -> impl IntoResponse {
    info!(
        query_fingerprint = %query_fingerprint(&payload.query),
        limit = payload.limit,
        "memory_retrieve"
    );

    Json(build_multi_layer_retrieve_response(&workspace, &payload).await)
}

async fn build_multi_layer_retrieve_response(
    workspace: &WorkspaceContext,
    payload: &MultiLayerRetrieveRequest,
) -> MultiLayerRetrieveResponse {
    let weights = payload.layer_weights.unwrap_or_default();

    let gating = AdaptiveGating::new(crate::retrieval::gating::GatingConfig {
        layer_weights: weights,
        relevance_threshold: payload.relevance_threshold.clamp(0.0, 1.0),
        rrf_k: payload.rrf_k,
        max_results: payload.limit.max(1),
    });

    let working_docs = workspace.workspace.memory.all_documents().await;
    let working_count = working_docs.len();

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

    let semantic_entities: Vec<EntityRecord> =
        workspace.workspace.entity_graph.all_entities().await;
    let semantic_count = semantic_entities.len();

    let results = gating.retrieve(
        &working_docs,
        &episodic_summaries,
        &semantic_entities,
        &payload.query,
    );

    let total_results = results.len();

    let coherence_report = if payload.include_coherence {
        let regularizer = RetentionRegularizer::with_defaults();
        Some(regularizer.check_coherence_with_entities(&working_docs, &semantic_entities))
    } else {
        None
    };

    let retrieved: Vec<RetrievedMemory> = results
        .into_iter()
        .take(payload.limit)
        .map(|r| {
            let crate::search::rrf::ScoredResult {
                id,
                content,
                score,
                source,
                path: _,
                updated_at: _,
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

    MultiLayerRetrieveResponse {
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
    }
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

pub async fn memory_reflect(
    Extension(workspace): Extension<WorkspaceContext>,
) -> impl IntoResponse {
    info!("?? Memory reflection request");
    let task = ConsolidationTask::default();
    match task.reflect(&workspace).await {
        Ok(result) => Json(serde_json::json!({
            "status": "ok",
            "data": result,
        })),
        Err(e) => {
            error!("Memory reflect error: {:?}", e);
            Json(serde_json::json!({
                "status": "error",
                "error": e.to_string(),
            }))
        }
    }
}
