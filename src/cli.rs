//! Xavier2 CLI - Command-line interface

use anyhow::Result;
use axum::{
    extract::State,
    routing::{get, post},
    Router,
};
use clap::{Parser, Subcommand};
use serde::Deserialize;
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tracing::info;

use xavier2::memory::qmd_memory::{MemoryDocument, QmdMemory};
use xavier2::memory::schema::MemoryQueryFilters;
use xavier2::memory::sqlite_vec_store::VecSqliteMemoryStore;
use xavier2::memory::surreal_store::{MemoryRecord, MemoryStore};
use xavier2::security::{
    ProcessResult, SecurityService,
};

/// CLI-specific application state with direct memory store access
#[derive(Clone)]
pub struct CliState {
    pub memory: Arc<QmdMemory>,
    pub store: Arc<dyn MemoryStore>,
    pub workspace_id: String,
    pub code_db: Arc<code_graph::db::CodeGraphDB>,
    pub code_indexer: Arc<code_graph::indexer::Indexer>,
    pub code_query: Arc<code_graph::query::QueryEngine>,
    pub security: Arc<SecurityService>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Start Xavier2 HTTP server
    Http { port: Option<u16> },
    /// Start Xavier2 MCP-stdio server
    Mcp,
    /// Search memories
    Search { query: String, limit: Option<usize> },
    /// Add a memory
    Add {
        content: String,
        title: Option<String>,
    },
    /// Show statistics
    Stats,
}

/// Xavier2 - Fast Vector Memory for AI Agents
#[derive(Parser)]
#[command(name = "xavier2")]
#[command(about = "Xavier2 - Fast Vector Memory for AI Agents", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Command,
}

impl Cli {
    pub async fn run(&self) -> Result<()> {
        match &self.cmd {
            Command::Http { port } => {
                let port = port.unwrap_or(8006);
                start_http_server(port).await
            }
            Command::Mcp => start_mcp_stdio().await,
            Command::Search { query, limit } => {
                println!("Searching memories: {}", query);
                println!("(Searching via HTTP API on localhost:8006)");
                let lim = limit.unwrap_or(10);
                search_memories(&query, lim).await
            }
            Command::Add { content, title } => {
                let title_display = title.as_deref().unwrap_or("Untitled");
                println!("Adding memory: {}", title_display);
                add_memory(content, title.as_ref().map(|s| s.as_str())).await
            }
            Command::Stats => {
                println!("Fetching Xavier2 statistics...");
                show_stats().await
            }
        }
    }
}

async fn start_http_server(port: u16) -> Result<()> {
    info!("Starting Xavier2 HTTP server on port {}", port);

    // Initialize the memory store
    let store = Arc::new(VecSqliteMemoryStore::from_env().await?);
    let workspace_id =
        std::env::var("XAVIER2_DEFAULT_WORKSPACE_ID").unwrap_or_else(|_| "default".to_string());
    let durable_state = store.load_workspace_state(&workspace_id).await?;
    let docs = Arc::new(RwLock::new(
        durable_state
            .memories
            .iter()
            .map(MemoryRecord::to_document)
            .collect::<Vec<MemoryDocument>>(),
    ));
    let memory = Arc::new(QmdMemory::new_with_workspace(docs, workspace_id.clone()));
    let store: Arc<dyn MemoryStore> = store;
    memory.set_store(Arc::clone(&store)).await;
    memory.init().await?;

    let code_db_path = code_graph_db_path();
    if let Some(parent) = code_db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let code_db = Arc::new(code_graph::db::CodeGraphDB::new(&code_db_path)?);
    let code_indexer = Arc::new(code_graph::indexer::Indexer::new(Arc::clone(&code_db)));
    let code_query = Arc::new(code_graph::query::QueryEngine::new(Arc::clone(&code_db)));

    let state = CliState {
        memory,
        store,
        workspace_id,
        code_db,
        code_indexer,
        code_query,
        security: Arc::new(SecurityService::new()),
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
    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/memory/search", post(search_handler))
        .route("/memory/add", post(add_handler))
        .route("/memory/stats", get(stats_handler))
        .route("/code/scan", post(code_scan_handler))
        .route("/code/find", post(code_find_handler))
        .route("/code/context", post(code_context_handler))
        .route("/code/stats", get(code_stats_handler))
        .route("/ready", get(readiness_handler))
        .route("/security/scan", post(security_scan_handler))
        .route("/memory/query", post(memory_query_handler))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(&addr).await?;

    info!("Xavier2 HTTP server listening on http://{}", addr);
    println!("Xavier2 HTTP server listening on http://{}", addr);
    println!("Press Ctrl+C to stop");

    axum::serve(listener, app).await?;

    Ok(())
}

fn code_graph_db_path() -> PathBuf {
    std::env::var("XAVIER2_CODE_GRAPH_DB_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("data").join("code_graph.db"))
}

// HTTP Handlers
async fn health_handler() -> &'static str {
    r#"{"status":"ok","service":"xavier2","version":"0.4.1"}"#
}

async fn readiness_handler(State(state): State<CliState>) -> impl axum::response::IntoResponse {
    let health = state.store.health().await.unwrap_or_else(|e| e.to_string());
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

    axum::Json(serde_json::json!({
        "status": "ok",
        "workspace_id": state.workspace_id,
        "memory_store": health,
        "code_graph": code_graph,
    }))
}

#[derive(Debug, Deserialize)]
struct SearchPayload {
    query: String,
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default)]
    filters: Option<MemoryQueryFilters>,
}

#[derive(Debug, Deserialize)]
struct CodeScanPayload {
    #[serde(default)]
    path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CodeFindPayload {
    #[serde(default)]
    query: String,
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    pattern: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CodeContextPayload {
    #[serde(default)]
    query: String,
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default = "default_token_budget")]
    budget_tokens: usize,
    #[serde(default)]
    kind: Option<String>,
}

fn default_token_budget() -> usize {
    800
}

fn default_limit() -> usize {
    10
}

async fn search_handler(
    State(state): State<CliState>,
    axum::Json(payload): axum::Json<SearchPayload>,
) -> impl axum::response::IntoResponse {
    // Security scan on query before searching
    let sec_result = state.security.process_input(&payload.query);
    if !sec_result.allowed {
        info!("Search blocked by security: injection detected (confidence={})", sec_result.detection.confidence);
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
    let limit = payload.limit.max(1).min(100);
    info!("Search request: query={}, limit={}", effective_query, limit);

    match state
        .memory
        .search_filtered(effective_query, limit, payload.filters.as_ref())
        .await
    {
        Ok(results) => {
            let search_results: Vec<serde_json::Value> = results
                .into_iter()
                .map(|document| {
                    serde_json::json!({
                        "id": document.id,
                        "path": document.path,
                        "content": document.content,
                        "metadata": document.metadata,
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
        Err(e) => {
            info!("Search error: {}", e);
            axum::Json(serde_json::json!({
                "results": [],
                "query": payload.query,
                "count": 0,
                "error": e.to_string(),
                "workspace_id": state.workspace_id,
            }))
        }
    }
}

#[derive(Debug, Deserialize)]
struct AddPayload {
    content: String,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    metadata: Option<serde_json::Value>,
    #[serde(default)]
    title: Option<String>,
}

async fn add_handler(
    State(state): State<CliState>,
    axum::Json(payload): axum::Json<AddPayload>,
) -> impl axum::response::IntoResponse {
    // Security scan on content before adding
    let sec_result = state.security.process_input(&payload.content);
    if !sec_result.allowed {
        info!("Add blocked by security: injection detected (confidence={})", sec_result.detection.confidence);
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

    match state
        .memory
        .add_document(path.clone(), effective_content.to_string(), metadata)
        .await
    {
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

async fn stats_handler(State(state): State<CliState>) -> impl axum::response::IntoResponse {
    let count = state.memory.count().await.unwrap_or(0);
    let usage = state.memory.usage().await;
    let cache = state.memory.cache_metrics().await;
    axum::Json(serde_json::json!({
        "status": "ok",
        "total_memories": count,
        "workspace_id": state.workspace_id,
        "storage_bytes": usage.storage_bytes,
        "cache_hits": cache.hits,
        "cache_misses": cache.misses,
        "version": "0.4.1",
    }))
}

// === Security Scan Handler ===
#[derive(Debug, Deserialize)]
struct SecurityScanPayload {
    input: String,
}

async fn security_scan_handler(
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

// === Memory Query Handler (with LLM synthesis) ===
#[derive(Debug, Deserialize)]
struct MemoryQueryPayload {
    query: String,
    limit: Option<usize>,
    filters: Option<serde_json::Value>,
}

async fn memory_query_handler(
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

    let limit = payload.limit.unwrap_or(10).max(1).min(100);
    info!("Memory query request: query={}, limit={}", payload.query, limit);

    // Use search (equivalent to hybrid search)
    match state.memory.search(&payload.query, limit).await {
        Ok(results) => {
            let documents: Vec<_> = results.into_iter().map(|doc| {
                serde_json::json!({
                    "id": doc.id,
                    "path": doc.path,
                    "content": doc.content,
                    "metadata": doc.metadata,
                    "embedding": doc.embedding,
                })
            }).collect();

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

async fn code_scan_handler(
    State(state): State<CliState>,
    axum::Json(payload): axum::Json<CodeScanPayload>,
) -> impl axum::response::IntoResponse {
    let path = payload.path.unwrap_or_else(|| ".".to_string());
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

async fn code_find_handler(
    State(state): State<CliState>,
    axum::Json(payload): axum::Json<CodeFindPayload>,
) -> impl axum::response::IntoResponse {
    // Security scan on query
    let sec_result = state.security.process_input(&payload.query);
    if !sec_result.allowed {
        info!("code/find blocked by security: injection detected (confidence={})", sec_result.detection.confidence);
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

    let limit = payload.limit.max(1).min(100);
    let kind_limit = if payload.query.trim().is_empty() {
        limit
    } else {
        10_000
    };
    info!(
        "Code find request: query={}, limit={}, kind={:?}, pattern={:?}",
        payload.query, limit, payload.kind, payload.pattern
    );

    let mut symbols = if let Some(pattern) = payload.pattern.as_deref() {
        state
            .code_query
            .search_by_pattern(pattern, limit)
            .unwrap_or_default()
    } else if let Some(kind) = payload.kind.as_deref() {
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
        "query": payload.query,
        "count": results.len(),
        "results": results,
    }))
}

async fn code_stats_handler(State(state): State<CliState>) -> impl axum::response::IntoResponse {
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

async fn code_context_handler(
    State(state): State<CliState>,
    axum::Json(payload): axum::Json<CodeContextPayload>,
) -> impl axum::response::IntoResponse {
    // Security scan on query
    let sec_result = state.security.process_input(&payload.query);
    if !sec_result.allowed {
        info!("code/context blocked by security: injection detected (confidence={})", sec_result.detection.confidence);
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

    let limit = payload.limit.max(1).min(100);
    let kind_limit = if payload.query.trim().is_empty() {
        limit
    } else {
        10_000
    };
    let budget_tokens = payload.budget_tokens.max(100).min(8000);

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

fn estimate_tokens(text: &str) -> usize {
    (text.len() / 4).max(1)
}

fn filter_symbols_by_query(symbols: &mut Vec<code_graph::types::Symbol>, query: &str) {
    let query = query.trim().to_ascii_lowercase();
    if query.is_empty() {
        return;
    }

    symbols.retain(|symbol| {
        symbol.name.to_ascii_lowercase().contains(&query)
            || symbol
                .signature
                .as_deref()
                .unwrap_or_default()
                .to_ascii_lowercase()
                .contains(&query)
            || symbol.file_path.to_ascii_lowercase().contains(&query)
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// MCP-stdio Server
// ─────────────────────────────────────────────────────────────────────────────

/// Start the MCP-stdio server.
/// Uses JSON-RPC 2.0 over stdin/stdout (newline-delimited messages).
/// This call blocks indefinitely, processing MCP protocol messages.
async fn start_mcp_stdio() -> Result<()> {
    // Initialize memory store directly (same as HTTP server)
    let store: Arc<dyn MemoryStore> = Arc::new(VecSqliteMemoryStore::from_env().await?);
    let workspace_id =
        std::env::var("XAVIER2_DEFAULT_WORKSPACE_ID").unwrap_or_else(|_| "default".to_string());
    let durable_state = store.load_workspace_state(&workspace_id).await?;
    let docs = Arc::new(RwLock::new(
        durable_state
            .memories
            .iter()
            .map(MemoryRecord::to_document)
            .collect::<Vec<MemoryDocument>>(),
    ));
    let memory = Arc::new(QmdMemory::new_with_workspace(docs, workspace_id.clone()));
    memory.set_store(Arc::clone(&store)).await;
    memory.init().await?;

    // Async stdin/stdout
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin).lines();
    let mut writer = tokio::io::BufWriter::new(stdout);

    const JSONRPC: &str = "2.0";

    loop {
        // Read next line (blocking until client sends something)
        let line = match reader.next_line().await {
            Ok(Some(l)) => l,
            Ok(None) => break, // EOF
            Err(_e) => {
                // Read error - bail out silently
                break;
            }
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Parse JSON-RPC request
        let request: serde_json::Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                // Parse error - respond and continue
                let resp = serde_json::json!({
                    "jsonrpc": JSONRPC,
                    "id": serde_json::Value::Null,
                    "error": {
                        "code": -32700,
                        "message": format!("Parse error: {}", e)
                    }
                });
                let line = format!("{}\n", resp);
                let _ = writer.write_all(line.as_bytes()).await;
                let _ = writer.flush().await;
                continue;
            }
        };

        let method = request.get("method").and_then(|v| v.as_str());
        let id = request.get("id").clone();
        // A notification has id === null or no id field
        let is_notification = id.as_ref().map_or(true, |v| v.is_null());

        let response: Option<serde_json::Value> = match method {
            None => Some(serde_json::json!({
                "jsonrpc": JSONRPC,
                "id": serde_json::Value::Null,
                "error": {
                    "code": -32600,
                    "message": "Invalid Request: method is required"
                }
            })),

            // ── initialize ──────────────────────────────────────────────
            Some("initialize") => {
                // initialized = true;  // MCP spec: remember to enforce pre-init rejection if needed
                Some(serde_json::json!({
                    "jsonrpc": JSONRPC,
                    "id": id,
                    "result": {
                        "protocolVersion": "2024-11-05",
                        "capabilities": { "tools": {} },
                        "serverInfo": {
                            "name": "xavier2",
                            "version": "0.4.1"
                        }
                    }
                }))
            }

            // ── initialized (notification - no response) ───────────────
            Some("initialized") if is_notification => None,

            // ── tools/list ──────────────────────────────────────────────
            Some("tools/list") => Some(serde_json::json!({
                "jsonrpc": JSONRPC,
                "id": id,
                "result": {
                    "tools": [
                        {
                            "name": "search",
                            "description": "Search memories in Xavier2 vector store",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "query": {
                                        "type": "string",
                                        "description": "Search query string"
                                    },
                                    "limit": {
                                        "type": "number",
                                        "description": "Max results (default 10, max 100)",
                                        "default": 10
                                    }
                                },
                                "required": ["query"]
                            }
                        },
                        {
                            "name": "add",
                            "description": "Add a memory to Xavier2",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "content": {
                                        "type": "string",
                                        "description": "Memory content text"
                                    },
                                    "title": {
                                        "type": "string",
                                        "description": "Optional title"
                                    }
                                },
                                "required": ["content"]
                            }
                        },
                        {
                            "name": "stats",
                            "description": "Get Xavier2 memory statistics (total count, cache metrics)",
                            "inputSchema": {
                                "type": "object",
                                "properties": {}
                            }
                        }
                    ]
                }
            })),

            // ── tools/call ──────────────────────────────────────────────
            Some("tools/call") => {
                let args = request
                    .get("params")
                    .and_then(|p| p.get("arguments"))
                    .and_then(|a| a.as_object())
                    .cloned()
                    .unwrap_or_default();

                let tool_name = request
                    .get("params")
                    .and_then(|p| p.get("name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let text = match tool_name {
                    "search" => {
                        let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                        let limit =
                            args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
                        let limit = limit.max(1).min(100);
                        match memory.search_filtered(query, limit, None).await {
                            Ok(results) => {
                                let summary = serde_json::json!({
                                    "count": results.len(),
                                    "results": results.into_iter().map(|doc| {
                                        serde_json::json!({
                                            "id": doc.id,
                                            "path": doc.path,
                                            "content": doc.content,
                                            "metadata": doc.metadata,
                                        })
                                    }).collect::<Vec<_>>()
                                });
                                serde_json::to_string_pretty(&summary).unwrap_or_default()
                            }
                            Err(e) => format!("{{\"error\": \"{}\"}}", e),
                        }
                    }
                    "add" => {
                        let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
                        let title = args.get("title").and_then(|v| v.as_str());
                        let path = format!("memory/{}", ulid::Ulid::new());
                        let mut metadata = serde_json::json!({});
                        if let Some(t) = title {
                            if let Some(obj) = metadata.as_object_mut() {
                                obj.insert("title".to_string(), serde_json::json!(t));
                            }
                        }
                        match memory
                            .add_document(path.clone(), content.to_string(), metadata)
                            .await
                        {
                            Ok(id) => serde_json::json!({
                                "status": "ok",
                                "path": path,
                                "id": id,
                            })
                            .to_string(),
                            Err(e) => format!("{{\"error\": \"{}\"}}", e),
                        }
                    }
                    "stats" => {
                        let count = memory.count().await.unwrap_or(0);
                        let usage = memory.usage().await;
                        let cache = memory.cache_metrics().await;
                        serde_json::json!({
                            "status": "ok",
                            "total_memories": count,
                            "workspace_id": workspace_id,
                            "storage_bytes": usage.storage_bytes,
                            "cache_hits": cache.hits,
                            "cache_misses": cache.misses,
                            "version": "0.4.1",
                        })
                        .to_string()
                    }
                    _ => format!(
                        "Unknown tool: {}. Available tools: search, add, stats",
                        tool_name
                    ),
                };

                Some(serde_json::json!({
                    "jsonrpc": JSONRPC,
                    "id": id,
                    "result": {
                        "content": [{
                            "type": "text",
                            "text": text
                        }]
                    }
                }))
            }

            // ── Unknown method ──────────────────────────────────────────
            Some(m) => Some(serde_json::json!({
                "jsonrpc": JSONRPC,
                "id": id,
                "error": {
                    "code": -32601,
                    "message": format!("Method not found: {}", m)
                }
            })),
        };

        // Send response only if not a notification
        if !is_notification {
            if let Some(resp) = response {
                let line = format!("{}\n", resp);
                let _ = writer.write_all(line.as_bytes()).await;
                let _ = writer.flush().await;
            }
        }
    }

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// CLI commands (HTTP-based for Search / Add / Stats)
// ─────────────────────────────────────────────────────────────────────────────

async fn search_memories(query: &str, limit: usize) -> Result<()> {
    let token = std::env::var("XAVIER2_TOKEN").unwrap_or_else(|_| "dev-token".to_string());
    let port = std::env::var("XAVIER2_PORT").unwrap_or_else(|_| "8006".to_string());
    let url = format!("http://localhost:{}/memory/search", port);

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .header("X-Xavier2-Token", &token)
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
            println!("Error connecting to Xavier2 server: {}", e);
            println!("Is the server running? (xavier2 http)");
        }
    }

    Ok(())
}

async fn add_memory(content: &str, title: Option<&str>) -> Result<()> {
    let token = std::env::var("XAVIER2_TOKEN").unwrap_or_else(|_| "dev-token".to_string());
    let port = std::env::var("XAVIER2_PORT").unwrap_or_else(|_| "8006".to_string());
    let url = format!("http://localhost:{}/memory/add", port);

    let mut body = serde_json::json!({
        "content": content,
        "metadata": {}
    });

    if let Some(t) = title {
        body["metadata"]["title"] = serde_json::json!(t);
    }

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .header("X-Xavier2-Token", &token)
        .json(&body)
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                println!("Memory added successfully!");
            } else {
                println!("Failed to add memory: {}", resp.status());
            }
        }
        Err(e) => {
            println!("Error connecting to Xavier2 server: {}", e);
            println!("Is the server running? (xavier2 http)");
        }
    }

    Ok(())
}

async fn show_stats() -> Result<()> {
    let token = std::env::var("XAVIER2_TOKEN").unwrap_or_else(|_| "dev-token".to_string());
    let port = std::env::var("XAVIER2_PORT").unwrap_or_else(|_| "8006".to_string());
    let url = format!("http://localhost:{}/memory/stats", port);

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("X-Xavier2-Token", &token)
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                let body: serde_json::Value = resp.json().await.unwrap_or_default();
                println!("\nXavier2 Statistics:");
                println!("{}", serde_json::to_string_pretty(&body)?);
            } else {
                println!("Failed to get stats: {}", resp.status());
            }
        }
        Err(e) => {
            println!("Error connecting to Xavier2 server: {}", e);
            println!("Is the server running? (xavier2 http)");
        }
    }

    Ok(())
}
