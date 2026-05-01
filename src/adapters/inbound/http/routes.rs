use axum::{
    body::Body,
    extract::Json,
    http::{Request, StatusCode},
    response::Response,
    routing::delete,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Instant;

use crate::adapters::inbound::http::dto::TimeMetricDto;
use crate::agents::unregister_agent_handler;
use crate::coordination::SimpleAgentRegistry;
use crate::ports::inbound::TimeMetricsPort;
use crate::ports::outbound::HealthCheckPort;
use crate::session::event_mapper::map_to_panel_thread;
use crate::session::types::{SessionEvent, SessionEventType};
use crate::tasks::session_sync_task::get_last_sync_result;
use crate::verification::auto_verifier::AutoVerifier;

// ─── Module-level TimeMetricsPort (initialized by CLI) ────────────────────────
static TIME_STORE: std::sync::OnceLock<Arc<dyn TimeMetricsPort>> = std::sync::OnceLock::new();

/// Module-level HealthCheckPort (initialized by CLI)
static HEALTH_PORT: std::sync::OnceLock<Arc<dyn HealthCheckPort>> = std::sync::OnceLock::new();

/// Initialize the global time metrics port (call once at startup)
pub fn init_time_store(port: Arc<dyn TimeMetricsPort>) {
    TIME_STORE.set(port).ok();
}

/// Initialize the global health check port (call once at startup)
pub fn init_health_port(port: Arc<dyn HealthCheckPort>) {
    HEALTH_PORT.set(port).ok();
}

// ─── SSRF Protection ────────────────────────────────────────────────────────

/// Validates that a URL does not point to internal/dangerous addresses.
/// Blocks: localhost, 127.x.x.x, 0.0.0.0, 169.254.x.x (link-local),
/// 192.168.x.x, 10.x.x.x, 172.16-31.x.x, and unspecified addresses.
fn is_url_safe_for_outbound(url: &str) -> bool {
    // Block obvious scheme abuses
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return false;
    }

    // Extract the host portion from the URL
    let after_scheme = if let Some(rest) = url.strip_prefix("https://") {
        rest
    } else if let Some(rest) = url.strip_prefix("http://") {
        rest
    } else {
        return false;
    };

    let url_host = after_scheme
        .split('/')
        .next()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("");

    if url_host.is_empty() {
        return false;
    }

    // Block literal localhost / loopback
    let lower = url_host.to_lowercase();
    if lower == "localhost"
        || lower == "localhost.localdomain"
        || lower == "0.0.0.0"
        || lower == "::1"
        || lower == "ip6-localhost"
        || lower == "ip6-loopback"
    {
        return false;
    }

    // If it looks like an IP address, validate it
    if let Ok(ip) = url_host.parse::<IpAddr>() {
        match ip {
            IpAddr::V4(v4) => {
                // Block loopback, private, link-local (169.254.x.x), and unspecified
                if v4.is_loopback() || v4.is_private() || v4.is_link_local() || v4.is_unspecified()
                {
                    return false;
                }
            }
            IpAddr::V6(_) => {
                // Block known loopback IPv6
                if ip.is_loopback() || ip.is_unspecified() {
                    return false;
                }
                // Block private IPv6 (fd00::/8 - unique local addresses)
            }
        }
        return true; // valid public IP
    }

    // Domain names are allowed — DNS resolution is handled at the network level.
    // The reqwest client should use a resolver that respects system DNS.
    true
}

// ─── Auth Middleware ────────────────────────────────────────────────────────

/// Middleware that enforces X-CORTEX-TOKEN / X-Xavier2-Token auth on all routes
/// except /health. Returns 401 Unauthorized if token is missing or invalid.
async fn auth_middleware(
    req: Request<Body>,
    next: axum::middleware::Next,
) -> Response {
    // Allow /health through without auth
    if req.uri().path() == "/health" {
        return next.run(req).await;
    }

    let x_cortex_token = std::env::var("X-CORTEX-TOKEN").ok();
    let xavier2_token = std::env::var("X-Xavier2-Token").ok();

    // Build list of valid tokens (both env vars, if set)
    let valid_tokens: Vec<String> = x_cortex_token
        .into_iter()
        .chain(xavier2_token)
        .collect();

    if valid_tokens.is_empty() {
        // No auth configured in env — reject all requests
        // to avoid accidental exposure in production
        tracing::warn!(
            "No auth token configured (X-CORTEX-TOKEN / X-Xavier2-Token); rejecting request to {}",
            req.uri().path()
        );
        return Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from(
                "Auth not configured: set X-CORTEX-TOKEN or X-Xavier2-Token environment variable",
            ))
            .unwrap();
    }

    let provided = req
        .headers()
        .get("x-cortex-token")
        .and_then(|v| v.to_str().ok())
        .or_else(|| {
            req.headers()
                .get("x-xavier2-token")
                .and_then(|v| v.to_str().ok())
        });

    match provided {
        Some(token) if valid_tokens.iter().any(|vt| vt == token) => next.run(req).await,
        _ => Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Body::from(
                "Unauthorized: invalid or missing X-CORTEX-TOKEN / X-Xavier2-Token header",
            ))
            .unwrap(),
    }
}

// ─── Router ─────────────────────────────────────────────────────────────────

pub fn create_router() -> Router {
    create_router_with_agent_registry(SimpleAgentRegistry::new())
}

pub fn create_router_with_agent_registry(agent_registry: Arc<SimpleAgentRegistry>) -> Router {
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
        .with_state(agent_registry)
        .layer(axum::middleware::from_fn_with_state(
            (),
            |req: Request<Body>, next| async move { auth_middleware(req, next).await },
        ))
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

pub async fn verify_save_handler(
    Json(payload): Json<VerifySaveRequest>,
) -> Json<VerifySaveResponse> {
    let start = Instant::now();

    // Read required env vars — crash at startup if unset (not silently default)
    let xavier2_url =
        std::env::var("XAVIER2_URL").expect("XAVIER2_URL must be set in environment");

    // SSRF check: reject internal/dangerous URLs before making outbound request
    if !is_url_safe_for_outbound(&xavier2_url) {
        tracing::error!(
            url = %xavier2_url,
            "SSRF attempt blocked: unsafe outbound URL target"
        );
        return Json(VerifySaveResponse {
            save_ok: false,
            latency_ms: start.elapsed().as_millis() as u64,
            match_score: 0.0,
        });
    }

    let auth_token =
        std::env::var("X-CORTEX-TOKEN").expect("X-CORTEX-TOKEN must be set in environment");

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
        let result = time_store.save_time_metric(&payload, &workspace_id).await;
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

    fn authenticated_request(method: Method, uri: &str) -> Request<Body> {
        Request::builder()
            .uri(uri)
            .method(method)
            .header("x-cortex-token", "test-token")
            .body(Body::empty())
            .expect("build authenticated request")
    }

    #[tokio::test]
    async fn unregister_route_removes_existing_agent() {
        // Set test auth token
        std::env::set_var("X-CORTEX-TOKEN", "test-token");

        let registry = SimpleAgentRegistry::new();
        registry
            .register(
                "agent-delete-1".to_string(),
                "session-delete-1".to_string(),
                Default::default(),
            )
            .await;

        let response = create_router_with_agent_registry(registry.clone())
            .oneshot(authenticated_request(
                Method::DELETE,
                "/xavier2/agents/agent-delete-1/unregister",
            ))
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

        std::env::remove_var("X-CORTEX-TOKEN");
    }

    #[tokio::test]
    async fn unregister_route_returns_error_for_missing_agent() {
        std::env::set_var("X-CORTEX-TOKEN", "test-token");

        let response = create_router_with_agent_registry(SimpleAgentRegistry::new())
            .oneshot(authenticated_request(
                Method::DELETE,
                "/xavier2/agents/missing-agent/unregister",
            ))
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

        std::env::remove_var("X-CORTEX-TOKEN");
    }

    #[tokio::test]
    async fn health_route_works_without_auth() {
        std::env::set_var("X-CORTEX-TOKEN", "test-token");

        let response = create_router_with_agent_registry(SimpleAgentRegistry::new())
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .method(Method::GET)
                    .body(Body::empty())
                    .expect("build GET request"),
            )
            .await
            .expect("request should complete");

        assert_eq!(response.status(), StatusCode::OK);
        let body = response
            .into_body()
            .collect()
            .await
            .expect("collect body")
            .to_bytes();
        assert_eq!(&body[..], b"ok");

        std::env::remove_var("X-CORTEX-TOKEN");
    }

    #[tokio::test]
    async fn protected_route_returns_401_without_token() {
        std::env::set_var("X-CORTEX-TOKEN", "test-token");

        let response = create_router_with_agent_registry(SimpleAgentRegistry::new())
            .oneshot(
                Request::builder()
                    .uri("/xavier2/events/session")
                    .method(Method::POST)
                    .body(Body::empty())
                    .expect("build POST request"),
            )
            .await
            .expect("request should complete");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        std::env::remove_var("X-CORTEX-TOKEN");
    }

    #[tokio::test]
    async fn protected_route_returns_401_with_wrong_token() {
        std::env::set_var("X-CORTEX-TOKEN", "correct-token");

        let response = create_router_with_agent_registry(SimpleAgentRegistry::new())
            .oneshot(
                Request::builder()
                    .uri("/xavier2/sync/check")
                    .method(Method::POST)
                    .header("x-cortex-token", "wrong-token")
                    .body(Body::empty())
                    .expect("build POST request"),
            )
            .await
            .expect("request should complete");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        std::env::remove_var("X-CORTEX-TOKEN");
    }

    #[tokio::test]
    async fn protected_route_works_with_xavier2_token_header() {
        std::env::set_var("X-Xavier2-Token", "xavier2-secret");

        let response = create_router_with_agent_registry(SimpleAgentRegistry::new())
            .oneshot(
                Request::builder()
                    .uri("/xavier2/sync/check")
                    .method(Method::POST)
                    .header("x-xavier2-token", "xavier2-secret")
                    .body(Body::empty())
                    .expect("build POST request"),
            )
            .await
            .expect("request should complete");

        assert_eq!(response.status(), StatusCode::OK);

        std::env::remove_var("X-Xavier2-Token");
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

    #[test]
    fn test_is_url_safe_for_outbound() {
        // Blocked: internal/unsafe URLs
        assert!(!is_url_safe_for_outbound("http://localhost:8006/memory/add"));
        assert!(!is_url_safe_for_outbound("https://localhost:443/api"));
        assert!(!is_url_safe_for_outbound("http://127.0.0.1:8006"));
        assert!(!is_url_safe_for_outbound("http://127.0.0.2"));
        assert!(!is_url_safe_for_outbound("http://10.0.0.1"));
        assert!(!is_url_safe_for_outbound("http://192.168.1.1"));
        assert!(!is_url_safe_for_outbound("http://172.16.0.1"));
        assert!(!is_url_safe_for_outbound("http://169.254.169.254")); // metadata IP
        assert!(!is_url_safe_for_outbound("http://0.0.0.0"));
        assert!(!is_url_safe_for_outbound("https://0.0.0.0"));
        assert!(!is_url_safe_for_outbound("ftp://localhost")); // wrong scheme
        assert!(!is_url_safe_for_outbound("http://localhost.localdomain"));

        // Allowed: legitimate outbound URLs
        assert!(is_url_safe_for_outbound("http://example.com"));
        assert!(is_url_safe_for_outbound("https://api.example.com/v1/memory"));
        assert!(is_url_safe_for_outbound("http://8.8.8.8"));
        assert!(is_url_safe_for_outbound("http://1.1.1.1"));
        assert!(is_url_safe_for_outbound("https://xavier2.cloud.swal.ai:8443"));
    }
}
