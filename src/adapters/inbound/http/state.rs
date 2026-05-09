use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use serde_json::Value;

use crate::ports::inbound::{
    AgentLifecyclePort, HealthPort, InputSecurityPort, MemoryQueryPort, SecurityScanPort,
    SessionPort, SessionSyncPort, TimeMetricsPort, VerificationPort,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub memory: Arc<dyn MemoryQueryPort>,
    pub security: Arc<dyn InputSecurityPort>,
    pub security_scan: Arc<dyn SecurityScanPort>,
    pub time_metrics: Arc<dyn TimeMetricsPort>,
    pub agent_lifecycle: Arc<dyn AgentLifecyclePort>,
    pub health: Arc<dyn HealthPort>,
    pub verification: Arc<dyn VerificationPort>,
    pub session_sync: Arc<dyn SessionSyncPort>,
    pub session: Arc<dyn SessionPort>,
    pub workspace_id: String,
    pub auth_token: String,

    // Code graph components (to be moved to ports in a future phase)
    pub code_db: Arc<code_graph::db::CodeGraphDB>,
    pub code_indexer: Arc<code_graph::indexer::Indexer>,
    pub code_query: Arc<code_graph::query::QueryEngine>,
}

/// Check that the `X-Xavier-Token` header matches the configured auth token.
pub fn check_auth(headers: &HeaderMap, state: &AppState) -> Result<(), (StatusCode, Json<Value>)> {
    match headers.get("X-Xavier-Token").and_then(|v| v.to_str().ok()) {
        Some(token) if token == state.auth_token => Ok(()),
        _ => Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "status": "error",
                "message": "Unauthorized",
            })),
        )),
    }
}
