//! HTTP server and WebSocket handlers

use anyhow::{anyhow, Result};
use axum::{
    body::Body,
    extract::{Path as AxumPath, State},
    http::{HeaderMap, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::cli::config::{
    code_graph_db_path, resolve_base_url, resolve_base_url_for_port, resolve_http_bind_host,
    resolve_http_port, resolve_http_token, state_panel_root, xavier_token,
};
use xavier::agents::rate_limit::RateLimitManager;
use xavier::agents::system3::{System3Actor, ActorConfig};
use crate::cli::state::CliState;
use super::code_graph::{
    code_find_symbols, filter_symbols_by_query,
};
use super::security::{
    secure_cli_input, secure_external_input, secure_optional_request_field,
};
use super::utils::estimate_tokens;
use xavier::adapters::inbound::http::routes::{
    sync_check_handler, time_metric_handler, verify_save_handler,
};
use xavier::adapters::outbound::http_health_adapter::HttpHealthAdapter;
use xavier::app::qmd_memory_adapter::QmdMemoryAdapter;
use xavier::coordination::SimpleAgentRegistry;
use xavier::memory::qmd_memory::{MemoryDocument, QmdMemory};
use xavier::memory::schema::MemoryQueryFilters;
use xavier::memory::session_store::{PanelMessage, SessionStore};
use xavier::memory::sqlite_vec_store::VecSqliteMemoryStore;
use xavier::memory::store::{MemoryRecord, MemoryStore};
use xavier::ports::inbound::{AgentLifecyclePort, MemoryQueryPort, TimeMetricsPort};
use xavier::ports::outbound::HealthCheckPort;
use xavier::security::SecurityService;
use xavier::server::panel::{
    panel_asset, panel_index, CreateThreadRequest, PanelChatRequest, PanelChatResponse,
};
use xavier::session::event_mapper::PanelThreadEntry;
use xavier::session::types::SessionEvent;
use xavier::tasks::session_sync_task::SessionSyncTask;
use xavier::tasks::store::{TaskService, InMemoryTaskStore};
use xavier::time::TimeMetricsStore;
use xavier::coordination::{KeyLendingEngine, XavierEventBus};

pub async fn start_http_server(port: u16) -> Result<()> {
    std::env::set_var("XAVIER_PORT", port.to_string());

    // Set default router model assignments (user can override via env)
    if std::env::var("XAVIER_ROUTER_FAST_MODEL").is_err() {
        std::env::set_var("XAVIER_ROUTER_FAST_MODEL", "opencode/minimax-m2.7");
    }
    if std::env::var("XAVIER_ROUTER_QUALITY_MODEL").is_err() {
        std::env::set_var("XAVIER_ROUTER_QUALITY_MODEL", "opencode/deepseek-v4-pro");
    }
    if std::env::var("XAVIER_ROUTER_RETRIEVED_MODEL").is_err() {
        std::env::set_var("XAVIER_ROUTER_RETRIEVED_MODEL", "opencode/minimax-m2.7");
    }
    if std::env::var("XAVIER_ROUTER_COMPLEX_MODEL").is_err() {
        std::env::set_var("XAVIER_ROUTER_COMPLEX_MODEL", "opencode/deepseek-v4-pro");
    }

    let bind_host = resolve_http_bind_host();
    let bind_addr = format!("{}:{}", bind_host, port);
    info!("Starting Xavier HTTP server on {}", bind_addr);
    let token = resolve_http_token()?;
    std::env::set_var("XAVIER_TOKEN", &token);

    // Initialize the memory store
    let mut store_inner = VecSqliteMemoryStore::from_env().await?;
    let (event_tx, _) = tokio::sync::broadcast::channel(100);
    store_inner.set_event_tx(event_tx);
    let store = Arc::new(store_inner);
    let workspace_id =
        std::env::var("XAVIER_DEFAULT_WORKSPACE_ID").unwrap_or_else(|_| "default".to_string());
    let durable_state = store.load_workspace_state(&workspace_id).await?;
    let docs = Arc::new(RwLock::new(
        durable_state
            .memories
            .iter()
            .map(MemoryRecord::to_document)
            .collect::<Vec<MemoryDocument>>(),
    ));
    let memory = Arc::new(QmdMemory::new_with_workspace(docs, workspace_id.clone()));
    let dyn_store: Arc<dyn MemoryStore> = store.clone();
    memory.set_store(dyn_store.clone()).await;
    memory.init().await?;
    let memory_port =
        Arc::new(QmdMemoryAdapter::new(Arc::clone(&memory))) as Arc<dyn MemoryQueryPort>;

    // Initialize TimeMetricsStore using the same SQLite connection
    let time_conn = store.clone_inner_conn();
    let time_store = Arc::new(TimeMetricsStore::new(time_conn));
    // Init schema (table created if not exists)
    {
        let conn = time_store.conn.lock();
        if let Err(e) = TimeMetricsStore::init_schema(&conn) {
            info!("TimeMetricsStore schema init warning: {}", e);
        }
    }
    // Register global time metrics port for HTTP handler (wrap in adapter)
    use xavier::adapters::inbound::http::routes::{init_health_port, init_time_store};
    use xavier::adapters::inbound::http::time_metrics_adapter::TimeMetricsAdapter;
    let health_adapter = Arc::new(HttpHealthAdapter::new(resolve_base_url_for_port(port)))
        as Arc<dyn HealthCheckPort>;
    let time_adapter =
        Arc::new(TimeMetricsAdapter::new(Arc::clone(&time_store))) as Arc<dyn TimeMetricsPort>;
    init_time_store(time_adapter);
    init_health_port(health_adapter.clone());

    let code_db_path = code_graph_db_path();
    if let Some(parent) = code_db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let code_db = Arc::new(::code_graph::db::CodeGraphDB::new(&code_db_path)?);
    let code_indexer = Arc::new(::code_graph::indexer::Indexer::new(Arc::clone(&code_db)));
    let code_query = Arc::new(::code_graph::query::QueryEngine::new(Arc::clone(&code_db)));

    // Determine workspace root for path traversal protection
    let workspace_dir = std::path::absolute(
        std::env::var("XAVIER_WORKSPACE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))),
    )
    .unwrap_or_else(|_| PathBuf::from("."));
    info!(
        "Workspace root for path security: {}",
        workspace_dir.display()
    );

    let panel_root = state_panel_root(&workspace_dir, &workspace_id);
    let panel_store = Arc::new(SessionStore::new(panel_root).await?);

    let _audit_logger = Arc::new(xavier::secrets::audit::QmdAuditLogger::new(store.clone_inner_conn()));
    {
        let conn = store.clone_inner_conn();
        let conn_lock = conn.lock();
        xavier::secrets::audit::QmdAuditLogger::init_schema(&conn_lock)?;
    }

    let rate_manager = Arc::new(RateLimitManager::new(store.clone_inner_conn()));
    {
        let conn = store.clone_inner_conn();
        let conn_lock = conn.lock();
        RateLimitManager::init_schema(&conn_lock)?;
    }

    let secrets_engine = Arc::new(KeyLendingEngine::new(Box::new(xavier::secrets::audit::QmdAuditLogger::new(store.clone_inner_conn()))));
    let event_bus = XavierEventBus::new(100);
    let tasks = Arc::new(TaskService::new(Arc::new(InMemoryTaskStore::new())).with_event_bus(event_bus.clone()));

    // Subscribe secrets engine to task completion events
    let secrets_engine_for_bus = secrets_engine.clone();
    let mut receiver = event_bus.subscribe();
    tokio::spawn(async move {
        info!("Secrets engine listening for task events...");
        while let Ok(event) = receiver.recv().await {
            if let xavier::coordination::events::XavierEvent::TaskCompleted { task } = event {
                if let Some(agent_id) = &task.assignee {
                    info!("Task {} completed by agent {}. Revoking ephemeral keys...", task.id, agent_id);
                    secrets_engine_for_bus.revoke_for_agent(agent_id, "Task Completed").await;
                }
            }
        }
    });

    // Periodically cleanup expired secrets
    let secrets_engine_for_cleanup = secrets_engine.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            let removed = secrets_engine_for_cleanup.cleanup_expired().await;
            if removed > 0 {
                info!("Cleaned up {} expired secret leases", removed);
            }
        }
    });

    let state = CliState {
        memory: memory_port,
        store,
        workspace_id,
        workspace_dir,
        code_db,
        code_indexer,
        code_query,
        security: Arc::new(SecurityService::new()),
        _time_store: Some(time_store),
        agent_registry: SimpleAgentRegistry::new() as Arc<dyn AgentLifecyclePort>,
        panel_store,
        secrets_engine,
        event_bus,
        tasks,
        rate_manager: rate_manager.clone(),
    };

    info!(
        "Memory store initialized for workspace: {}",
        state.workspace_id
    );
    println!(
        "Memory store initialized for workspace: {}",
        state.workspace_id
    );
    println!("Code graph DB: {}", code_db_path.display());

    // Build router with state-aware routes
    #[allow(unused_mut)]
    let mut protected_routes = Router::new()
        .route("/memory/search", post(search_handler))
        .route("/memory/add", post(add_handler))
        .route("/memory/delete", post(delete_handler))
        .route("/memory/stats", get(stats_handler))
        .route("/code/scan", post(code_scan_handler))
        .route("/code/find", post(code_find_handler))
        .route("/code/context", post(code_context_handler))
        .route("/code/stats", get(code_stats_handler))
        .route("/v1/account/usage", get(account_usage_handler))
        .route("/security/scan", post(security_scan_handler))
        .route("/memory/query", post(memory_query_handler))
        .route("/session/compact", post(session_compact_handler))
        .route("/xavier/events/session", post(session_event_handler))
        .route("/xavier/time/metric", post(time_metric_handler))
        .route("/xavier/agents/register", post(agent_register_handler))
        .route("/xavier/agents/active", get(agent_active_handler))
        .route(
            "/xavier/agents/{id}/heartbeat",
            post(agent_heartbeat_handler),
        )
        .route("/xavier/agents/{id}/push", post(agent_push_context_handler))
        .route(
            "/xavier/agents/{id}/unregister",
            post(agent_unregister_handler),
        )
        .route("/xavier/sync/check", post(sync_check_handler))
        .route("/xavier/sync/check", get(sync_check_handler))
        .route("/xavier/verify/save", post(verify_save_handler))
        .route(
            "/panel/api/threads",
            get(panel_list_threads).post(panel_create_thread),
        )
        .route(
            "/panel/api/threads/{thread_id}",
            get(panel_get_thread).delete(panel_delete_thread),
        )
        .route("/panel/api/chat", post(panel_process_chat))
        .route("/secrets/lend", post(lend_handler))
        .route("/secrets/leases", get(leases_handler))
        .route("/secrets/revoke", post(revoke_handler))
        .route("/secrets/status/{token}", get(status_handler))
        .route("/v1/proxy/chat/completions", post(crate::cli::proxy::chat_proxy))
        .route("/v1/proxy/chat/completions/batch", post(crate::cli::proxy::chat_batch_proxy))
        .route("/v1/usage/status/{provider}", get(usage_status_handler))
        .route("/v1/usage/update", post(usage_update_handler))
        .route("/v1/usage/cooldown", post(usage_cooldown_handler))
        .layer(middleware::from_fn(auth_middleware));

    // Add enterprise plugin routes if feature is enabled
    #[cfg(feature = "enterprise")]
    {
        use xavier::adapters::inbound::http::routes::{
            plugins_health_handler, plugins_sync_handler,
        };
        protected_routes = protected_routes
            .route("/plugins/health", get(plugins_health_handler))
            .route("/plugins/sync", post(plugins_sync_handler));
    }

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/build", get(build_handler))
        .route("/ready", get(readiness_handler))
        .route("/readiness", get(readiness_handler))
        .route("/panel", get(panel_index))
        .route("/panel/assets/{*path}", get(panel_asset))
        .merge(protected_routes)
        .with_state(state);

    let listener = TcpListener::bind(&bind_addr).await?;
    let bound_addr = listener.local_addr()?;

    info!("Xavier HTTP server listening on http://{}", bound_addr);
    println!("Xavier HTTP server listening on http://{}", bound_addr);
    println!("Press Ctrl+C to stop");

    // Initialize enterprise plugin registry if feature is enabled
    #[cfg(feature = "enterprise")]
    {
        use xavier::adapters::inbound::http::routes::init_plugin_registry;
        init_plugin_registry();
        info!("Enterprise plugin system initialized");
    }

    // Start session sync task cron (M5)
    let sync_task = SessionSyncTask::new(health_adapter);
    let sync_shutdown = sync_task.spawn_cron_once();
    if sync_shutdown.is_some() {
        info!("SessionSyncTask cron started");
    } else {
        info!("SessionSyncTask cron already running; skipped duplicate start");
    }

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            if let Err(error) = tokio::signal::ctrl_c().await {
                info!("Failed to listen for Ctrl+C shutdown signal: {}", error);
            }
            if let Some(shutdown) = sync_shutdown {
                shutdown.shutdown();
                // Await the cron task to finish with a 5-second timeout
                shutdown.wait_for_shutdown(Duration::from_secs(5)).await;
            }
        })
        .await?;

    Ok(())
}

pub async fn health_handler() -> Response {
    json_response(
        StatusCode::OK,
        serde_json::json!({
            "status": "ok",
            "service": "xavier",
            "version": env!("CARGO_PKG_VERSION"),
        }),
    )
}

pub async fn readiness_handler(State(state): State<CliState>) -> Response {
    let memory_store = match state.store.health().await {
        Ok(detail) => serde_json::json!({
            "ready": true,
            "detail": detail,
        }),
        Err(error) => serde_json::json!({
            "ready": false,
            "detail": error.to_string(),
        }),
    };
    let code_graph = state
        .code_db
        .stats()
        .map(|stats| {
            serde_json::json!({
                "ready": true,
                "total_files": stats.total_files,
                "total_symbols": stats.total_symbols,
                "total_imports": stats.total_imports,
            })
        })
        .unwrap_or_else(|error| {
            serde_json::json!({
                "ready": false,
                "error": error.to_string(),
            })
        });

    let ready = memory_store["ready"].as_bool().unwrap_or(false)
        && code_graph["ready"].as_bool().unwrap_or(false);

    json_response(
        if ready {
            StatusCode::OK
        } else {
            StatusCode::SERVICE_UNAVAILABLE
        },
        serde_json::json!({
            "status": if ready { "ok" } else { "degraded" },
            "service": "xavier",
            "workspace_id": state.workspace_id,
            "memory_store": memory_store,
            "code_graph": code_graph,
        }),
    )
}

pub async fn build_handler(State(state): State<CliState>) -> Response {
    json_response(
        StatusCode::OK,
        serde_json::json!({
            "service": "xavier",
            "version": env!("CARGO_PKG_VERSION"),
            "workspace_id": state.workspace_id,
            "base_url": resolve_base_url(),
            "memory_backend": std::env::var("XAVIER_MEMORY_BACKEND").unwrap_or_else(|_| "vec".to_string()),
            "code_graph_db_path": code_graph_db_path(),
        }),
    )
}

pub async fn account_usage_handler(State(_state): State<CliState>, headers: HeaderMap) -> Response {
    let expected_token = match std::env::var("XAVIER_TOKEN") {
        Ok(token) => token,
        Err(_) => {
            return json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({
                    "status": "error",
                    "message": "XAVIER_TOKEN is not configured",
                }),
            )
        }
    };

    let provided_token = headers
        .get("X-Xavier-Token")
        .and_then(|value| value.to_str().ok());
    if provided_token != Some(expected_token.as_str()) {
        return json_response(
            StatusCode::UNAUTHORIZED,
            serde_json::json!({
                "status": "error",
                "message": "Unauthorized",
            }),
        );
    }

    json_response(
        StatusCode::OK,
        serde_json::json!({
            "status": "ok",
            "optimization": {
                "router_direct_count": 0,
                "semantic_cache_hits": 0,
                "semantic_cache_misses": 0,
            },
        }),
    )
}

pub fn json_response(status: StatusCode, body: serde_json::Value) -> Response {
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .header("x-request-id", uuid::Uuid::new_v4().to_string())
        .body(axum::body::Body::from(body.to_string()))
        .unwrap_or_else(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({"status":"error"}).to_string(),
            )
                .into_response()
        })
}

pub async fn auth_middleware(req: Request<Body>, next: Next) -> Response {
    let expected_token = match std::env::var("XAVIER_TOKEN") {
        Ok(token) => token,
        Err(_) => {
            return json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({"status":"error","message":"XAVIER_TOKEN is not configured"}),
            );
        }
    };

    let provided_token = req
        .headers()
        .get("X-Xavier-Token")
        .and_then(|value| value.to_str().ok());

    if provided_token != Some(expected_token.as_str()) {
        return json_response(
            StatusCode::UNAUTHORIZED,
            serde_json::json!({"status":"error","message":"Unauthorized"}),
        );
    }

    next.run(req).await
}

pub async fn panel_list_threads(State(state): State<CliState>) -> Response {
    json_response(
        StatusCode::OK,
        serde_json::json!(state.panel_store.list_threads().await),
    )
}

pub async fn panel_create_thread(
    State(state): State<CliState>,
    Json(payload): Json<CreateThreadRequest>,
) -> Response {
    let title_hint = payload
        .title
        .or(payload.message)
        .unwrap_or_else(|| "New Thread".to_string());

    match state.panel_store.create_thread(&title_hint).await {
        Ok(thread) => json_response(
            StatusCode::OK,
            serde_json::to_value(xavier::memory::session_store::ThreadSummary::from(&thread))
                .unwrap_or_else(|_| serde_json::json!({})),
        ),
        Err(error) => json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            serde_json::json!({ "error": error.to_string() }),
        ),
    }
}

pub async fn panel_get_thread(
    State(state): State<CliState>,
    AxumPath(thread_id): AxumPath<String>,
) -> Response {
    match state.panel_store.get_thread(&thread_id).await {
        Some(thread) => json_response(
            StatusCode::OK,
            serde_json::to_value(SessionStore::detail_from_thread(thread))
                .unwrap_or_else(|_| serde_json::json!({})),
        ),
        None => json_response(
            StatusCode::NOT_FOUND,
            serde_json::json!({ "error": "thread not found" }),
        ),
    }
}

pub async fn panel_delete_thread(
    State(state): State<CliState>,
    AxumPath(thread_id): AxumPath<String>,
) -> Response {
    match state.panel_store.delete_thread(&thread_id).await {
        Ok(true) => json_response(StatusCode::OK, serde_json::json!({ "deleted": true })),
        Ok(false) => json_response(
            StatusCode::NOT_FOUND,
            serde_json::json!({ "error": "thread not found" }),
        ),
        Err(error) => json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            serde_json::json!({ "error": error.to_string() }),
        ),
    }
}

pub async fn panel_process_chat(
    State(state): State<CliState>,
    Json(payload): Json<PanelChatRequest>,
) -> Response {
    match panel_process_chat_inner(&state, payload).await {
        Ok(response) => json_response(
            StatusCode::OK,
            serde_json::to_value(response).unwrap_or_else(|_| serde_json::json!({})),
        ),
        Err(error) => json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            serde_json::json!({ "error": error.to_string() }),
        ),
    }
}

pub async fn panel_process_chat_inner(
    state: &CliState,
    payload: PanelChatRequest,
) -> Result<PanelChatResponse> {
    let thread = match payload.thread_id.as_deref() {
        Some(thread_id) => state
            .panel_store
            .get_thread(thread_id)
            .await
            .ok_or_else(|| anyhow!("thread {thread_id} not found"))?,
        None => state.panel_store.create_thread(&payload.message).await?,
    };

    let user_message = PanelMessage {
        id: ulid::Ulid::new().to_string(),
        role: "user".to_string(),
        plain_text: payload.message.clone(),
        openui_lang: None,
        created_at: chrono::Utc::now(),
        metadata: serde_json::json!({}),
    };
    state
        .panel_store
        .append_message(&thread.id, user_message)
        .await?;

    let assistant_message = PanelMessage {
        id: ulid::Ulid::new().to_string(),
        role: "assistant".to_string(),
        plain_text: format!(
            "Structured Xavier response for: {}",
            payload.message.trim()
        ),
        openui_lang: Some(format!(
            "<SectionBlock title=\"Xavier\" description=\"{}\"><InfoCard title=\"Status\" value=\"Ready\" /></SectionBlock>",
            payload.message.replace('"', "'")
        )),
        created_at: chrono::Utc::now(),
        metadata: serde_json::json!({
            "rules": ["deterministic", "ci-safe"],
            "components": ["SectionBlock", "InfoCard"],
            "timings": { "total_ms": 0 }
        }),
    };

    let updated = state
        .panel_store
        .append_message(&thread.id, assistant_message)
        .await?;

    Ok(PanelChatResponse {
        thread: xavier::memory::session_store::ThreadSummary::from(&updated),
        messages: updated.messages,
    })
}

pub async fn search_handler(
    State(state): State<CliState>,
    axum::Json(payload): axum::Json<SearchPayload>,
) -> impl axum::response::IntoResponse {
    // Security scan on query before searching
    let sec_result = state.security.process_input(&payload.query);
    if !sec_result.allowed {
        info!(
            "Search blocked by security: injection detected (confidence={})",
            sec_result.detection.confidence
        );
        return axum::Json(serde_json::json!({
            "results": <Vec<serde_json::Value>>::new(),
            "query": payload.query,
            "count": 0,
            "blocked": true,
            "reason": "security_policy_violation",
            "detection": {
                "is_injection": sec_result.detection.is_injection,
                "confidence": sec_result.detection.confidence,
                "attack_type": sec_result.detection.attack_type.as_str(),
            },
            "workspace_id": state.workspace_id,
        }));
    }

    let effective_query = sec_result.effective_input();
    let limit = payload.limit.clamp(1, 100);
    info!("Search request: query={}, limit={}", effective_query, limit);

    let results: Vec<MemoryRecord> = match state.memory.search(effective_query, None).await {
        Ok(results) => results,
        Err(e) => {
            info!("Search error: {}", e);
            return axum::Json(serde_json::json!({
                "results": [],
                "query": payload.query,
                "count": 0,
                "error": e.to_string(),
                "workspace_id": state.workspace_id,
            }));
        }
    };

    {
        // Deduplicate results by content hash, keeping most recent by updated_at
        // NOTE: deduplicate_by_content_hash was removed - results passed through
        let search_results: Vec<serde_json::Value> = results
            .into_iter()
            .map(|document| {
                serde_json::json!({
                    "id": document.id,
                    "content": document.content,
                    "embedding": document.embedding,
                })
            })
            .collect();

        axum::Json(serde_json::json!({
            "results": search_results,
            "query": payload.query,
            "count": search_results.len(),
            "workspace_id": state.workspace_id,
        }))
    }
}

pub async fn add_handler(
    State(state): State<CliState>,
    axum::Json(payload): axum::Json<AddPayload>,
) -> impl axum::response::IntoResponse {
    // Security scan on content before adding
    let sec_result = state.security.process_input(&payload.content);
    if !sec_result.allowed {
        info!(
            "Add blocked by security: injection detected (confidence={})",
            sec_result.detection.confidence
        );
        return axum::Json(serde_json::json!({
            "status": "blocked",
            "reason": "security_policy_violation",
            "detection": {
                "is_injection": sec_result.detection.is_injection,
                "confidence": sec_result.detection.confidence,
                "attack_type": sec_result.detection.attack_type.as_str(),
                "message": sec_result.detection.message,
            }
        }));
    }

    let effective_content = sec_result.effective_input();

    let path = payload
        .path
        .unwrap_or_else(|| format!("memory/{}", ulid::Ulid::new()));
    let mut metadata = payload.metadata.unwrap_or(serde_json::json!({}));

    // Add title to metadata if provided
    if let Some(title) = payload.title {
        if let Some(obj) = metadata.as_object_mut() {
            obj.insert("title".to_string(), serde_json::json!(title));
        }
    }

    info!(
        "Add memory request: path={}, content_len={}",
        path,
        effective_content.len()
    );

    let record = MemoryRecord {
        id: String::new(),
        workspace_id: state.workspace_id.clone(),
        path: path.clone(),
        content: effective_content.to_string(),
        metadata: serde_json::json!({"kind": "Context", "namespace": "Global"}),
        embedding: vec![],
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        revision: 1,
        primary: true,
        parent_id: None,
        revisions: vec![],
    };
    match state.memory.add(record).await {
        Ok(id) => {
            info!("Memory added successfully: {}", path);
            axum::Json(serde_json::json!({
                "status": "ok",
                "message": "Memory added",
                "path": path,
                "id": id,
                "security": {
                    "scanned": true,
                    "sanitized": sec_result.sanitized_input.is_some(),
                    "attack_type": sec_result.detection.attack_type.as_str(),
                }
            }))
        }
        Err(e) => {
            info!("Add memory error: {}", e);
            axum::Json(serde_json::json!({
                "status": "error",
                "message": e.to_string(),
            }))
        }
    }
}

pub async fn delete_handler(
    State(state): State<CliState>,
    headers: HeaderMap,
    axum::extract::Json(payload): axum::extract::Json<DeleteMemoryRequest>,
) -> Response {
    let expected_token = match std::env::var("XAVIER_TOKEN") {
        Ok(token) => token,
        Err(_) => {
            return json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({"status":"error","message":"XAVIER_TOKEN not configured"}),
            );
        }
    };

    match headers
        .get("X-Xavier-Token")
        .and_then(|value| value.to_str().ok())
    {
        Some(token) if token == expected_token => {}
        _ => {
            return json_response(
                StatusCode::UNAUTHORIZED,
                serde_json::json!({"status":"error","message":"Unauthorized"}),
            );
        }
    }

    let id_or_path = payload
        .id
        .or(payload.path)
        .filter(|value| !value.trim().is_empty());
    let Some(id_or_path) = id_or_path else {
        return json_response(
            StatusCode::BAD_REQUEST,
            serde_json::json!({"status":"error","message":"Provide either id or path"}),
        );
    };

    match state.store.delete(&state.workspace_id, &id_or_path).await {
        Ok(Some(record)) => json_response(
            StatusCode::OK,
            serde_json::json!({
                "status": "ok",
                "deleted": true,
                "id": record.id,
                "path": record.path,
            }),
        ),
        Ok(None) => json_response(
            StatusCode::NOT_FOUND,
            serde_json::json!({
                "status": "not_found",
                "deleted": false,
                "id_or_path": id_or_path,
            }),
        ),
        Err(error) => json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            serde_json::json!({
                "status": "error",
                "message": error.to_string(),
            }),
        ),
    }
}

pub async fn stats_handler(State(state): State<CliState>) -> impl axum::response::IntoResponse {
    axum::Json(serde_json::json!({
        "status": "ok",
        "workspace_id": state.workspace_id,
        "version": "0.4.1",
    }))
}

pub async fn security_scan_handler(
    State(state): State<CliState>,
    axum::Json(payload): axum::Json<SecurityScanPayload>,
) -> impl axum::response::IntoResponse {
    let result = state.security.process_input(&payload.input);
    axum::Json(serde_json::json!({
        "status": if result.allowed { "allowed" } else { "blocked" },
        "allowed": result.allowed,
        "detection": {
            "is_injection": result.detection.is_injection,
            "confidence": result.detection.confidence,
            "attack_type": result.detection.attack_type.as_str(),
            "message": result.detection.message,
        },
        "sanitized_input": result.sanitized_input,
        "original_input": result.original_input,
    }))
}

pub async fn memory_query_handler(
    State(state): State<CliState>,
    axum::Json(payload): axum::Json<MemoryQueryPayload>,
) -> impl axum::response::IntoResponse {
    // Security scan on query
    let sec_result = state.security.process_input(&payload.query);
    if !sec_result.allowed {
        return axum::Json(serde_json::json!({
            "status": "blocked",
            "reason": "security_policy_violation",
            "detection": {
                "is_injection": sec_result.detection.is_injection,
                "confidence": sec_result.detection.confidence,
                "attack_type": sec_result.detection.attack_type.as_str(),
            }
        }));
    }

    let limit = payload.limit.unwrap_or(10).clamp(1, 100);
    info!(
        "Memory query request: query={}, limit={}",
        payload.query, limit
    );

    // Use search (equivalent to hybrid search)
    match state.memory.search(&payload.query, None).await {
        Ok(results) => {
            let documents: Vec<_> = results
                .into_iter()
                .map(|doc| {
                    serde_json::json!({
                        "id": doc.id,
                        "content": doc.content,
                        "embedding": doc.embedding,
                    })
                })
                .collect();

            axum::Json(serde_json::json!({
                "status": "ok",
                "query": payload.query,
                "count": documents.len(),
                "results": documents,
                "workspace_id": state.workspace_id,
            }))
        }
        Err(e) => {
            info!("Memory query error: {}", e);
            axum::Json(serde_json::json!({
                "status": "error",
                "message": e.to_string(),
            }))
        }
    }
}

pub async fn code_scan_handler(
    State(state): State<CliState>,
    axum::Json(payload): axum::Json<CodeScanPayload>,
) -> impl axum::response::IntoResponse {
    // Security: validate path to prevent path traversal
    let requested_path = payload.path.unwrap_or_else(|| ".".to_string());

    // Security scan on path
    let sec_result = state.security.process_input(&requested_path);
    if !sec_result.allowed {
        info!(
            "code/scan blocked by security: injection detected (confidence={})",
            sec_result.detection.confidence
        );
        return axum::Json(serde_json::json!({
            "status": "blocked",
            "reason": "security_policy_violation",
            "detection": {
                "is_injection": sec_result.detection.is_injection,
                "confidence": sec_result.detection.confidence,
                "attack_type": sec_result.detection.attack_type.as_str(),
            }
        }));
    }

    // Path traversal protection via canonicalization
    let workspace_root =
        std::path::absolute(&state.workspace_dir).unwrap_or_else(|_| PathBuf::from("."));
    let Ok(abs_path) = std::path::absolute(&requested_path) else {
        return axum::Json(serde_json::json!({
            "status": "error",
            "message": "invalid path",
            "indexed_files": 0,
        }));
    };
    if !abs_path.starts_with(&workspace_root) {
        warn!(
            "Path traversal blocked: {} is outside workspace root {}",
            abs_path.display(),
            workspace_root.display()
        );
        return axum::Json(serde_json::json!({
            "status": "error",
            "message": "path outside workspace not allowed",
            "indexed_files": 0,
        }));
    }

    let path = requested_path;
    info!("Code scan request: path={}", path);

    match state.code_indexer.index(std::path::Path::new(&path)).await {
        Ok(stats) => axum::Json(serde_json::json!({
            "status": "ok",
            "indexed_files": stats.total_files,
            "indexed_symbols": stats.total_symbols,
            "indexed_imports": stats.total_imports,
            "duration_ms": stats.duration_ms,
            "paths": [path],
            "languages": stats.languages,
        })),
        Err(error) => axum::Json(serde_json::json!({
            "status": "error",
            "message": error.to_string(),
            "indexed_files": 0,
            "indexed_symbols": 0,
            "indexed_imports": 0,
            "paths": [path],
        })),
    }
}

pub async fn code_find_handler(
    State(state): State<CliState>,
    axum::Json(payload): axum::Json<CodeFindPayload>,
) -> impl axum::response::IntoResponse {
    // Security scan on query
    let sec_result = state.security.process_input(&payload.query);
    if !sec_result.allowed {
        info!(
            "code/find blocked by security: injection detected (confidence={})",
            sec_result.detection.confidence
        );
        return axum::Json(serde_json::json!({
            "status": "blocked",
            "reason": "security_policy_violation",
            "blocked": true,
            "detection": {
                "is_injection": sec_result.detection.is_injection,
                "confidence": sec_result.detection.confidence,
                "attack_type": sec_result.detection.attack_type.as_str(),
            }
        }));
    }

    let query = sec_result.effective_input().to_string();
    let pattern = match secure_optional_request_field(
        &state.security,
        "code/find pattern",
        payload.pattern.as_deref(),
    ) {
        Ok(pattern) => pattern,
        Err(sec_result) => {
            info!(
                "code/find blocked by security: pattern rejected (confidence={})",
                sec_result.detection.confidence
            );
            return axum::Json(serde_json::json!({
                "status": "blocked",
                "reason": "security_policy_violation",
                "blocked": true,
                "field": "pattern",
                "detection": {
                    "is_injection": sec_result.detection.is_injection,
                    "confidence": sec_result.detection.confidence,
                    "attack_type": sec_result.detection.attack_type.as_str(),
                }
            }));
        }
    };
    let kind = match secure_optional_request_field(
        &state.security,
        "code/find kind",
        payload.kind.as_deref(),
    ) {
        Ok(kind) => kind,
        Err(sec_result) => {
            info!(
                "code/find blocked by security: kind rejected (confidence={})",
                sec_result.detection.confidence
            );
            return axum::Json(serde_json::json!({
                "status": "blocked",
                "reason": "security_policy_violation",
                "blocked": true,
                "field": "kind",
                "detection": {
                    "is_injection": sec_result.detection.is_injection,
                    "confidence": sec_result.detection.confidence,
                    "attack_type": sec_result.detection.attack_type.as_str(),
                }
            }));
        }
    };
    let limit = payload.limit.clamp(1, 100);
    info!(
        "Code find request: query={}, limit={}, kind={:?}, pattern={:?}",
        query, limit, kind, pattern
    );

    let symbols = code_find_symbols(
        &state.code_query,
        &query,
        kind.as_deref(),
        pattern.as_deref(),
        limit,
    );

    let results: Vec<_> = symbols
        .into_iter()
        .map(|symbol| {
            serde_json::json!({
                "id": symbol.id,
                "path": symbol.file_path,
                "symbol": symbol.name,
                "symbol_type": format!("{:?}", symbol.kind),
                "language": format!("{:?}", symbol.lang),
                "line": symbol.start_line,
                "end_line": symbol.end_line,
                "signature": symbol.signature,
                "parent": symbol.parent,
            })
        })
        .collect();

    axum::Json(serde_json::json!({
        "status": "ok",
        "query": query,
        "count": results.len(),
        "results": results,
    }))
}

pub async fn code_stats_handler(State(state): State<CliState>) -> impl axum::response::IntoResponse {
    match state.code_db.stats() {
        Ok(stats) => axum::Json(serde_json::json!({
            "status": "ok",
            "total_files": stats.total_files,
            "total_symbols": stats.total_symbols,
            "total_imports": stats.total_imports,
            "languages": stats.languages,
        })),
        Err(error) => axum::Json(serde_json::json!({
            "status": "error",
            "message": error.to_string(),
            "total_files": 0,
            "total_symbols": 0,
            "total_imports": 0,
        })),
    }
}

pub async fn code_context_handler(
    State(state): State<CliState>,
    axum::Json(payload): axum::Json<CodeContextPayload>,
) -> impl axum::response::IntoResponse {
    // Security scan on query
    let sec_result = state.security.process_input(&payload.query);
    if !sec_result.allowed {
        info!(
            "code/context blocked by security: injection detected (confidence={})",
            sec_result.detection.confidence
        );
        return axum::Json(serde_json::json!({
            "status": "blocked",
            "reason": "security_policy_violation",
            "blocked": true,
            "detection": {
                "is_injection": sec_result.detection.is_injection,
                "confidence": sec_result.detection.confidence,
                "attack_type": sec_result.detection.attack_type.as_str(),
            }
        }));
    }

    let limit = payload.limit.clamp(1, 100);
    let kind_limit = if payload.query.trim().is_empty() {
        limit
    } else {
        10_000
    };
    let budget_tokens = payload.budget_tokens.clamp(100, 8000);

    let mut symbols = if let Some(kind) = payload.kind.as_deref() {
        match kind.to_ascii_lowercase().as_str() {
            "function" | "fn" => state.code_query.functions(kind_limit).unwrap_or_default(),
            "struct" => state.code_query.structs(kind_limit).unwrap_or_default(),
            "class" => state.code_query.classes(kind_limit).unwrap_or_default(),
            "enum" => state.code_query.enums(kind_limit).unwrap_or_default(),
            _ => state
                .code_query
                .search(&payload.query, limit)
                .map(|result| result.symbols)
                .unwrap_or_default(),
        }
    } else {
        state
            .code_query
            .search(&payload.query, limit)
            .map(|result| result.symbols)
            .unwrap_or_default()
    };
    filter_symbols_by_query(&mut symbols, &payload.query);
    symbols.truncate(limit);

    let mut used_tokens = 0usize;
    let mut context = Vec::new();

    for symbol in symbols {
        let signature = symbol.signature.clone().unwrap_or_default();
        let compact = serde_json::json!({
            "symbol": symbol.name,
            "symbol_type": format!("{:?}", symbol.kind),
            "language": format!("{:?}", symbol.lang),
            "path": symbol.file_path,
            "line": symbol.start_line,
            "end_line": symbol.end_line,
            "signature": signature,
        });
        let estimated = estimate_tokens(&compact.to_string());
        if used_tokens + estimated > budget_tokens && !context.is_empty() {
            break;
        }
        used_tokens += estimated;
        context.push(compact);
    }

    axum::Json(serde_json::json!({
        "status": "ok",
        "query": payload.query,
        "budget_tokens": budget_tokens,
        "estimated_tokens": used_tokens,
        "count": context.len(),
        "context": context,
    }))
}

pub async fn session_event_handler(
    State(state): State<CliState>,
    axum::Json(event): axum::Json<SessionEvent>,
) -> impl axum::response::IntoResponse {
    let entry = match PanelThreadEntry::from_session_event(&event) {
        Some(e) => e,
        None => {
            return axum::Json(serde_json::json!({
                "status": "skipped",
                "reason": "no_content",
                "session_id": event.session_id,
            }))
        }
    };

    let entry_content =
        match secure_external_input(&state.security, "session event content", &entry.content) {
            Ok(content) => content,
            Err(response) => return axum::Json(response),
        };

    let content = format!(
        "[{}] {}: {}",
        entry.timestamp.format("%Y-%m-%d %H:%M:%S"),
        entry.role,
        entry_content
    );
    let _metadata = serde_json::json!({
        "session_id": event.session_id,
        "role": entry.role,
        "event_type": entry.event_type,
        "kind": "session_event",
    });

    let record_path = format!("sessions/{}/thread", event.session_id);
    let record = MemoryRecord {
        id: String::new(),
        workspace_id: state.workspace_id.clone(),
        path: record_path.clone(),
        content,
        metadata: serde_json::json!({"kind": "Context", "namespace": "Session"}),
        embedding: vec![],
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        revision: 1,
        primary: true,
        parent_id: None,
        revisions: vec![],
    };
    match state.memory.add(record).await {
        Ok(id) => {
            info!("Session event indexed: {} -> {}", event.session_id, id);
            axum::Json(serde_json::json!({
                "status": "ok",
                "session_id": event.session_id,
                "path": record_path,
                "id": id,
            }))
        }
        Err(e) => {
            info!("Failed to index session event: {}", e);
            axum::Json(serde_json::json!({
                "status": "error",
                "error": e.to_string(),
            }))
        }
    }
}

pub async fn session_compact_handler(
    State(state): State<CliState>,
    axum::Json(payload): axum::Json<SessionCompactPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let session_id = &payload.session_id;
    let threshold = payload.threshold_percent.clamp(1.0, 100.0);

    // Get current token usage - use provided value or query from memory
    let current_tokens = match payload.current_tokens {
        Some(t) => t,
        None => {
            // Query session context to estimate token count
            match state
                .memory
                .search(&format!("session {} compact", session_id), None)
                .await
            {
                Ok(docs) => {
                    let total_chars: usize = docs.iter().map(|d| d.content.len()).sum();
                    total_chars / 4
                }
                Err(_) => 0,
            }
        }
    };

    // Estimate max tokens based on typical 200K context window
    let estimated_max_tokens = 200_000;
    let usage_percent = (current_tokens as f64 / estimated_max_tokens as f64) * 100.0;

    let triggered = usage_percent >= threshold;

    if !triggered {
        return Ok(axum::Json(serde_json::json!({
            "status": "ok",
            "triggered": false,
            "session_id": session_id,
            "usage_percent": usage_percent,
            "threshold_percent": threshold,
            "message": format!(
                "Compaction not needed: {:.1}% < {:.1}%",
                usage_percent,
                threshold
            ),
        })));
    }

    // Compaction triggered - fetch session history, keep last 20%
    let search_path = format!("sessions/{}/thread", session_id);
    let all_docs = match state.memory.get(&search_path).await {
        Ok(Some(doc)) => vec![doc],
        Ok(None) => state
            .memory
            .search(&search_path, None)
            .await
            .unwrap_or_default(),
        Err(_) => vec![],
    };

    let total_docs = all_docs.len();
    let keep_count = (total_docs as f64 * 0.20).ceil() as usize;
    let compact_docs: Vec<_> = all_docs.iter().rev().take(keep_count).collect();

    let mut compacted_content = String::new();
    compacted_content.push_str(&format!(
        "[COMPACTED] Session {} - Original {} entries, kept {}\n",
        session_id,
        total_docs,
        compact_docs.len()
    ));

    if let Some(oldest) = all_docs.first() {
        compacted_content.push_str(&format!(
            "[EARLIEST] {}\n",
            &oldest.content[..oldest.content.len().min(200)]
        ));
    }

    compacted_content.push_str("\n=== KEPT RECENT ENTRIES ===\n");
    for doc in &compact_docs {
        let truncate_content = if doc.content.len() > 500 {
            format!("{}... [truncated]", &doc.content[..500])
        } else {
            doc.content.clone()
        };
        compacted_content.push_str(&format!("[ENTRY] {}\n\n", truncate_content));
    }

    let compact_path = format!("context/{}/compact", session_id);
    let _metadata = serde_json::json!({
        "session_id": session_id,
        "original_entries": total_docs,
        "kept_entries": compact_docs.len(),
        "compaction_percent": 20,
        "usage_percent": usage_percent,
        "threshold_percent": threshold,
        "kind": "session_compact",
    });
    let record = MemoryRecord {
        id: String::new(),
        workspace_id: state.workspace_id.clone(),
        path: compact_path.clone(),
        content: compacted_content.clone(),
        metadata: serde_json::json!({"kind": "Context", "namespace": "Session"}),
        embedding: vec![],
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        revision: 1,
        primary: true,
        parent_id: None,
        revisions: vec![],
    };
    match state.memory.add(record).await {
        Ok(id) => {
            info!(
                "Session {} compacted: {} -> {} entries, saved to {}",
                session_id,
                total_docs,
                compact_docs.len(),
                id
            );
            Ok(axum::Json(serde_json::json!({
                "status": "ok",
                "triggered": true,
                "session_id": session_id,
                "usage_percent": usage_percent,
                "threshold_percent": threshold,
                "original_entries": total_docs,
                "kept_entries": compact_docs.len(),
                "compacted_path": compact_path,
                "compacted_id": id,
                "message": format!(
                    "Compacted session {}: {} -> {} entries (kept last 20%)",
                    session_id,
                    total_docs,
                    compact_docs.len()
                ),
            })))
        }
        Err(e) => {
            info!("Session compaction error: {}", e);
            Ok(axum::Json(serde_json::json!({
                "status": "error",
                "triggered": true,
                "session_id": session_id,
                "error": e.to_string(),
            })))
        }
    }
}

pub async fn agent_register_handler(
    State(state): State<CliState>,
    axum::Json(payload): axum::Json<AgentRegisterPayload>,
) -> impl axum::response::IntoResponse {
    let metadata = xavier::coordination::agent_registry::AgentMetadata {
        name: payload.name,
        capabilities: payload.capabilities.unwrap_or_default(),
        role: payload.role,
        endpoint: payload.endpoint,
    };
    let session_id = payload
        .session_id
        .unwrap_or_else(|| payload.agent_id.clone());

    let success = state
        .agent_registry
        .register(payload.agent_id.clone(), session_id.clone(), metadata)
        .await;

    axum::Json(serde_json::json!({
        "status": if success { "ok" } else { "error" },
        "agent_id": payload.agent_id,
        "session_id": session_id,
        "message": if success { "Agent registered successfully" } else { "Registration failed" },
    }))
}

pub async fn agent_heartbeat_handler(
    State(state): State<CliState>,
    axum::extract::Path(agent_id): axum::extract::Path<String>,
) -> impl axum::response::IntoResponse {
    let success = state.agent_registry.heartbeat(&agent_id).await;

    axum::Json(serde_json::json!({
        "status": if success { "ok" } else { "error" },
        "agent_id": agent_id,
        "message": if success { "Heartbeat recorded" } else { "Agent not found" },
    }))
}

pub async fn agent_active_handler(State(state): State<CliState>) -> impl axum::response::IntoResponse {
    let active = state.agent_registry.get_active_agents().await;

    axum::Json(serde_json::json!({
        "status": "ok",
        "active_agents": active.len(),
        "agents": active.iter().map(|a| serde_json::json!({
            "agent_id": a.agent_id,
            "session_id": a.session_id,
            "last_heartbeat": a.last_heartbeat.to_rfc3339(),
            "name": a.metadata.name,
            "capabilities": a.metadata.capabilities,
            "role": a.metadata.role,
            "endpoint": a.metadata.endpoint,
        })).collect::<Vec<_>>(),
    }))
}

pub async fn agent_push_context_handler(
    State(state): State<CliState>,
    axum::extract::Path(agent_id): axum::extract::Path<String>,
    axum::Json(payload): axum::Json<AgentPushContextPayload>,
) -> impl axum::response::IntoResponse {
    // Verify agent exists
    let agent = state.agent_registry.get(&agent_id).await;
    if agent.is_none() {
        return axum::Json(serde_json::json!({
            "status": "error",
            "message": "Agent not registered",
        }));
    }

    let context = match secure_external_input(&state.security, "agent context", &payload.context) {
        Ok(context) => context,
        Err(response) => return axum::Json(response),
    };

    // Store context in memory at agents/{id}/context
    let path = format!("agents/{}/context", agent_id);
    let mut metadata = payload.metadata.unwrap_or(serde_json::json!({}));
    if let Some(obj) = metadata.as_object_mut() {
        obj.insert("agent_id".to_string(), serde_json::json!(agent_id));
        obj.insert(
            "pushed_at".to_string(),
            serde_json::json!(chrono::Utc::now().to_rfc3339()),
        );
    }

    let record = MemoryRecord {
        id: String::new(),
        workspace_id: state.workspace_id.clone(),
        path: path.clone(),
        content: context,
        metadata,
        embedding: vec![],
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        revision: 1,
        primary: true,
        parent_id: None,
        revisions: vec![],
    };
    match state.memory.add(record).await {
        Ok(doc_id) => axum::Json(serde_json::json!({
            "status": "ok",
            "path": path,
            "document_id": doc_id,
            "message": "Context stored successfully",
        })),
        Err(e) => axum::Json(serde_json::json!({
            "status": "error",
            "message": format!("Failed to store context: {}", e),
        })),
    }
}

pub async fn agent_unregister_handler(
    State(state): State<CliState>,
    axum::extract::Path(agent_id): axum::extract::Path<String>,
) -> impl axum::response::IntoResponse {
    let success = state.agent_registry.unregister(&agent_id).await;

    axum::Json(serde_json::json!({
        "status": if success { "ok" } else { "error" },
        "agent_id": agent_id,
        "message": if success { "Agent unregistered" } else { "Agent not found or already unregistered" },
    }))
}

pub async fn search_memories(query: &str, limit: usize) -> Result<()> {
    let query = secure_cli_input("search query", query, 4_096)?;
    let limit = limit.clamp(1, 100);
    let token = xavier_token();
    let base_url = resolve_base_url();
    let url = format!("{}/memory/search", base_url);

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .header("X-Xavier-Token", &token)
        .json(&serde_json::json!({
            "query": query,
            "limit": limit
        }))
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                let body: serde_json::Value = resp.json().await.unwrap_or_default();
                println!("\nSearch results for: {}", query);
                println!("{}", serde_json::to_string_pretty(&body)?);
            } else {
                println!("Search failed with status: {}", resp.status());
            }
        }
        Err(e) => {
            println!("Error connecting to Xavier server: {}", e);
            println!("Configured endpoint: {}", base_url);
            println!("Is the server running? (xavier http)");
        }
    }

    Ok(())
}

// ── Payload structs ──────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub(crate) struct SearchPayload {
    query: String,
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default, rename = "filters")]
    _filters: Option<MemoryQueryFilters>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CodeScanPayload {
    #[serde(default)]
    path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CodeFindPayload {
    query: String,
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    pattern: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CodeContextPayload {
    #[serde(default)]
    query: String,
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default = "default_token_budget")]
    budget_tokens: usize,
    #[serde(default)]
    kind: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AddPayload {
    content: String,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    metadata: Option<serde_json::Value>,
    #[serde(default)]
    title: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DeleteMemoryRequest {
    id: Option<String>,
    path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SecurityScanPayload {
    input: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MemoryQueryPayload {
    query: String,
    limit: Option<usize>,
    #[serde(default, rename = "filters")]
    _filters: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SessionCompactPayload {
    session_id: String,
    #[serde(default)]
    current_tokens: Option<usize>,
    #[serde(default = "default_compaction_threshold")]
    threshold_percent: f64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AgentRegisterPayload {
    agent_id: String,
    session_id: Option<String>,
    name: Option<String>,
    capabilities: Option<Vec<String>>,
    role: Option<String>,
    endpoint: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AgentHeartbeatPayload {
    agent_id: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AgentPushContextPayload {
    context: String,
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct SessionContext {
    pub session_id: String,
    pub context: Option<String>,
    pub tokens_restored: usize,
}

#[derive(Debug, Deserialize)]
pub struct SwarmConfig {
    pub agents: Vec<SwarmAgentConfig>,
}

#[derive(Debug, Deserialize)]
pub struct SwarmAgentConfig {
    pub name: String,
    pub provider: String,
    pub model: Option<String>,
    pub skills: Option<Vec<String>>,
    pub context: Option<HashMap<String, String>>,
    pub task: String,
}

pub fn default_limit() -> usize {
    10
}

pub fn default_token_budget() -> usize {
    800
}

pub fn default_compaction_threshold() -> f64 {
    80.0
}

// Secrets Handlers
#[derive(Debug, Deserialize)]
pub struct LendSecretPayload {
    pub secret_name: String,
    pub secret_value: String,
    pub agent_id: String,
    pub ttl_seconds: u64,
}

#[derive(Debug, Deserialize)]
pub struct RevokeLeasePayload {
    pub token: String,
}

pub async fn lend_handler(
    State(state): State<CliState>,
    Json(payload): Json<LendSecretPayload>,
) -> Response {
    match state.secrets_engine.lend(&payload.secret_name, &payload.secret_value, &payload.agent_id, payload.ttl_seconds).await {
        Ok(lease) => json_response(StatusCode::OK, serde_json::to_value(lease).unwrap_or_default()),
        Err(e) => json_response(StatusCode::INTERNAL_SERVER_ERROR, serde_json::json!({ "error": e.to_string() })),
    }
}

pub async fn leases_handler(State(state): State<CliState>) -> Response {
    let leases = state.secrets_engine.list_leases().await;
    json_response(StatusCode::OK, serde_json::to_value(leases).unwrap_or_default())
}

pub async fn revoke_handler(
    State(state): State<CliState>,
    Json(payload): Json<RevokeLeasePayload>,
) -> Response {
    match state.secrets_engine.revoke(&payload.token, "Manual API Call").await {
        Ok(_) => json_response(StatusCode::OK, serde_json::json!({ "status": "revoked" })),
        Err(e) => json_response(StatusCode::NOT_FOUND, serde_json::json!({ "error": e.to_string() })),
    }
}

pub async fn status_handler(
    State(state): State<CliState>,
    AxumPath(token): AxumPath<String>,
) -> Response {
    match state.secrets_engine.get_lease(&token).await {
        Some(status) => json_response(StatusCode::OK, serde_json::to_value(status).unwrap_or_default()),
        None => json_response(StatusCode::NOT_FOUND, serde_json::json!({ "error": "Lease not found" })),
    }
}

#[derive(Debug, Deserialize)]
pub struct UsageUpdatePayload {
    pub provider: String,
    pub percentage: f32,
}

#[derive(Debug, Deserialize)]
pub struct UsageCooldownPayload {
    pub provider: String,
    pub minutes: i64,
}

pub async fn usage_status_handler(
    State(state): State<CliState>,
    AxumPath(provider): AxumPath<String>,
) -> Response {
    match state.rate_manager.get_status(&provider).await {
        Ok(status) => json_response(StatusCode::OK, serde_json::to_value(status).unwrap_or_default()),
        Err(e) => json_response(StatusCode::INTERNAL_SERVER_ERROR, serde_json::json!({ "error": e.to_string() })),
    }
}

pub async fn usage_update_handler(
    State(state): State<CliState>,
    Json(payload): Json<UsageUpdatePayload>,
) -> Response {
    match state.rate_manager.update_manual_limit(&payload.provider, payload.percentage).await {
        Ok(_) => json_response(StatusCode::OK, serde_json::json!({ "status": "ok" })),
        Err(e) => json_response(StatusCode::INTERNAL_SERVER_ERROR, serde_json::json!({ "error": e.to_string() })),
    }
}

pub async fn usage_cooldown_handler(
    State(state): State<CliState>,
    Json(payload): Json<UsageCooldownPayload>,
) -> Response {
    match state.rate_manager.report_429(&payload.provider, payload.minutes).await {
        Ok(_) => json_response(StatusCode::OK, serde_json::json!({ "status": "ok" })),
        Err(e) => json_response(StatusCode::INTERNAL_SERVER_ERROR, serde_json::json!({ "error": e.to_string() })),
    }
}
