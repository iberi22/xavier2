use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::Deserialize;
use crate::adapters::inbound::http::state::check_auth;
use crate::adapters::inbound::http::AppState;
use crate::domain::memory::{MemoryQueryFilters, MemoryRecord as DomainMemoryRecord};
use tracing::info;

#[derive(Debug, Deserialize)]
pub struct SearchPayload {
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub filters: Option<MemoryQueryFilters>,
}

#[derive(Debug, Deserialize)]
pub struct AddPayload {
    pub content: String,
    pub path: String,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct MemoryQueryPayload {
    pub query: String,
    pub limit: Option<usize>,
    pub filters: Option<serde_json::Value>,
}

fn default_limit() -> usize {
    10
}

pub async fn search_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<SearchPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    check_auth(&headers, &state)?;
    // Security scan on query before searching
    let sec_result = match state.security.process_input(&payload.query).await {
        Ok(res) => res,
        Err(e) => {
            return Ok(Json(serde_json::json!({
                "results": [],
                "query": payload.query,
                "count": 0,
                "error": format!("Security scan error: {}", e),
                "workspace_id": state.workspace_id,
            })));
        }
    };

    if !sec_result.allowed {
        info!(
            "Search blocked by security: injection detected (confidence={})",
            sec_result.detection_confidence
        );
        return Ok(Json(serde_json::json!({
            "results": <Vec<serde_json::Value>>::new(),
            "query": payload.query,
            "count": 0,
            "blocked": true,
            "reason": "security_policy_violation",
            "detection": {
                "is_injection": sec_result.is_injection,
                "confidence": sec_result.detection_confidence,
                "attack_type": sec_result.attack_type,
            },
            "workspace_id": state.workspace_id,
        })));
    }

    let effective_query = sec_result.sanitized_input.as_deref().unwrap_or(&sec_result.original_input);
    let limit = payload.limit.clamp(1, 100);
    info!("Search request: query={}, limit={}", effective_query, limit);

    match state.memory.search(effective_query, payload.filters).await {
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

            Ok(Json(serde_json::json!({
                "status": "ok",
                "query": payload.query,
                "count": documents.len(),
                "results": documents,
                "workspace_id": state.workspace_id,
            })))
        }
        Err(e) => {
            info!("Search error: {}", e);
            Ok(Json(serde_json::json!({
                "status": "error",
                "message": e.to_string(),
            })))
        }
    }
}

pub async fn add_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<AddPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    check_auth(&headers, &state)?;
    let record = DomainMemoryRecord {
        id: String::new(),
        workspace_id: state.workspace_id.clone(),
        path: payload.path.clone(),
        content: payload.content.clone(),
        metadata: payload.metadata.clone(),
        embedding: vec![],
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        revision: 1,
        primary: true,
        parent_id: None,
        cluster_id: None,
        level: crate::memory::schema::MemoryLevel::Raw,
        relation: None,
        revisions: vec![],
    };

    match state.memory.add(record).await {
        Ok(id) => Ok(Json(serde_json::json!({
            "status": "ok",
            "id": id,
            "path": payload.path,
            "workspace_id": state.workspace_id,
        }))),
        Err(e) => Ok(Json(serde_json::json!({
            "status": "error",
            "message": e.to_string(),
        }))),
    }
}

pub async fn stats_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    // Note: MemoryQueryPort doesn't have stats() yet, might need to add it or use storage directly
    // For now returning placeholder or calling list
    Json(serde_json::json!({
        "status": "ok",
        "workspace_id": state.workspace_id,
        "message": "Memory stats not yet implemented in port interface",
    }))
}

pub async fn memory_query_handler(
    State(state): State<AppState>,
    Json(payload): Json<MemoryQueryPayload>,
) -> Json<serde_json::Value> {
    // Security scan on query
    let sec_result = match state.security.process_input(&payload.query).await {
        Ok(res) => res,
        Err(e) => {
            return Json(serde_json::json!({
                "status": "error",
                "message": format!("Security scan error: {}", e),
            }));
        }
    };

    if !sec_result.allowed {
        return Json(serde_json::json!({
            "status": "blocked",
            "reason": "security_policy_violation",
            "detection": {
                "is_injection": sec_result.is_injection,
                "confidence": sec_result.detection_confidence,
                "attack_type": sec_result.attack_type,
            }
        }));
    }

    let _limit = payload.limit.unwrap_or(10).clamp(1, 100);
    let effective_query = sec_result.sanitized_input.as_deref().unwrap_or(&sec_result.original_input);

    match state.memory.search(effective_query, None).await {
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

            Json(serde_json::json!({
                "status": "ok",
                "query": payload.query,
                "count": documents.len(),
                "results": documents,
                "workspace_id": state.workspace_id,
            }))
        }
        Err(e) => {
            info!("Memory query error: {}", e);
            Json(serde_json::json!({
                "status": "error",
                "message": e.to_string(),
            }))
        }
    }
}
