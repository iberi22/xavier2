use axum::{
    extract::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

use crate::adapters::inbound::http::dto::TimeMetricDto;
use crate::adapters::inbound::http::state::AppState;
use crate::agents::unregister_agent_handler;
use crate::coordination::SimpleAgentRegistry;
use crate::ports::inbound::{AgentLifecyclePort, InputSecurityPort, TimeMetricsPort};
use crate::ports::outbound::HealthCheckPort;
use crate::session::event_mapper::PanelThreadEntry;
use crate::session::types::SessionEvent;
use crate::settings::XavierSettings;
use crate::tasks::session_sync_task::get_last_sync_result;

// ─── Router ─────────────────────────────────────────────────────────────────

pub fn create_router(state: AppState) -> Router {
    create_router_with_state(state)
}

pub fn create_router_with_agent_registry(agent_registry: Arc<dyn AgentLifecyclePort>) -> Router {
    let state = AppState {
        memory: Arc::new(crate::app::qmd_memory_adapter::QmdMemoryAdapter::new(
            Arc::new(crate::memory::qmd_memory::QmdMemory::new(Arc::new(
                tokio::sync::RwLock::new(Vec::new()),
            ))),
        )),
        security: Arc::new(crate::app::security_service::SecurityService::new()),
        security_scan: Arc::new(crate::app::security_service::SecurityService::new()),
        time_metrics: Arc::new(
            crate::adapters::inbound::http::time_metrics_adapter::TimeMetricsAdapter::new(
                Arc::new(crate::time::TimeMetricsStore::new(Arc::new(
                    parking_lot::Mutex::new(rusqlite::Connection::open_in_memory().unwrap()),
                ))),
            ),
        ),
        agent_lifecycle: agent_registry,
        health: Arc::new(crate::app::health_service::HealthService::new()),
        verification: Arc::new(crate::app::verification_service::VerificationService::new()),
        session_sync: Arc::new(SessionSyncMock),
        session: Arc::new(SessionMock),
        workspace_id: "test".to_string(),
        auth_token: "test-token".to_string(),
        code_db: Arc::new(code_graph::db::CodeGraphDB::in_memory().unwrap()),
        code_indexer: Arc::new(code_graph::indexer::Indexer::new(Arc::new(
            code_graph::db::CodeGraphDB::in_memory().unwrap(),
        ))),
        code_query: Arc::new(code_graph::query::QueryEngine::new(Arc::new(
            code_graph::db::CodeGraphDB::in_memory().unwrap(),
        ))),
    };
    create_router_with_state(state)
}

struct SessionSyncMock;

#[async_trait::async_trait]
impl crate::ports::inbound::SessionSyncPort for SessionSyncMock {
    async fn check(&self) -> anyhow::Result<crate::tasks::session_sync_task::SyncCheckResult> {
        Ok(Default::default())
    }
    async fn last_result(&self) -> crate::tasks::session_sync_task::SyncCheckResult {
        Default::default()
    }
}

struct SessionMock;

#[async_trait::async_trait]
impl crate::ports::inbound::SessionPort for SessionMock {
    async fn handle_event(&self, _event: crate::session::types::SessionEvent) -> bool {
        true
    }
    async fn handle_and_index_event(
        &self,
        event: crate::session::types::SessionEvent,
    ) -> anyhow::Result<crate::ports::inbound::session_port::SessionEventResult> {
        Ok(crate::ports::inbound::session_port::SessionEventResult {
            status: "ok".to_string(),
            session_id: event.session_id,
            memory_id: None,
            mapped: true,
        })
    }
}

fn create_router_with_state(state: AppState) -> Router {
    let router = Router::new()
        .route("/health", get(health_handler))
        .route(
            "/xavier/agents/{id}/unregister",
            post(unregister_agent_handler),
        )
        .route("/xavier/verify/save", post(verify_save_handler))
        .route("/xavier/time/metric", post(time_metric_handler))
        .route("/xavier/events/session", post(session_event_handler))
        .route(
            "/xavier/sync/check",
            get(sync_check_handler).post(sync_check_handler),
        );

    // Add enterprise plugin routes if feature is enabled
    #[cfg(feature = "enterprise")]
    let router = router
        .route("/plugins/health", get(plugins_health_handler))
        .route("/plugins/sync", post(plugins_sync_handler));

    router.with_state(state)
}

async fn health_handler() -> &'static str {
    "ok"
}

pub async fn session_event_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(event): Json<SessionEvent>,
) -> Json<serde_json::Value> {
    let Some(entry) = PanelThreadEntry::from_session_event(&event) else {
        return Json(serde_json::json!({
            "status": "ok",
            "session_id": event.session_id,
            "mapped": false,
        }));
    };

    let result = match state.security.process_input(&entry.content).await {
        Ok(res) => res,
        Err(e) => {
            return Json(serde_json::json!({
                "status": "error",
                "message": format!("Security scan error: {}", e),
                "session_id": event.session_id,
            }));
        }
    };

    if !result.allowed {
        return Json(serde_json::json!({
            "status": "blocked",
            "blocked": true,
            "reason": "security_policy_violation",
            "session_id": event.session_id,
            "mapped": false,
            "detection": {
                "is_injection": result.is_injection,
                "confidence": result.detection_confidence,
                "attack_type": result.attack_type,
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
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(payload): Json<VerifySaveRequest>,
) -> Json<VerifySaveResponse> {
    let start = Instant::now();

    let xavier_url = std::env::var("XAVIER_URL")
        .unwrap_or_else(|_| XavierSettings::current().client_base_url());

    // Validate internal URL to prevent SSRF
    if let Err(e) = crate::security::url_validator::validate_internal_url(&xavier_url) {
        tracing::error!("Internal URL validation failed: {}", e);
        return Json(VerifySaveResponse {
            save_ok: false,
            latency_ms: start.elapsed().as_millis() as u64,
            match_score: 0.0,
        });
    }

    let auth_token = match std::env::var("XAVIER_TOKEN") {
        Ok(token) => token,
        Err(_) => {
            tracing::error!("XAVIER_TOKEN is required for verification requests");
            return Json(VerifySaveResponse {
                save_ok: false,
                latency_ms: start.elapsed().as_millis() as u64,
                match_score: 0.0,
            });
        }
    };

    let result = state
        .verification
        .verify_save(&xavier_url, &auth_token, &payload.path, &payload.content)
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
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(payload): Json<TimeMetricDto>,
) -> Json<TimeMetricResponse> {
    let workspace_id = &state.workspace_id;

    let domain_metric: crate::domain::memory::TimeMetric = payload.clone().into();
    let result = state
        .time_metrics
        .save_time_metric(&domain_metric, workspace_id)
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

// ─── Enterprise Plugin Endpoints ────────────────────────────────────────────
// These endpoints are only available when the "enterprise" feature is enabled

#[cfg(feature = "enterprise")]
#[derive(Debug, Serialize)]
pub struct PluginsHealthResponse {
    pub status: String,
    pub plugins: Vec<PluginHealthStatus>,
}

#[cfg(feature = "enterprise")]
#[derive(Debug, Serialize)]
pub struct PluginHealthStatus {
    pub name: String,
    pub version: String,
    pub healthy: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[cfg(feature = "enterprise")]
#[derive(Debug, Deserialize)]
pub struct PluginSyncRequest {
    pub direction: String, // "push", "pull", or "both"
}

#[cfg(feature = "enterprise")]
#[derive(Debug, Serialize)]
pub struct PluginSyncResponse {
    pub status: String,
    pub results: Vec<PluginSyncResult>,
}

#[cfg(feature = "enterprise")]
#[derive(Debug, Serialize)]
pub struct PluginSyncResult {
    pub plugin_name: String,
    pub success: bool,
    pub items_synced: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Plugin registry singleton (lazy-initialized)
#[cfg(feature = "enterprise")]
static PLUGIN_REGISTRY: std::sync::OnceLock<
    std::sync::Arc<tokio::sync::RwLock<crate::adapters::inbound::http::plugins::PluginRegistry>>,
> = std::sync::OnceLock::new();

/// Initialize the plugin registry with Cortex plugin if configured
#[cfg(feature = "enterprise")]
pub fn init_plugin_registry() {
    use crate::adapters::inbound::http::plugins::{
        cortex::{CortexConfig, CortexPlugin},
        PluginRegistry,
    };

    let registry = PluginRegistry::new();

    // Try to initialize Cortex plugin if env vars are set
    if CortexConfig::is_configured() {
        tracing::info!("Initializing Cortex Enterprise plugin");

        // We need to spawn this because we can't await in init
        tokio::spawn(async move {
            if let Some(plugin) = CortexPlugin::from_env() {
                let registry = get_plugin_registry();
                registry.write().await.register(Box::new(plugin));
                tracing::info!("Cortex Enterprise plugin registered");
            }
        });
    } else {
        tracing::debug!("Cortex Enterprise not configured (missing env vars)");
    }

    let registry_arc = std::sync::Arc::new(tokio::sync::RwLock::new(registry));
    if PLUGIN_REGISTRY.set(registry_arc).is_err() {
        tracing::error!("Plugin registry already initialized");
    }
}

/// Get the plugin registry (panics if not initialized)
#[cfg(feature = "enterprise")]
pub fn get_plugin_registry(
) -> std::sync::Arc<tokio::sync::RwLock<crate::adapters::inbound::http::plugins::PluginRegistry>> {
    PLUGIN_REGISTRY
        .get()
        .expect("Plugin registry not initialized. Call init_plugin_registry() at startup.")
        .clone()
}

#[cfg(feature = "enterprise")]
pub async fn plugins_health_handler() -> Json<PluginsHealthResponse> {
    use crate::adapters::inbound::http::plugins::Plugin;

    let registry = get_plugin_registry();
    let registry = registry.read().await;

    let mut plugins = Vec::new();

    for plugin in registry.plugins() {
        let name = plugin.name().to_string();
        let version = plugin.version().to_string();

        // Run health check
        let health_result = plugin.health_check().await;
        let (healthy, error) = match health_result {
            Ok(()) => (true, None),
            Err(e) => (false, Some(e)),
        };

        plugins.push(PluginHealthStatus {
            name,
            version,
            healthy,
            error,
        });
    }

    let status = if plugins.iter().all(|p| p.healthy) {
        "healthy"
    } else if plugins.iter().any(|p| p.healthy) {
        "degraded"
    } else {
        "unhealthy"
    };

    Json(PluginsHealthResponse {
        status: status.to_string(),
        plugins,
    })
}

#[cfg(feature = "enterprise")]
pub async fn plugins_sync_handler(
    Json(payload): Json<PluginSyncRequest>,
) -> Json<PluginSyncResponse> {
    use crate::adapters::inbound::http::plugins::{Plugin, SyncDirection};

    let direction = match payload.direction.to_lowercase().as_str() {
        "push" => SyncDirection::Push,
        "pull" => SyncDirection::Pull,
        "both" => SyncDirection::Both,
        _ => {
            return Json(PluginSyncResponse {
                status: "error".to_string(),
                results: vec![],
            });
        }
    };

    let registry = get_plugin_registry();
    let registry = registry.read().await;

    let mut results = Vec::new();
    let mut any_success = false;
    let mut any_failure = false;

    for plugin in registry.plugins() {
        let plugin_name = plugin.name().to_string();

        match plugin.sync(direction).await {
            Ok(sync_result) => {
                if sync_result.success {
                    any_success = true;
                } else {
                    any_failure = true;
                }

                results.push(PluginSyncResult {
                    plugin_name,
                    success: sync_result.success,
                    items_synced: sync_result.items_synced,
                    error: sync_result.error,
                });
            }
            Err(e) => {
                any_failure = true;
                results.push(PluginSyncResult {
                    plugin_name,
                    success: false,
                    items_synced: 0,
                    error: Some(e),
                });
            }
        }
    }

    let status = if any_failure && !any_success {
        "error"
    } else if any_failure {
        "partial"
    } else {
        "success"
    };

    Json(PluginSyncResponse {
        status: status.to_string(),
        results,
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
            .oneshot(post_request("/xavier/agents/agent-delete-1/unregister"))
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
            .oneshot(post_request("/xavier/agents/missing-agent/unregister"))
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
    use crate::tasks::session_sync_task::{SyncCheckResult, LAST_CHECK_RESULT};

    #[tokio::test]
    async fn sync_check_handler_uses_cached_sync_result() {
        let test_result = SyncCheckResult {
            status: "alert".to_string(),
            lag_ms: 42_000,
            save_ok_rate: 0.90,
            match_score: 0.88,
            active_agents: 7,
            timestamp_ms: 1_234_567,
            alerts: vec![
                "Index lag 42000ms exceeds threshold 30000ms".to_string(),
                "Save ok rate 90.0% below threshold 95.0%".to_string(),
            ],
        };
        *LAST_CHECK_RESULT.write().unwrap() = test_result;

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
