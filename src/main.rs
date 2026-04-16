// Xavier2 - Cognitive Memory System
// Minimal viable slice for production

use anyhow::Result;
use axum::{
    body::Body,
    http::{
        header::{HeaderName, HeaderValue},
        Request, StatusCode,
    },
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use clap::{Parser, Subcommand};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use xavier2::{
    api,
    adapters::outbound::vec::pattern_adapter::PatternAdapter,
    agents::RuntimeConfig,
    app::security_service::SecurityService,
    memory::file_indexer::FileIndexer,
    server,
    workspace::{UsageEvent, WorkspaceRegistry},
    AppState,
};

#[derive(Parser)]
#[command(name = "xavier2")]
#[command(about = "Xavier2 - Cognitive Memory Runtime for AI Agents", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the Xavier2 HTTP server (default)
    Server,
    /// Synchronization operations
    Sync {
        #[command(subcommand)]
        action: SyncAction,
    },
    /// Start MCP server in stdio mode (for IDE integration)
    McpStdio,
    /// Generate a 12-hour session token
    Token,
    /// Import OpenClaw or Engram artifacts into Xavier2 memory
    BridgeImport {
        #[arg(long)]
        source: xavier2::memory::bridge::BridgeSource,
        #[arg(long)]
        path: String,
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        scope: Option<String>,
        #[arg(long)]
        agent_id: Option<String>,
        #[arg(long)]
        session_id: Option<String>,
    },
}

#[derive(Subcommand)]
enum SyncAction {
    /// Export local memories to compressed chunks
    Export,
    /// Import memories from compressed chunks
    Import,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let is_mcp = matches!(cli.command, Some(Commands::McpStdio));

    let log_filter = std::env::var("RUST_LOG")
        .ok()
        .or_else(|| std::env::var("XAVIER2_LOG_LEVEL").ok())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            if is_mcp {
                "warn".to_string()
            } else {
                "info".to_string()
            }
        });

    let json_log = std::env::var("XAVIER2_LOG_FORMAT").ok().as_deref() == Some("json");

    if is_mcp {
        tracing_subscriber::registry()
            .with(EnvFilter::new(log_filter))
            .with(
                tracing_subscriber::fmt::layer()
                    .with_writer(std::io::stderr)
                    .with_ansi(false),
            )
            .init();
    } else if json_log {
        tracing_subscriber::registry()
            .with(EnvFilter::new(log_filter))
            .with(tracing_subscriber::fmt::layer().json())
            .init();
    } else {
        tracing_subscriber::registry()
            .with(EnvFilter::new(log_filter))
            .with(tracing_subscriber::fmt::layer())
            .init();
    }

    match cli.command {
        Some(Commands::Sync { action }) => handle_sync(action).await,
        Some(Commands::McpStdio) => handle_mcp_stdio().await,
        Some(Commands::Token) => handle_token_generation().await,
        Some(Commands::BridgeImport {
            source,
            path,
            project,
            scope,
            agent_id,
            session_id,
        }) => handle_bridge_import(source, path, project, scope, agent_id, session_id).await,
        _ => start_server().await,
    }
}

async fn handle_mcp_stdio() -> Result<()> {
    let state = setup_app_state().await?;
    let token = std::env::var("XAVIER2_TOKEN").unwrap_or_else(|_| "dev-token".to_string());

    let context = state
        .workspace_registry
        .authenticate(&token)
        .await
        .ok_or_else(|| anyhow::anyhow!("Failed to authenticate default workspace"))?;

    xavier2::server::mcp_stdio::run_stdio_loop(state, context).await
}

async fn handle_token_generation() -> Result<()> {
    let state = setup_app_state().await?;
    let token = std::env::var("XAVIER2_TOKEN").unwrap_or_else(|_| "dev-token".to_string());

    let context = state
        .workspace_registry
        .authenticate(&token)
        .await
        .ok_or_else(|| anyhow::anyhow!("Failed to authenticate default workspace"))?;

    let session_token = context.workspace.generate_session_token().await?;
    println!("Session token generated (valid for 12 hours):");
    println!("{}", session_token);
    Ok(())
}

async fn handle_bridge_import(
    source: xavier2::memory::bridge::BridgeSource,
    path: String,
    project: Option<String>,
    scope: Option<String>,
    agent_id: Option<String>,
    session_id: Option<String>,
) -> Result<()> {
    let state = setup_app_state().await?;
    let token = std::env::var("XAVIER2_TOKEN").unwrap_or_else(|_| "dev-token".to_string());

    let context = state
        .workspace_registry
        .authenticate(&token)
        .await
        .ok_or_else(|| anyhow::anyhow!("Failed to authenticate default workspace"))?;

    let stats = xavier2::memory::bridge::import_from_path(
        &context.workspace.memory,
        source,
        path,
        xavier2::memory::bridge::BridgeImportOptions {
            project,
            scope,
            agent_id,
            session_id,
        },
    )
    .await?;

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "status": "ok",
            "source": stats.source,
            "imported": stats.imported,
            "skipped": stats.skipped,
        }))?
    );

    Ok(())
}

async fn setup_app_state() -> Result<AppState> {
    let db_path = code_graph_db_path()?;
    let code_db = Arc::new(code_graph::db::CodeGraphDB::new(&db_path)?);
    let code_indexer = Arc::new(code_graph::indexer::Indexer::new(code_db.clone()));
    let code_query = Arc::new(code_graph::query::QueryEngine::new(code_db.clone()));

    let indexer = FileIndexer::new(
        xavier2::memory::file_indexer::FileIndexerConfig::default(),
        Some(code_indexer.clone()),
    );
    let workspace_registry =
        Arc::new(WorkspaceRegistry::default_from_env(RuntimeConfig::from_env()).await?);

    Ok(AppState {
        workspace_registry,
        code_indexer,
        code_query,
        code_db,
        indexer,
        pattern_adapter: Arc::new(PatternAdapter::new()),
        security_service: Arc::new(SecurityService::new()),
    })
}

async fn handle_sync(action: SyncAction) -> Result<()> {
    let workspace_registry = WorkspaceRegistry::default_from_env(RuntimeConfig::from_env()).await?;
    let token = std::env::var("XAVIER2_TOKEN").unwrap_or_else(|_| "dev-token".to_string());

    let context = workspace_registry
        .authenticate(&token)
        .await
        .ok_or_else(|| anyhow::anyhow!("Failed to authenticate default workspace"))?;

    match action {
        SyncAction::Export => {
            tracing::info!("Exporting memories to chunks...");
            let hash = context.workspace.export_sync().await?;
            println!("Exported chunk: {}", hash);
        }
        SyncAction::Import => {
            tracing::info!("Importing memories from chunks...");
            let count = context.workspace.import_sync().await?;
            println!("Imported {} new documents", count);
        }
    }
    Ok(())
}

fn print_license_notice() {
    let enterprise_key = std::env::var("XAVIER2_LICENSE_KEY").ok();
    let is_commercial = std::env::var("XAVIER2_COMMERCIAL_USE").is_ok();

    if enterprise_key.is_some() {
        eprintln!(
            r#"
 ╔══════════════════════════════════════════════════════════════╗
 ║  Xavier2 Enterprise License                                 ║
 ║  Thank you for your support of Xavier2 & SouthWest AI Labs  ║
 ╚══════════════════════════════════════════════════════════════╝
"#
        );
    } else if is_commercial {
        eprintln!(
            r#"
 ┌──────────────────────────────────────────────────────────────┐
 │  Xavier2 is free under MIT. If this instance is used for    │
 │  commercial products or services, please consider supporting │
 │  the project: enterprise@southwest-ai-labs.com              │
 │  Pricing: https://github.com/iberi22/xavier2-1/blob/main/docs/PRICING.md │
 └──────────────────────────────────────────────────────────────┘
"#
        );
    }
}

async fn start_server() -> Result<()> {
    print_license_notice();
    tracing::info!("Starting Xavier2 - Cognitive Memory Runtime");
    let state = setup_app_state().await?;

    let app = Router::new()
        .route("/health", get(server::http::health))
        .route("/readiness", get(server::http::readiness))
        .route("/build", get(server::http::build_info))
        .route("/panel", get(server::panel::panel_index))
        .route("/panel/", get(server::panel::panel_index))
        .route("/panel/assets/{*path}", get(server::panel::panel_asset))
        .route(
            "/panel/api/threads",
            get(server::panel::list_threads).post(server::panel::create_thread),
        )
        .route(
            "/panel/api/threads/{thread_id}",
            get(server::panel::get_thread).delete(server::panel::delete_thread),
        )
        .route("/panel/api/chat", post(server::panel::process_chat))
        .route("/memory/add", post(server::http::memory_add))
        .route("/memory/delete", post(server::http::memory_delete))
        .route("/memory/reset", post(server::http::memory_reset))
        .route("/memory/search", post(server::http::memory_search))
        .route("/memory/hybrid", post(api::search::hybrid_search))
        .route(
            "/memory/hybrid-search",
            post(server::http::memory_hybrid_search),
        )
        .route("/memory/query", post(server::http::memory_query))
        .route("/memory/curate", post(server::http::memory_curate))
        .route("/memory/manage", post(server::http::memory_manage))
        .route("/memory/decay", post(server::http::memory_decay))
        .route(
            "/memory/consolidate",
            post(server::http::memory_consolidate),
        )
        .route("/memory/reflect", post(server::http::memory_reflect))
        .route("/memory/quality", get(server::http::memory_quality))
        .route("/memory/evict", delete(server::http::memory_evict))
        .route("/memory/stats", get(server::http::memory_stats))
        .route("/memory/retrieve", post(server::http::memory_retrieve))
        .route("/memory/graph", get(server::http::memory_graph))
        .route(
            "/memory/graph/entity/{entity_id}",
            get(api::graph::memory_graph_entity),
        )
        .route(
            "/memory/graph/relations",
            get(api::graph::memory_graph_relations),
        )
        .route("/memory/graph/hops", post(server::http::memory_graph_hops))
        .route("/bridge/import", post(server::http::bridge_import))
        .route("/agents/run", post(server::http::agents_run))
        .route("/sync", post(server::http::sync_tier1))
        .route("/v1/account/usage", get(server::http::account_usage))
        .route("/v1/account/limits", get(server::http::account_limits))
        .route("/v1/sync/policies", get(server::http::sync_policies))
        .route(
            "/v1/providers/embeddings/status",
            get(server::http::embedding_provider_status),
        )
        .route("/code/scan", post(server::http::code_scan))
        .route("/code/find", post(server::http::code_find))
        .route("/code/stats", get(server::http::code_stats))
        // Security / Anticipator endpoints
        .route("/security/scan", post(server::http::security_scan))
        .route("/security/config", get(server::http::security_config))
        .route(
            "/mcp",
            post(server::mcp_server::mcp_post_handler)
                .get(server::mcp_server::mcp_get_handler)
                .delete(server::mcp_server::mcp_delete_handler),
        )
        .route(
            "/v1/memories",
            post(server::v1_api::v1_memories_add).get(server::v1_api::v1_memories_list),
        )
        .route(
            "/v1/memories/{id}",
            get(server::v1_api::v1_memories_get)
                .put(server::v1_api::v1_memories_update)
                .delete(server::v1_api::v1_memories_delete),
        )
        .route(
            "/v1/memories/search",
            post(server::v1_api::v1_memories_search),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .layer(middleware::from_fn(http_observability_middleware))
        .with_state(state);

    let addr = server_addr()?;
    tracing::info!("Xavier2 HTTP server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C, starting graceful shutdown");
        },
        _ = terminate => {
            tracing::info!("Received SIGTERM, starting graceful shutdown");
        },
    }
}

fn code_graph_db_path() -> Result<std::path::PathBuf> {
    let configured = std::env::var("XAVIER2_CODE_GRAPH_DB_PATH")
        .unwrap_or_else(|_| default_code_graph_db_path());
    let path = std::path::PathBuf::from(configured);

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    Ok(path)
}

fn default_code_graph_db_path() -> String {
    if std::path::Path::new("/data").exists() {
        "/data/code_graph.db".to_string()
    } else {
        "data/code_graph.db".to_string()
    }
}

fn server_addr() -> Result<SocketAddr> {
    let host = std::env::var("XAVIER2_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("XAVIER2_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(8003);

    Ok(format!("{host}:{port}").parse()?)
}

async fn auth_middleware(
    axum::extract::State(state): axum::extract::State<AppState>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    let dev_mode = std::env::var("XAVIER2_DEV_MODE").is_ok();
    let path = req.uri().path().to_string();

    // Skip auth for health check and in dev mode if explicitly set
    if path == "/health" || path == "/readiness" {
        return next.run(req).await;
    }

    // Allow loading the panel shell; keep all panel API requests protected.
    if path == "/panel" || path == "/panel/" || path.starts_with("/panel/assets/") {
        return next.run(req).await;
    }

    let token = if dev_mode {
        std::env::var("XAVIER2_TOKEN").unwrap_or_else(|_| "dev-token".to_string())
    } else if let Some(auth_header) = req.headers().get("X-Xavier2-Token") {
        match auth_header.to_str() {
            Ok(value) => value.to_string(),
            Err(_) => {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(serde_json::json!({
                        "status": "error",
                        "error": "Unauthorized",
                        "message": "Invalid X-Xavier2-Token header"
                    })),
                )
                    .into_response();
            }
        }
    } else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "status": "error",
                "error": "Unauthorized",
                "message": "Invalid or missing X-Xavier2-Token"
            })),
        )
            .into_response();
    };

    if let Some(workspace) = state.workspace_registry.authenticate(&token).await {
        let request_id = req
            .extensions()
            .get::<RequestId>()
            .map(|value| value.0.as_str())
            .unwrap_or("unknown");
        if let Err(error) = workspace
            .workspace
            .record_request(UsageEvent::from_request(req.method().as_str(), &path))
            .await
        {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to persist workspace usage state: {error}"),
            )
                .into_response();
        }
        if let Err(error) = workspace.workspace.ensure_within_request_limit().await {
            return (StatusCode::TOO_MANY_REQUESTS, error.to_string()).into_response();
        }

        tracing::info!(
            request_id,
            workspace_id = %workspace.workspace_id,
            path,
            "workspace_authenticated"
        );
        req.extensions_mut().insert(workspace);
        return next.run(req).await;
    }

    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({
            "status": "error",
            "error": "Unauthorized",
            "message": "Invalid or missing X-Xavier2-Token"
        })),
    )
        .into_response()
}

#[derive(Clone, Debug)]
struct RequestId(String);

async fn http_observability_middleware(mut req: Request<Body>, next: Next) -> Response {
    let request_id = req
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let remote_ip = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.split(',').next().unwrap_or(value).trim().to_string())
        .filter(|value| !value.is_empty());
    let start = Instant::now();

    req.extensions_mut().insert(RequestId(request_id.clone()));
    let mut response = next.run(req).await;
    let latency_ms = start.elapsed().as_millis() as u64;

    if let Ok(header_value) = HeaderValue::from_str(&request_id) {
        response
            .headers_mut()
            .insert(HeaderName::from_static("x-request-id"), header_value);
    }

    tracing::info!(
        request_id = %request_id,
        method = %method,
        path,
        status = response.status().as_u16(),
        latency_ms,
        remote_ip = remote_ip.as_deref().unwrap_or("unknown"),
        "http_request"
    );

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request, routing::get, Router};
    use tower::util::ServiceExt;

    #[test]
    fn code_graph_db_path_prefers_explicit_env() {
        let temp_dir =
            std::env::temp_dir().join(format!("xavier2-main-test-{}", std::process::id()));
        let db_path = temp_dir.join("nested").join("graph.db");

        unsafe {
            std::env::set_var("XAVIER2_CODE_GRAPH_DB_PATH", &db_path);
        }

        let resolved = code_graph_db_path().unwrap();
        assert_eq!(resolved, db_path.as_path());
        assert!(db_path.parent().unwrap().exists());

        unsafe {
            std::env::remove_var("XAVIER2_CODE_GRAPH_DB_PATH");
        }
    }

    #[test]
    fn server_addr_uses_env_configuration() {
        unsafe {
            std::env::set_var("XAVIER2_HOST", "127.0.0.1");
            std::env::set_var("XAVIER2_PORT", "8123");
        }

        let addr = server_addr().unwrap();
        assert_eq!(addr, "127.0.0.1:8123".parse().unwrap());

        unsafe {
            std::env::remove_var("XAVIER2_HOST");
            std::env::remove_var("XAVIER2_PORT");
        }
    }

    #[tokio::test]
    async fn panel_shell_is_public_but_panel_api_requires_token() {
        unsafe {
            std::env::set_var("XAVIER2_TOKEN", "test-token");
        }

        let workspace_registry = Arc::new(
            WorkspaceRegistry::default_from_env(RuntimeConfig::default())
                .await
                .unwrap(),
        );
        let state = AppState {
            workspace_registry,
            code_indexer: Arc::new(code_graph::indexer::Indexer::new(Arc::new(
                code_graph::db::CodeGraphDB::new(
                    &std::env::temp_dir().join(format!("xavier2-auth-{}.db", std::process::id())),
                )
                .unwrap(),
            ))),
            code_query: Arc::new(code_graph::query::QueryEngine::new(Arc::new(
                code_graph::db::CodeGraphDB::new(
                    &std::env::temp_dir()
                        .join(format!("xavier2-auth-query-{}.db", std::process::id())),
                )
                .unwrap(),
            ))),
            code_db: Arc::new(
                code_graph::db::CodeGraphDB::new(
                    &std::env::temp_dir()
                        .join(format!("xavier2-auth-db-{}.db", std::process::id())),
                )
                .unwrap(),
            ),
            indexer: FileIndexer::new(
                xavier2::memory::file_indexer::FileIndexerConfig::default(),
                None,
            ),
            pattern_adapter: Arc::new(xavier2::adapters::outbound::vec::pattern_adapter::PatternAdapter::new()),
            security_service: Arc::new(xavier2::app::security_service::SecurityService::new()),
        };

        let app = Router::new()
            .route("/panel", get(|| async { "panel" }))
            .route("/panel/api/threads", get(|| async { "threads" }))
            .layer(middleware::from_fn_with_state(
                state.clone(),
                auth_middleware,
            ))
            .with_state(state);

        let panel_request = Request::builder()
            .uri("/panel")
            .body(Body::empty())
            .unwrap();
        let panel_response = app.clone().oneshot(panel_request).await.unwrap();
        assert_eq!(panel_response.status(), StatusCode::OK);

        let protected_request = Request::builder()
            .uri("/panel/api/threads")
            .body(Body::empty())
            .unwrap();
        let protected_response = app.clone().oneshot(protected_request).await.unwrap();
        assert_eq!(protected_response.status(), StatusCode::UNAUTHORIZED);

        let authorized_request = Request::builder()
            .uri("/panel/api/threads")
            .header("X-Xavier2-Token", "test-token")
            .body(Body::empty())
            .unwrap();
        let authorized_response = app.oneshot(authorized_request).await.unwrap();
        assert_eq!(authorized_response.status(), StatusCode::OK);

        unsafe {
            std::env::remove_var("XAVIER2_TOKEN");
        }
    }

    #[tokio::test]
    async fn request_id_is_attached_to_success_and_error_responses() {
        let app = Router::new()
            .route("/ok", get(|| async { StatusCode::OK }))
            .route(
                "/blocked",
                get(|| async { (StatusCode::UNAUTHORIZED, "blocked") }),
            )
            .layer(middleware::from_fn(http_observability_middleware));

        let ok_response = app
            .clone()
            .oneshot(Request::builder().uri("/ok").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert!(ok_response.headers().contains_key("x-request-id"));

        let blocked_response = app
            .oneshot(
                Request::builder()
                    .uri("/blocked")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(blocked_response.headers().contains_key("x-request-id"));
    }
}
