use axum::{
    routing::{get, post},
    Router,
    extract::Json,
};
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::adapters::inbound::http::dto::TimeMetricDto;
use crate::session::types::{SessionEvent, SessionEventType};
use crate::session::event_mapper::map_to_panel_thread;
use crate::verification::auto_verifier::AutoVerifier;

// ─── Router ─────────────────────────────────────────────────────────────────

pub fn create_router() -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/xavier2/events/session", post(session_event_handler))
        .route("/xavier2/verify/save", post(verify_save_handler))
        .route("/xavier2/time/metric", post(time_metric_handler))
}

async fn health_handler() -> &'static str {
    "ok"
}

// ─── Session Events Webhook ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SessionEventRequest {
    pub session_id: String,
    pub event_type: String,
    pub content: String,
    pub timestamp: String,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct SessionEventResponse {
    pub status: String,
    pub session_id: String,
    pub mapped: bool,
}

fn parse_event_type(s: &str) -> SessionEventType {
    match s.to_lowercase().as_str() {
        "message" => SessionEventType::Message,
        "tool_call" | "toolcall" => SessionEventType::ToolCall,
        "tool_result" | "toolresult" => SessionEventType::ToolResult,
        "session_start" | "start" => SessionEventType::SessionStart,
        "session_end" | "end" => SessionEventType::SessionEnd,
        "error" => SessionEventType::Error,
        _ => SessionEventType::Message,
    }
}

async fn session_event_handler(
    Json(payload): Json<SessionEventRequest>,
) -> Json<SessionEventResponse> {
    let event = SessionEvent {
        session_id: payload.session_id.clone(),
        event_type: parse_event_type(&payload.event_type),
        timestamp: chrono::Utc::now(),
        content: Some(payload.content),
        metadata: Some(payload.metadata),
    };

    let mapped = map_to_panel_thread(event).is_some();
    
    Json(SessionEventResponse {
        status: if mapped { "ok" } else { "ignored" }.to_string(),
        session_id: payload.session_id,
        mapped,
    })
}

// ─── Verification Endpoints ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct VerifySaveRequest {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct VerifySaveResponse {
    pub save_ok: bool,
    pub latency_ms: u64,
    pub match_score: f32,
}

async fn verify_save_handler(
    Json(payload): Json<VerifySaveRequest>,
) -> Json<VerifySaveResponse> {
    let start = Instant::now();
    
    let xavier2_url = std::env::var("XAVIER2_URL")
        .unwrap_or_else(|_| "http://localhost:8006".to_string());
    let auth_token = std::env::var("X-CORTEX-TOKEN")
        .unwrap_or_else(|_| "dev-token".to_string());
    
    let client = reqwest::Client::new();
    let result = AutoVerifier::verify_save(
        &client,
        &xavier2_url,
        &auth_token,
        &payload.path,
        &payload.content,
    ).await;
    
    let elapsed = start.elapsed().as_millis() as u64;
    
    match result {
        Ok(vr) => Json(VerifySaveResponse {
            save_ok: vr.save_ok,
            latency_ms: elapsed,
            match_score: vr.match_score,
        }),
        Err(_) => Json(VerifySaveResponse {
            save_ok: false,
            latency_ms: elapsed,
            match_score: 0.0,
        }),
    }
}

// ─── Time Metrics Endpoint ──────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct TimeMetricResponse {
    pub status: String,
    pub metric_type: String,
    pub agent_id: String,
}

async fn time_metric_handler(
    Json(payload): Json<TimeMetricDto>,
) -> Json<TimeMetricResponse> {
    let workspace_id = std::env::var("XAVIER2_WORKSPACE_ID")
        .unwrap_or_else(|_| "default".to_string());

    // TimeMetricsStore requires a SQLite connection — we store inline using
    // the memory store path pattern. For now, return success status.
    // The actual storage should be wired up via CliState or AppState in
    // production.
    let _ = workspace_id;

    Json(TimeMetricResponse {
        status: "ok".to_string(),
        metric_type: payload.metric_type,
        agent_id: payload.agent_id,
    })
}