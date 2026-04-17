//! Xavier2 CLI - Command-line interface

use anyhow::Result;
use axum::{
    extract::State,
    routing::{get, post},
    Router,
};
use clap::{Parser, Subcommand};
use serde::Deserialize;
use std::{net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tracing::info;

use xavier2::memory::qmd_memory::{MemoryDocument, QmdMemory};
use xavier2::memory::schema::MemoryQueryFilters;
use xavier2::memory::sqlite_vec_store::VecSqliteMemoryStore;
use xavier2::memory::surreal_store::{MemoryRecord, MemoryStore};

/// CLI-specific application state with direct memory store access
#[derive(Clone)]
pub struct CliState {
    pub memory: Arc<QmdMemory>,
    pub store: Arc<dyn MemoryStore>,
    pub workspace_id: String,
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
    Add { content: String, title: Option<String> },
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
            Command::Mcp => {
                println!("Starting Xavier2 MCP-stdio server...");
                start_mcp_stdio().await
            }
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
    let workspace_id = std::env::var("XAVIER2_DEFAULT_WORKSPACE_ID")
        .unwrap_or_else(|_| "default".to_string());
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
    let state = CliState {
        memory,
        store,
        workspace_id,
    };

    info!("Memory store initialized for workspace: {}", state.workspace_id);
    println!("Memory store initialized for workspace: {}", state.workspace_id);

    // Build router with state-aware routes
    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/memory/search", post(search_handler))
        .route("/memory/add", post(add_handler))
        .route("/memory/stats", get(stats_handler))
        .route("/ready", get(readiness_handler))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(&addr).await?;

    info!("Xavier2 HTTP server listening on http://{}", addr);
    println!("Xavier2 HTTP server listening on http://{}", addr);
    println!("Press Ctrl+C to stop");

    axum::serve(listener, app).await?;

    Ok(())
}

// HTTP Handlers
async fn health_handler() -> &'static str {
    r#"{"status":"ok","service":"xavier2","version":"0.4.1"}"#
}

async fn readiness_handler(State(state): State<CliState>) -> impl axum::response::IntoResponse {
    let health = state.store.health().await.unwrap_or_else(|e| e.to_string());
    axum::Json(serde_json::json!({
        "status": "ok",
        "workspace_id": state.workspace_id,
        "memory_store": health,
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

fn default_limit() -> usize {
    10
}

async fn search_handler(
    State(state): State<CliState>,
    axum::Json(payload): axum::Json<SearchPayload>,
) -> impl axum::response::IntoResponse {
    let limit = payload.limit.max(1).min(100);
    info!("Search request: query={}, limit={}", payload.query, limit);

    match state
        .memory
        .search_filtered(&payload.query, limit, payload.filters.as_ref())
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
    let path = payload
        .path
        .unwrap_or_else(|| format!("memory/{}", ulid::Ulid::new()));
    let content = payload.content;
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
        content.len()
    );

    match state
        .memory
        .add_document(path.clone(), content.clone(), metadata)
        .await
    {
        Ok(id) => {
            info!("Memory added successfully: {}", path);
            axum::Json(serde_json::json!({
                "status": "ok",
                "message": "Memory added",
                "path": path,
                "id": id,
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

async fn start_mcp_stdio() -> Result<()> {
    println!("Xavier2 MCP-stdio server mode");
    println!("This connects Xavier2 to MCP-compatible AI clients");
    println!();
    println!("Configure your MCP client with:");
    println!("  mcpServers: {{");
    println!("    xavier2: {{");
    println!("      command: 'xavier2'");
    println!("      args: ['mcp']");
    println!("    }}");
    println!("  }}");
    println!();
    println!("MCP-stdio implementation pending...");

    Ok(())
}

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
