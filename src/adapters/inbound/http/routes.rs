use axum::{
    extract::{Json, Path, State},
    routing::delete,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

use crate::adapters::inbound::http::dto::TimeMetricDto;
use crate::app::state::AppState;
use crate::domain::memory::{
    EvidenceKind, MemoryKind, MemoryNamespace, MemoryProvenance, MemoryRecord,
};
use crate::ports::outbound::HealthCheckPort;
use crate::session::event_mapper::map_to_panel_thread;
use crate::session::types::{SessionEvent, SessionEventType};
use crate::tasks::session_sync_task::get_last_sync_result;
use crate::verification::auto_verifier::AutoVerifier;

/// Module-level HealthCheckPort (initialized by CLI)
static HEALTH_PORT: std::sync::OnceLock<Arc<dyn HealthCheckPort>> = std::sync::OnceLock::new();

/// Initialize the global health check port (call once at startup)
pub fn init_health_port(port: Arc<dyn HealthCheckPort>) {
    HEALTH_PORT.set(port).ok();
}

// ─── Router ─────────────────────────────────────────────────────────────────

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/xavier2/events/session", post(session_event_handler))
        .route(
            "/xavier2/agents/{id}/unregister",
            delete(unregister_agent_handler),
        )
        .route("/xavier2/verify/save", post(verify_save_handler))
        .route("/xavier2/time/metric", post(time_metric_handler))
        .route("/xavier2/sync/check", post(sync_check_handler))
        .with_state(state)
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

pub async fn session_event_handler(
    State(state): State<AppState>,
    Json(payload): Json<SessionEventRequest>,
) -> Json<SessionEventResponse> {
    let event = SessionEvent {
        session_id: payload.session_id.clone(),
        event_type: parse_event_type(&payload.event_type),
        timestamp: chrono::Utc::now(),
        content: Some(payload.content),
        metadata: Some(payload.metadata),
    };

    let mapped_entry = map_to_panel_thread(event);
    let mapped = mapped_entry.is_some();

    if let Some(entry) = mapped_entry {
        let content = serde_json::to_string(&entry).unwrap_or_else(|_| entry.content.clone());
        let record = MemoryRecord {
            id: String::new(),
            content,
            kind: MemoryKind::Context,
            namespace: MemoryNamespace::Global,
            provenance: MemoryProvenance {
                source: format!("sessions/{}/thread", entry.session_id),
                evidence_kind: EvidenceKind::Direct,
                confidence: 1.0,
            },
            embedding: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        if let Err(error) = state.memory.add(record).await {
            tracing::warn!("failed to store session event: {}", error);
        }
    }

    Json(SessionEventResponse {
        status: if mapped { "ok" } else { "ignored" }.to_string(),
        session_id: payload.session_id,
        mapped,
    })
}

async fn unregister_agent_handler(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
) -> Json<serde_json::Value> {
    let success = state.agent_registry.unregister(&agent_id).await;

    Json(serde_json::json!({
        "status": if success { "ok" } else { "error" },
        "agent_id": agent_id,
        "message": if success { "Agent unregistered" } else { "Agent not found or already unregistered" },
    }))
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

pub async fn verify_save_handler(
    Json(payload): Json<VerifySaveRequest>,
) -> Json<VerifySaveResponse> {
    let start = Instant::now();

    let xavier2_url =
        std::env::var("XAVIER2_URL").unwrap_or_else(|_| "http://localhost:8006".to_string());

    // Validate internal URL to prevent SSRF
    if let Err(e) = crate::security::url_validator::validate_internal_url(&xavier2_url) {
        tracing::error!("Internal URL validation failed: {}", e);
        return Json(VerifySaveResponse {
            save_ok: false,
            latency_ms: start.elapsed().as_millis() as u64,
            match_score: 0.0,
        });
    }

    let auth_token = std::env::var("X-CORTEX-TOKEN").unwrap_or_else(|_| "dev-token".to_string());

    let client = reqwest::Client::new();
    let result = AutoVerifier::verify_save(
        &client,
        &xavier2_url,
        &auth_token,
        &payload.path,
        &payload.content,
    )
    .await;

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

pub async fn time_metric_handler(
    State(state): State<AppState>,
    Json(payload): Json<TimeMetricDto>,
) -> Json<TimeMetricResponse> {
    let workspace_id =
        std::env::var("XAVIER2_WORKSPACE_ID").unwrap_or_else(|_| "default".to_string());

    let result = state
        .time_metrics
        .save_time_metric(&payload, &workspace_id)
        .await;
    match result {
        Ok(()) => {
            return Json(TimeMetricResponse {
                status: "saved".to_string(),
                metric_type: payload.metric_type,
                agent_id: payload.agent_id,
            });
        }
        Err(e) => {
            tracing::warn!("TimeMetricsStore save error: {}", e);
        }
    }

    Json(TimeMetricResponse {
        status: "ok".to_string(),
        metric_type: payload.metric_type,
        agent_id: payload.agent_id,
    })
}

// ─── Session Sync Check Endpoint ────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct SyncCheckResponse {
    pub status: String,
    pub lag_ms: u64,
    pub save_ok_rate: f64,
    pub match_score: f64,
    pub active_agents: u64,
    pub timestamp_ms: u64,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub alerts: Vec<String>,
}

pub async fn sync_check_handler() -> Json<SyncCheckResponse> {
    // Return cached sync check results from the SessionSyncTask cron.
    let result = get_last_sync_result();

    Json(SyncCheckResponse {
        status: result.status,
        lag_ms: result.lag_ms,
        save_ok_rate: result.save_ok_rate,
        match_score: result.match_score,
        active_agents: result.active_agents,
        timestamp_ms: result.timestamp_ms,
        alerts: result.alerts,
    })
}

#[cfg(test)]
mod route_tests {
    use async_trait::async_trait;
    use axum::{
        body::Body,
        http::{Method, Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use std::sync::Arc;
    use tower::util::ServiceExt;

    use super::create_router;
    use crate::adapters::inbound::http::dto::TimeMetricDto;
    use crate::app::state::AppState;
    use crate::coordination::SimpleAgentRegistry;
    use crate::domain::memory::{MemoryNamespace, MemoryRecord};
    use crate::ports::inbound::{
        AgentLifecyclePort, InputSecurityPort, MemoryQueryPort, TimeMetricsPort,
    };
    use crate::security::{ProcessResult, SecurityService};

    struct NoopMemory;

    #[async_trait]
    impl MemoryQueryPort for NoopMemory {
        async fn search(
            &self,
            _query: &str,
            _filters: Option<crate::domain::memory::MemoryQueryFilters>,
        ) -> anyhow::Result<Vec<MemoryRecord>> {
            Ok(Vec::new())
        }

        async fn add(&self, _record: MemoryRecord) -> anyhow::Result<String> {
            Ok("memory-id".to_string())
        }

        async fn delete(&self, _id: &str) -> anyhow::Result<Option<MemoryRecord>> {
            Ok(None)
        }

        async fn get(&self, _id: &str) -> anyhow::Result<Option<MemoryRecord>> {
            Ok(None)
        }

        async fn list(
            &self,
            _namespace: MemoryNamespace,
            _limit: usize,
        ) -> anyhow::Result<Vec<MemoryRecord>> {
            Ok(Vec::new())
        }
    }

    struct NoopTimeMetrics;

    #[async_trait]
    impl TimeMetricsPort for NoopTimeMetrics {
        async fn save_time_metric(
            &self,
            _metric: &TimeMetricDto,
            _workspace_id: &str,
        ) -> Result<(), String> {
            Ok(())
        }
    }

    struct TestSecurity(SecurityService);

    impl InputSecurityPort for TestSecurity {
        fn process_input(&self, input: &str) -> ProcessResult {
            self.0.process_input(input)
        }
    }

    fn test_state(agent_registry: Arc<dyn AgentLifecyclePort>) -> AppState {
        AppState {
            memory: Arc::new(NoopMemory),
            time_metrics: Arc::new(NoopTimeMetrics),
            agent_registry,
            security: Arc::new(TestSecurity(SecurityService::new())),
        }
    }

    fn delete(uri: &str) -> Request<Body> {
        Request::builder()
            .uri(uri)
            .method(Method::DELETE)
            .body(Body::empty())
            .expect("build DELETE request")
    }

    #[tokio::test]
    async fn unregister_route_removes_existing_agent() {
        let registry = SimpleAgentRegistry::new();
        registry
            .register(
                "agent-delete-1".to_string(),
                "session-delete-1".to_string(),
                Default::default(),
            )
            .await;

        let response = create_router(test_state(registry.clone()))
            .oneshot(delete("/xavier2/agents/agent-delete-1/unregister"))
            .await
            .expect("request should complete");

        assert_eq!(response.status(), StatusCode::OK);
        let body = response
            .into_body()
            .collect()
            .await
            .expect("collect body")
            .to_bytes();
        let parsed: serde_json::Value =
            serde_json::from_slice(&body).expect("parse unregister response");

        assert_eq!(parsed["status"], "ok");
        assert_eq!(parsed["agent_id"], "agent-delete-1");
        assert_eq!(parsed["message"], "Agent unregistered");
        assert!(registry.get("agent-delete-1").await.is_none());
    }

    #[tokio::test]
    async fn unregister_route_returns_error_for_missing_agent() {
        let response = create_router(test_state(SimpleAgentRegistry::new()))
            .oneshot(delete("/xavier2/agents/missing-agent/unregister"))
            .await
            .expect("request should complete");

        assert_eq!(response.status(), StatusCode::OK);
        let body = response
            .into_body()
            .collect()
            .await
            .expect("collect body")
            .to_bytes();
        let parsed: serde_json::Value =
            serde_json::from_slice(&body).expect("parse unregister response");

        assert_eq!(parsed["status"], "error");
        assert_eq!(parsed["agent_id"], "missing-agent");
        assert_eq!(parsed["message"], "Agent not found or already unregistered");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::session_sync_task::{
        LAST_CHECK_ACTIVE_AGENTS, LAST_CHECK_ALERTS, LAST_CHECK_LAG_MS, LAST_CHECK_MATCH_SCORE,
        LAST_CHECK_SAVE_OK_RATE, LAST_CHECK_STATUS, LAST_CHECK_TIMESTAMP_MS,
    };
    use std::sync::atomic::Ordering;

    #[tokio::test]
    async fn sync_check_handler_uses_cached_sync_result() {
        LAST_CHECK_LAG_MS.store(42_000, Ordering::SeqCst);
        LAST_CHECK_TIMESTAMP_MS.store(1_234_567, Ordering::SeqCst);
        LAST_CHECK_ACTIVE_AGENTS.store(7, Ordering::SeqCst);
        *LAST_CHECK_SAVE_OK_RATE.lock().unwrap() = 0.90;
        *LAST_CHECK_MATCH_SCORE.lock().unwrap() = 0.88;
        *LAST_CHECK_STATUS.lock().unwrap() = "alert".to_string();
        *LAST_CHECK_ALERTS.lock().unwrap() = vec![
            "Index lag 42000ms exceeds threshold 30000ms".to_string(),
            "Save ok rate 90.0% below threshold 95.0%".to_string(),
        ];

        let Json(response) = sync_check_handler().await;

        assert_eq!(response.status, "alert");
        assert_eq!(response.lag_ms, 42_000);
        assert_eq!(response.save_ok_rate, 0.90);
        assert_eq!(response.match_score, 0.88);
        assert_eq!(response.active_agents, 7);
        assert_eq!(response.timestamp_ms, 1_234_567);
        assert_eq!(response.alerts.len(), 2);
    }
}
