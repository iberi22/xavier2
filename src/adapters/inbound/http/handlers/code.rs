use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::Deserialize;
use crate::adapters::inbound::http::state::check_auth;
use crate::adapters::inbound::http::AppState;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct CodeScanPayload {
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CodeFindPayload {
    #[serde(default)]
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub pattern: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CodeContextPayload {
    #[serde(default)]
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default = "default_token_budget")]
    pub budget_tokens: usize,
    #[serde(default)]
    pub kind: Option<String>,
}

fn default_token_budget() -> usize {
    800
}

fn default_limit() -> usize {
    10
}

pub async fn code_scan_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<CodeScanPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    check_auth(&headers, &state)?;
    let requested_path = payload.path.as_deref().unwrap_or(".");

    // Security scan on path
    let sec_result = match state.security.process_input(requested_path).await {
        Ok(res) => res,
        Err(e) => return Ok(Json(serde_json::json!({ "status": "error", "message": e.to_string() }))),
    };

    if !sec_result.allowed {
        return Ok(Json(serde_json::json!({
            "status": "blocked",
            "reason": "security_policy_violation",
            "detection": {
                "is_injection": sec_result.is_injection,
                "confidence": sec_result.detection_confidence,
                "attack_type": sec_result.attack_type,
            }
        })));
    }

    if requested_path.contains("..") {
        return Ok(Json(serde_json::json!({
            "status": "error",
            "message": "path traversal not allowed",
            "indexed_files": 0,
        })));
    }

    let _limit = payload.path.is_none(); // placeholder to avoid clippy warning if any

    match state.code_indexer.index(Path::new(requested_path)).await {
        Ok(stats) => Ok(Json(serde_json::json!({
            "status": "ok",
            "indexed_files": stats.total_files,
            "indexed_symbols": stats.total_symbols,
            "indexed_imports": stats.total_imports,
            "duration_ms": stats.duration_ms,
            "paths": [requested_path],
            "languages": stats.languages,
        }))),
        Err(error) => Ok(Json(serde_json::json!({
            "status": "error",
            "message": error.to_string(),
        }))),
    }
}

pub async fn code_find_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<CodeFindPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    check_auth(&headers, &state)?;
    let sec_result = match state.security.process_input(&payload.query).await {
        Ok(res) => res,
        Err(e) => return Ok(Json(serde_json::json!({ "status": "error", "message": e.to_string() }))),
    };

    if !sec_result.allowed {
        return Ok(Json(serde_json::json!({
            "status": "blocked",
            "reason": "security_policy_violation",
            "detection": {
                "is_injection": sec_result.is_injection,
                "confidence": sec_result.detection_confidence,
                "attack_type": sec_result.attack_type,
            }
        })));
    }

    let query = sec_result.sanitized_input.as_deref().unwrap_or(&sec_result.original_input).to_string();
    let limit = payload.limit.clamp(1, 100);

    let symbols = code_find_symbols(
        &state.code_query,
        &query,
        payload.kind.as_deref(),
        payload.pattern.as_deref(),
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

    Ok(Json(serde_json::json!({
        "status": "ok",
        "query": query,
        "count": results.len(),
        "results": results,
    })))
}

pub async fn code_stats_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    check_auth(&headers, &state)?;
    match state.code_db.stats() {
        Ok(stats) => Ok(Json(serde_json::json!({
            "status": "ok",
            "total_files": stats.total_files,
            "total_symbols": stats.total_symbols,
            "total_imports": stats.total_imports,
            "languages": stats.languages,
        }))),
        Err(error) => Ok(Json(serde_json::json!({
            "status": "error",
            "message": error.to_string(),
        }))),
    }
}

pub async fn code_context_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<CodeContextPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    check_auth(&headers, &state)?;
    let sec_result = match state.security.process_input(&payload.query).await {
        Ok(res) => res,
        Err(e) => return Ok(Json(serde_json::json!({ "status": "error", "message": e.to_string() }))),
    };

    if !sec_result.allowed {
        return Ok(Json(serde_json::json!({
            "status": "blocked",
            "reason": "security_policy_violation",
            "detection": {
                "is_injection": sec_result.is_injection,
                "confidence": sec_result.detection_confidence,
                "attack_type": sec_result.attack_type,
            }
        })));
    }

    let limit = payload.limit.clamp(1, 100);
    let kind_limit = if payload.query.trim().is_empty() { limit } else { 10_000 };
    let budget_tokens = payload.budget_tokens.clamp(100, 8000);

    let mut symbols = if let Some(kind) = payload.kind.as_deref() {
        match kind.to_ascii_lowercase().as_str() {
            "function" | "fn" => state.code_query.functions(kind_limit).unwrap_or_default(),
            "struct" => state.code_query.structs(kind_limit).unwrap_or_default(),
            "class" => state.code_query.classes(kind_limit).unwrap_or_default(),
            "enum" => state.code_query.enums(kind_limit).unwrap_or_default(),
            _ => state.code_query.search(&payload.query, limit).map(|result| result.symbols).unwrap_or_default(),
        }
    } else {
        state.code_query.search(&payload.query, limit).map(|result| result.symbols).unwrap_or_default()
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
        let estimated = (compact.to_string().len() / 4).max(1);
        if used_tokens + estimated > budget_tokens && !context.is_empty() {
            break;
        }
        used_tokens += estimated;
        context.push(compact);
    }

    Ok(Json(serde_json::json!({
        "status": "ok",
        "query": payload.query,
        "budget_tokens": budget_tokens,
        "estimated_tokens": used_tokens,
        "count": context.len(),
        "context": context,
    })))
}

// Helper functions (copied from cli.rs)

fn code_find_symbols(
    code_query: &code_graph::query::QueryEngine,
    query: &str,
    kind: Option<&str>,
    pattern: Option<&str>,
    limit: usize,
) -> Vec<code_graph::types::Symbol> {
    let limit = limit.clamp(1, 100);
    let broad_limit = if query.trim().is_empty() { limit } else { 10_000 };

    let mut symbols = if let Some(pattern) = pattern.filter(|p| !p.trim().is_empty()) {
        if is_supported_code_pattern(pattern) {
            code_query.search_by_pattern(pattern, broad_limit).unwrap_or_default()
        } else {
            search_code_symbols_with_fallback(code_query, pattern, broad_limit)
        }
    } else if let Some(kind) = kind.filter(|k| !k.trim().is_empty()) {
        match kind.to_ascii_lowercase().as_str() {
            "function" | "fn" => code_query.functions(broad_limit).unwrap_or_default(),
            "struct" => code_query.structs(broad_limit).unwrap_or_default(),
            "class" => code_query.classes(broad_limit).unwrap_or_default(),
            "enum" => code_query.enums(broad_limit).unwrap_or_default(),
            _ => search_code_symbols_with_fallback(code_query, query, broad_limit),
        }
    } else {
        search_code_symbols_with_fallback(code_query, query, broad_limit)
    };

    filter_symbols_by_query(&mut symbols, query);
    symbols.truncate(limit);
    symbols
}

fn is_supported_code_pattern(pattern: &str) -> bool {
    matches!(
        pattern,
        "function_call" | "function_definition" | "struct_definition" | "struct" | "class_definition" | "class" | "enum_definition" | "enum" | "module_definition" | "module" | "import" | "use_statement"
    )
}

fn search_code_symbols_with_fallback(
    code_query: &code_graph::query::QueryEngine,
    query: &str,
    limit: usize,
) -> Vec<code_graph::types::Symbol> {
    let query = query.trim();
    let mut symbols = code_query.search(query, limit).map(|result| result.symbols).unwrap_or_default();

    if symbols.is_empty() {
        if let Some(token) = best_symbol_query_token(query) {
            if token != query {
                symbols = code_query.search(token, limit).map(|result| result.symbols).unwrap_or_default();
            }
        }
    }
    symbols
}

fn best_symbol_query_token(query: &str) -> Option<&str> {
    query
        .split(|ch: char| !(ch == '_' || ch.is_ascii_alphanumeric()))
        .filter(|token| !token.is_empty())
        .filter(|token| {
            !matches!(
                token.to_ascii_lowercase().as_str(),
                "fn" | "function" | "struct" | "class" | "enum" | "async" | "pub"
            )
        })
        .max_by_key(|token| token.len())
}

fn filter_symbols_by_query(symbols: &mut Vec<code_graph::types::Symbol>, query: &str) {
    let query = query.trim().to_ascii_lowercase();
    if query.is_empty() { return; }

    symbols.retain(|symbol| {
        symbol.name.to_ascii_lowercase().contains(&query)
            || symbol.signature.as_deref().unwrap_or_default().to_ascii_lowercase().contains(&query)
            || symbol.file_path.to_ascii_lowercase().contains(&query)
    });
}
