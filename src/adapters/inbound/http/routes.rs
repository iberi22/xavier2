use axum::{
    extract::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

use crate::adapters::inbound::http::dto::TimeMetricDto;
use crate::agents::unregister_agent_handler;
use crate::coordination::SimpleAgentRegistry;
use crate::ports::inbound::{AgentLifecyclePort, TimeMetricsPort};
use crate::ports::outbound::HealthCheckPort;
use crate::security::SecurityService;
use crate::session::event_mapper::PanelThreadEntry;
use crate::session::types::SessionEvent;
use crate::tasks::session_sync_task::get_last_sync_result;
use crate::verification::auto_verifier::AutoVerifier;

// ─── Module-level TimeMetricsPort (initialized by CLI) ────────────────────────
static TIME_STORE: std::sync::OnceLock<Arc<dyn TimeMetricsPort>> = std::sync::OnceLock::new();

/// Module-level HealthCheckPort (initialized by CLI)
static HEALTH_PORT: std::sync::OnceLock<Arc<dyn HealthCheckPort>> = std::sync::OnceLock::new();

/// Initialize the global time metrics port (call once at startup)
pub fn init_time_store(port: Arc<dyn TimeMetricsPort>) {
    if TIME_STORE.set(port).is_err() {
        tracing::error!("TIME_STORE global already initialized (called init_time_store twice)");
    }
}

/// Initialize the global health check port (call once at startup)
pub fn init_health_port(port: Arc<dyn HealthCheckPort>) {
    if HEALTH_PORT.set(port).is_err() {
        tracing::error!("HEALTH_PORT global already initialized (called init_health_port twice)");
    }
}

// ─── Router ─────────────────────────────────────────────────────────────────

pub fn create_router() -> Router {
    create_router_with_agent_registry(SimpleAgentRegistry::new())
}

pub fn create_router_with_agent_registry(agent_registry: Arc<dyn AgentLifecyclePort>) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route(
            "/xavier2/agents/{id}/unregister",
            post(unregister_agent_handler),
        )
        .route("/xavier2/verify/save", post(verify_save_handler))
        .route("/xavier2/time/metric", post(time_metric_handler))
        .route("/xavier2/events/session", post(session_event_handler))
        .route("/xavier2/sync/check", post(sync_check_handler))
        .with_state(agent_registry)
}

async fn health_handler() -> &'static str {
    "ok"
}

pub async fn session_event_handler(Json(event): Json<SessionEvent>) -> Json<serde_json::Value> {
    let Some(entry) = PanelThreadEntry::from_session_event(&event) else {
        return Json(serde_json::json!({
            "status": "ok",
            "session_id": event.session_id,
            "mapped": false,
        }));
    };

    let security = SecurityService::new();
    let result = security.process_input(&entry.content);

    if !result.allowed {
        return Json(serde_json::json!({
            "status": "blocked",
            "blocked": true,
            "reason": "security_policy_violation",
            "session_id": event.session_id,
            "mapped": false,
            "detection": {
                "is_injection": result.detection.is_injection,
                "confidence": result.detection.confidence,
                "attack_type": format!("{:?}", result.detection.attack_type),
            },
        }));
    }

    Json(serde_json::json!({
        "status": "ok",
        "session_id": event.session_id,
        "mapped": true,
        "content_sanitized": result.sanitized_input.is_some(),
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

    let auth_token = match std::env::var("XAVIER2_TOKEN") {
        Ok(token) => token,
        Err(_) => {
            tracing::error!("XAVIER2_TOKEN is required for verification requests");
            return Json(VerifySaveResponse {
                save_ok: false,
                latency_ms: start.elapsed().as_millis() as u64,
                match_score: 0.0,
            });
        }
    };

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

pub async fn time_metric_handler(Json(payload): Json<TimeMetricDto>) -> Json<TimeMetricResponse> {
    let workspace_id =
        std::env::var("XAVIER2_WORKSPACE_ID").unwrap_or_else(|_| "default".to_string());

    // Try to save via TimeMetricsStore if available
    if let Some(time_store) = TIME_STORE.get() {
        let domain_metric: crate::domain::memory::TimeMetric = payload.clone().into();
        let result = time_store
            .save_time_metric(&domain_metric, &workspace_id)
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
    use axum::{
        body::Body,
        http::{Method, Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use tower::util::ServiceExt;

    use super::create_router_with_agent_registry;
    use crate::coordination::SimpleAgentRegistry;

    fn post_request(uri: &str) -> Request<Body> {
        Request::builder()
            .uri(uri)
            .method(Method::POST)
            .body(Body::empty())
            .expect("build POST request")
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

        let response = create_router_with_agent_registry(registry.clone())
            .oneshot(post_request("/xavier2/agents/agent-delete-1/unregister"))
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
        let response = create_router_with_agent_registry(SimpleAgentRegistry::new())
            .oneshot(post_request("/xavier2/agents/missing-agent/unregister"))
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
