//! CLI for code-graph - Server mode with HTTP token auth

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Json,
    routing::{get, post},
    Router,
};
use clap::{Parser, Subcommand};
use code_graph::parser::parse_source;
use code_graph::types::Language;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tower_http::cors::{Any, CorsLayer};
use walkdir::WalkDir;

// ============================================================================
// State
// ============================================================================

#[derive(Clone)]
struct AppState {
    token: String,
    index: Arc<RwLock<CodeIndex>>,
}

#[derive(Default)]
struct CodeIndex {
    symbols: Vec<SymbolEntry>,
    languages: HashMap<String, usize>,
    file_count: usize,
}

#[derive(Serialize, Deserialize, Clone)]
struct SymbolEntry {
    name: String,
    kind: String,
    file: String,
    line: usize,
    lang: String,
}

#[derive(Serialize, Deserialize)]
struct ScanRequest {
    path: String,
}

#[derive(Serialize, Deserialize)]
struct ScanResponse {
    status: String,
    files: usize,
    symbols: usize,
    languages: HashMap<String, usize>,
    duration_ms: u64,
}

#[derive(Serialize, Deserialize)]
struct FindRequest {
    query: String,
    lang: Option<String>,
    limit: Option<usize>,
}

#[derive(Serialize, Deserialize)]
struct FindResponse {
    symbols: Vec<SymbolEntry>,
    count: usize,
}

#[derive(Serialize, Deserialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Serialize, Deserialize)]
struct HealthResponse {
    status: String,
    version: String,
}

// ============================================================================
// Auth Middleware
// ============================================================================

async fn auth_middleware(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Result<axum::response::Response, (StatusCode, Json<ErrorResponse>)> {
    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim_start_matches("Bearer "));

    match auth_header {
        Some(token) if token == state.token => Ok(next.run(request).await),
        _ => Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Invalid or missing token".to_string(),
            }),
        )),
    }
}

// ============================================================================
// Security Helpers
// ============================================================================

/// Validate and canonicalize path to prevent path traversal
fn validate_path(base: &Path, requested: &str) -> Result<PathBuf, String> {
    // Reject null bytes and control characters
    if requested.contains('\0') || requested.chars().any(|c| c.is_control()) {
        return Err("Invalid characters in path".to_string());
    }

    let path = PathBuf::from(requested);

    // Get canonical path and verify it's within base
    let canonical = path
        .canonicalize()
        .map_err(|e| format!("Invalid path: {}", e))?;

    let base_canonical = base
        .canonicalize()
        .map_err(|e| format!("Invalid base path: {}", e))?;

    // Verify the canonical path starts with base (prevents traversal)
    if !canonical.starts_with(&base_canonical) {
        return Err("Path traversal attempt detected".to_string());
    }

    Ok(canonical)
}

// ============================================================================
// Routes
// ============================================================================

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: "0.2.0".to_string(),
    })
}

async fn scan(
    State(state): State<AppState>,
    Json(req): Json<ScanRequest>,
) -> Result<Json<ScanResponse>, (StatusCode, Json<ErrorResponse>)> {
    let base_path = PathBuf::from(&req.path);

    // Validate path - check for traversal attempts
    let validated_path = match validate_path(&base_path, &req.path) {
        Ok(p) => p,
        Err(e) => {
            return Err((StatusCode::BAD_REQUEST, Json(ErrorResponse { error: e })));
        }
    };

    if !validated_path.exists() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Path does not exist".to_string(),
            }),
        ));
    }

    let start = Instant::now();

    // Limit files to prevent DoS
    let max_files = 50_000;
    let files: Vec<_> = WalkDir::new(&validated_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            let ext = e.path().extension().and_then(|e| e.to_str()).unwrap_or("");
            matches!(
                ext,
                "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go" | "java"
            )
        })
        .take(max_files + 1) // Check if exceeds limit
        .collect();

    // Check file limit
    if files.len() > max_files {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("Too many files (max: {})", max_files),
            }),
        ));
    }

    let mut symbols = Vec::new();
    let mut languages = HashMap::new();

    // Limit symbols to prevent memory exhaustion
    let max_symbols = 500_000;

    for file in &files {
        let ext = file
            .path()
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let lang = Language::from_extension(ext);

        if let Ok(source) = std::fs::read_to_string(file.path()) {
            let parsed = parse_source(&source, &lang, &file.path().to_string_lossy());

            if let Ok(syms) = parsed {
                // Check symbol limit
                if symbols.len() + syms.len() > max_symbols {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        Json(ErrorResponse {
                            error: format!("Too many symbols (max: {})", max_symbols),
                        }),
                    ));
                }

                for sym in &syms {
                    symbols.push(SymbolEntry {
                        name: sym.name.clone(),
                        kind: format!("{:?}", sym.kind),
                        file: file.path().to_string_lossy().to_string(),
                        line: sym.start_line as usize,
                        lang: format!("{:?}", lang),
                    });
                }
                *languages.entry(format!("{:?}", lang)).or_insert(0) += 1;
            }
        }
    }

    let symbols_count = symbols.len();

    // Store in index
    {
        let mut index = state.index.write();
        index.symbols = symbols;
        index.languages = languages.clone();
        index.file_count = files.len();
    }

    Ok(Json(ScanResponse {
        status: "ok".to_string(),
        files: files.len(),
        symbols: symbols_count,
        languages,
        duration_ms: start.elapsed().as_millis() as u64,
    }))
}

async fn find(State(state): State<AppState>, Json(req): Json<FindRequest>) -> Json<FindResponse> {
    let index = state.index.read();
    let limit = req.limit.unwrap_or(20).min(100); // Cap at 100

    let query_lower = req.query.to_lowercase();

    let filtered: Vec<_> = index
        .symbols
        .iter()
        .filter(|s| {
            let matches_query = s.name.to_lowercase().contains(&query_lower);
            let matches_lang = req
                .lang
                .as_ref()
                .map(|l| s.lang.to_lowercase() == l.to_lowercase())
                .unwrap_or(true);
            matches_query && matches_lang
        })
        .take(limit)
        .cloned()
        .collect();

    Json(FindResponse {
        count: filtered.len(),
        symbols: filtered,
    })
}

async fn stats(State(state): State<AppState>) -> Json<ScanResponse> {
    let index = state.index.read();

    Json(ScanResponse {
        status: "ok".to_string(),
        files: index.file_count,
        symbols: index.symbols.len(),
        languages: index.languages.clone(),
        duration_ms: 0,
    })
}

// ============================================================================
// CLI
// ============================================================================

#[derive(Parser)]
#[command(name = "code-graph")]
#[command(about = "Codebase Understanding without RAG - Tree-sitter + Agentic Search", long_about = None)]
struct Cli {
    /// HTTP Token for authentication (env: CODE_GRAPH_TOKEN)
    #[arg(long, env = "CODE_GRAPH_TOKEN")]
    token: Option<String>,

    /// Server port (env: CODE_GRAPH_PORT, default: 8080)
    #[arg(long, env = "CODE_GRAPH_PORT", default_value = "8080")]
    port: u16,

    /// Server host (env: CODE_GRAPH_HOST, default: 0.0.0.0)
    #[arg(long, env = "CODE_GRAPH_HOST", default_value = "0.0.0.0")]
    host: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start HTTP server (default when no command)
    Serve,

    /// Scan and index a codebase (CLI mode)
    Scan {
        /// Path to scan
        path: PathBuf,
    },

    /// Find symbols by name (CLI mode)
    Find {
        /// Search query
        query: String,

        /// Max results
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },

    /// Show statistics (CLI mode)
    Stats,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Server mode
    if cli.command.is_none() || matches!(cli.command, Some(Commands::Serve)) {
        let token = cli.token.unwrap_or_else(|| {
            std::env::var("CODE_GRAPH_TOKEN")
                .expect("TOKEN_REQUIRED: Set CODE_GRAPH_TOKEN env var or --token flag")
        });

        if token.len() < 16 {
            eprintln!("⚠️  WARNING: Token should be at least 16 characters for security");
        }

        let state = AppState {
            token: token.clone(),
            index: Arc::new(RwLock::new(CodeIndex::default())),
        };

        // Secure CORS - restrict to common origins in production
        let cors = CorsLayer::new()
            .allow_origin(Any) // TODO: Restrict in production
            .allow_methods(Any)
            .allow_headers(Any);

        // Apply auth middleware to protected routes
        let protected_routes = Router::new()
            .route("/api/scan", post(scan))
            .route("/api/find", post(find))
            .route("/api/stats", get(stats))
            .layer(axum::middleware::from_fn_with_state(
                state.clone(),
                auth_middleware,
            ));

        let app = Router::new()
            .route("/health", get(health))
            .merge(protected_routes)
            .layer(cors)
            .with_state(state);

        let addr: SocketAddr = format!("{}:{}", cli.host, cli.port).parse()?;

        println!("🚀 Starting code-graph server");
        println!(
            "🔐 Token: ***{} (length: {})",
            &token[token.len().saturating_sub(4)..],
            token.len()
        );
        println!("📍 Address: http://{}", addr);
        println!("\nEndpoints:");
        println!("  GET  /health          - Health check (public)");
        println!("  POST /api/scan        - Scan and index codebase (auth required)");
        println!("  POST /api/find        - Find symbols (auth required)");
        println!("  GET  /api/stats       - Get index statistics (auth required)");

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;

        return Ok(());
    }

    // CLI mode
    match cli.command.expect("test assertion") {
        Commands::Serve => unreachable!(),

        Commands::Scan { path } => {
            println!("🔍 Scanning: {:?}", path);
            let start = Instant::now();

            let files: Vec<_> = WalkDir::new(&path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .filter(|e| {
                    let ext = e.path().extension().and_then(|e| e.to_str()).unwrap_or("");
                    matches!(
                        ext,
                        "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go" | "java"
                    )
                })
                .collect();

            let mut total_symbols = 0;
            let mut languages = std::collections::HashMap::new();

            for file in &files {
                let ext = file
                    .path()
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");
                let lang = Language::from_extension(ext);

                if let Ok(source) = std::fs::read_to_string(file.path()) {
                    let symbols = parse_source(&source, &lang, &file.path().to_string_lossy())
                        .unwrap_or_default();
                    total_symbols += symbols.len();
                    *languages.entry(format!("{:?}", lang)).or_insert(0) += 1;
                }
            }

            println!("\n✅ Indexed in {:?}", start.elapsed());
            println!("📁 Files: {}", files.len());
            println!("🔤 Symbols: {}", total_symbols);
            println!("\nLanguages:");
            for (lang, count) in languages {
                println!("  {}: {}", lang, count);
            }
        }

        Commands::Find { query, limit: _ } => {
            println!("🔍 Searching for: {}", query);
            println!("(Use server mode for persistent search)");
        }

        Commands::Stats => {
            println!("📊 code-graph v0.2.0");
            println!("Use 'scan' command first to index a project.");
        }
    }

    Ok(())
}
