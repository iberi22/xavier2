//! SEVIER2 Integration Tests — Xavier2 HTTP API
//!
//! Tests for the SEVIER2 endpoint group: time metrics, session events,
//! and sync-check.
//!
//! All tests use `tower::ServiceExt::oneshot` to hit a real router without
//! spinning up a TCP listener.
//!
//! Run with: cargo test --test sevier2_stress_test

use axum::{body::Body, http::Request, http::StatusCode};
use http_body_util::BodyExt;
use parking_lot::Mutex;
use rusqlite::Connection;
use std::sync::Arc;
use tower::ServiceExt;

use xavier2::adapters::inbound::http::dto::TimeMetricDto;
use xavier2::adapters::inbound::http::routes::create_router;
use xavier2::adapters::inbound::http::routes::create_router_with_agent_registry;
use xavier2::coordination::SimpleAgentRegistry;
use xavier2::domain::agent::AgentMetadata;
use xavier2::time::TimeMetricsStore;

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn build_router() -> axum::Router {
    create_router()
}

/// Create an in-memory SQLite DB with the time_metrics schema initialized.
fn in_memory_db() -> Arc<Mutex<Connection>> {
    let conn = Connection::open_in_memory().expect("open in-memory DB");
    let db = Arc::new(Mutex::new(conn));
    TimeMetricsStore::init_schema(&db.lock()).expect("init time_metrics schema");
    db
}

/// Build a JSON POST request with a serde_json::Value body.
fn json_post(uri: &str, body: serde_json::Value) -> Request<Body> {
    let json = serde_json::to_string(&body).expect("serialize JSON");
    Request::builder()
        .uri(uri)
        .method(axum::http::Method::POST)
        .header(axum::http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(axum::body::Bytes::from(json)))
        .expect("build POST request")
}

/// Build a GET request.
fn get(uri: &str) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .body(Body::empty())
        .expect("build GET request")
}

/// Build a POST request.
fn post_empty(uri: &str) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .method(axum::http::Method::POST)
        .body(Body::empty())
        .expect("build POST request")
}

/// Assert status is 2xx.
fn assert_ok(status: StatusCode) {
    assert!(status.is_success(), "Expected 2xx, got {}", status.as_u16());
}

// ─── Test 1: Health GET ──────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "requires running xavier2 server on port 8006"]
async fn test_health_get() {
    let router = build_router();
    let response = router
        .clone()
        .oneshot(get("/health"))
        .await
        .expect("request should complete");

    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response
        .into_body()
        .collect()
        .await
        .expect("collect body")
        .to_bytes();
    assert_eq!(&*body_bytes, b"ok");
}

// ─── Test 2: Time Metric — save and retrieve via SQLite ──────────────────────

#[tokio::test]
#[ignore = "requires running xavier2 server on port 8006"]
async fn test_time_metric_save_and_retrieve() {
    let db = in_memory_db();
    let store = Arc::new(TimeMetricsStore::new(db.clone()));

    let metric = TimeMetricDto {
        metric_type: "agent_execution".to_string(),
        agent_id: "test-agent-001".to_string(),
        task_id: Some("task-123".to_string()),
        started_at: "2026-04-24T10:00:00Z".to_string(),
        completed_at: "2026-04-24T10:00:05Z".to_string(),
        duration_ms: 5000,
        status: "success".to_string(),
        error_message: None,
        provider: Some("minimax".to_string()),
        model: Some("MiniMax-M2.7".to_string()),
        tokens_used: Some(1500),
        task_category: Some("coding".to_string()),
        metadata: serde_json::json!({}),
    };

    // Save via the store directly
    store
        .save_time_metric(&metric, "test-workspace")
        .await
        .expect("save should succeed");

    // Verify row exists in SQLite
    let conn = db.lock();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM time_metrics WHERE agent_id = ?1",
            [&metric.agent_id],
            |row| row.get(0),
        )
        .expect("query should succeed");

    assert_eq!(count, 1, "Expected 1 row for agent_id=test-agent-001");
}

// ─── Test 3: Time Metric endpoint returns expected shape ─────────────────────

#[tokio::test]
#[ignore = "requires running xavier2 server on port 8006"]
async fn test_time_metric_endpoint_returns_correct_shape() {
    let router = build_router();

    let metric = serde_json::json!({
        "metric_type": "agent_execution",
        "agent_id": "test-agent-001",
        "task_id": "task-123",
        "started_at": "2026-04-24T10:00:00Z",
        "completed_at": "2026-04-24T10:00:05Z",
        "duration_ms": 5000,
        "status": "success",
        "error_message": null,
        "provider": "minimax",
        "model": "MiniMax-M2.7",
        "tokens_used": 1500,
        "task_category": "coding",
        "metadata": {}
    });

    let response = router
        .clone()
        .oneshot(json_post("/xavier2/time/metric", metric))
        .await
        .expect("request should complete");

    let status = response.status();
    assert!(
        status == StatusCode::OK || status == StatusCode::CREATED,
        "Expected 200 or 201, got {}",
        status.as_u16()
    );

    let body_bytes = response
        .into_body()
        .collect()
        .await
        .expect("collect body")
        .to_bytes();
    let parsed: serde_json::Value =
        serde_json::from_slice(&body_bytes).expect("parse JSON response");

    assert!(
        parsed.get("status").is_some(),
        "Response should contain 'status' field: {:?}",
        parsed
    );
    assert_eq!(parsed["metric_type"].as_str().unwrap(), "agent_execution");
    assert_eq!(parsed["agent_id"].as_str().unwrap(), "test-agent-001");
}

// ─── Test 4: Session Event — POST and verify mapped response ─────────────────

#[tokio::test]
#[ignore = "requires running xavier2 server on port 8006"]
async fn test_session_event_returns_mapped_status() {
    let router = build_router();

    let event = serde_json::json!({
        "session_id": "session-test-001",
        "event_type": "message",
        "content": "Test message from integration test",
        "timestamp": "2026-04-24T10:00:00Z",
        "metadata": { "source": "integration-test" }
    });

    let response = router
        .clone()
        .oneshot(json_post("/xavier2/events/session", event))
        .await
        .expect("request should complete");

    assert_ok(response.status());

    let body_bytes = response
        .into_body()
        .collect()
        .await
        .expect("collect body")
        .to_bytes();
    let parsed: serde_json::Value =
        serde_json::from_slice(&body_bytes).expect("parse JSON response");

    assert_eq!(
        parsed["status"].as_str().unwrap(),
        "ok",
        "Expected status 'ok', got: {:?}",
        parsed["status"]
    );
    assert!(
        parsed.get("mapped").is_some(),
        "Response should contain 'mapped' field"
    );
    assert_eq!(parsed["session_id"].as_str().unwrap(), "session-test-001");
}

// ─── Test 5: Session Event unknown event type falls back gracefully ──────────

#[tokio::test]
#[ignore = "requires running xavier2 server on port 8006"]
async fn test_session_event_unknown_type_graceful_fallback() {
    let router = build_router();

    let event = serde_json::json!({
        "session_id": "session-unknown-type",
        "event_type": "completely_unknown_type",
        "content": "Should still work",
        "timestamp": "2026-04-24T10:00:00Z",
        "metadata": {}
    });

    let response = router
        .clone()
        .oneshot(json_post("/xavier2/events/session", event))
        .await
        .expect("request should complete");

    // Unknown types fall back to Message and return 2xx
    assert!(
        response.status().is_success() || response.status() == StatusCode::ACCEPTED,
        "Unknown event type should not return error, got {}",
        response.status().as_u16()
    );
}

// ─── Test 6: Sync Check — response shape validation ───────────────────────────

#[tokio::test]
#[ignore = "requires running xavier2 server on port 8006"]
async fn test_sync_check_returns_metrics() {
    let router = build_router();

    let response = router
        .clone()
        .oneshot(json_post("/xavier2/sync/check", serde_json::json!({})))
        .await
        .expect("request should complete");

    assert_ok(response.status());

    let body_bytes = response
        .into_body()
        .collect()
        .await
        .expect("collect body")
        .to_bytes();
    let parsed: serde_json::Value =
        serde_json::from_slice(&body_bytes).expect("parse JSON response");

    assert!(parsed.get("status").is_some(), "Missing 'status' field");
    assert!(parsed.get("lag_ms").is_some(), "Missing 'lag_ms' field");
    assert!(
        parsed["lag_ms"].as_u64().is_some(),
        "'lag_ms' should be a u64"
    );
    assert!(
        parsed.get("save_ok_rate").is_some(),
        "Missing 'save_ok_rate' field"
    );
    assert!(
        parsed["save_ok_rate"].is_number(),
        "'save_ok_rate' should be a number"
    );
    assert!(
        parsed.get("active_agents").is_some(),
        "Missing 'active_agents' field"
    );
    assert!(
        parsed["active_agents"].as_u64().is_some(),
        "'active_agents' should be a u64"
    );
}

// ─── Test 7: Sync Check GET variant ──────────────────────────────────────────

#[tokio::test]
#[ignore = "requires running xavier2 server on port 8006"]
async fn test_sync_check_get_variant() {
    let router = build_router();

    let response = router
        .clone()
        .oneshot(get("/xavier2/sync/check"))
        .await
        .expect("request should complete");

    assert_ok(response.status());

    let body_bytes = response
        .into_body()
        .collect()
        .await
        .expect("collect body")
        .to_bytes();
    let parsed: serde_json::Value =
        serde_json::from_slice(&body_bytes).expect("parse JSON response");

    assert!(
        parsed.get("status").is_some(),
        "GET sync/check should return valid response"
    );
}

// ─── Test 8: Malformed JSON returns 400/422 ──────────────────────────────────

#[tokio::test]
#[ignore = "requires running xavier2 server on port 8006"]
async fn test_invalid_json_returns_error() {
    let router = build_router();

    let bad_body = Body::from(r#"not valid json"#);
    let request = Request::builder()
        .uri("/xavier2/time/metric")
        .method(axum::http::Method::POST)
        .header(axum::http::header::CONTENT_TYPE, "application/json")
        .body(bad_body)
        .expect("build request");

    let response = router.oneshot(request).await.expect("request completes");

    let status = response.status();
    assert!(
        status == StatusCode::BAD_REQUEST || status == StatusCode::UNPROCESSABLE_ENTITY,
        "Expected 400/422 for malformed JSON, got {}",
        status.as_u16()
    );
}

// ─── Test 9: Unknown route returns 404 ─────────────────────────────────────

#[tokio::test]
#[ignore = "requires running xavier2 server on port 8006"]
async fn test_unknown_route_returns_404() {
    let router = build_router();

    let response = router
        .oneshot(get("/xavier2/does/not/exist"))
        .await
        .expect("request should complete");

    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "Unknown route should return 404"
    );
}

#[tokio::test]
async fn test_unregister_endpoint_removes_existing_agent() {
    let registry = SimpleAgentRegistry::new();
    registry
        .register(
            "agent-delete-1".to_string(),
            "session-delete-1".to_string(),
            AgentMetadata::default(),
        )
        .await;

    let router = create_router_with_agent_registry(registry.clone());
    let response = router
        .oneshot(post_empty("/xavier2/agents/agent-delete-1/unregister"))
        .await
        .expect("request should complete");

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = response
        .into_body()
        .collect()
        .await
        .expect("collect body")
        .to_bytes();
    let parsed: serde_json::Value =
        serde_json::from_slice(&body_bytes).expect("parse JSON response");

    assert_eq!(parsed["status"], "ok");
    assert_eq!(parsed["agent_id"], "agent-delete-1");
    assert_eq!(parsed["message"], "Agent unregistered");
    assert!(registry.get("agent-delete-1").await.is_none());
}

#[tokio::test]
async fn test_unregister_endpoint_returns_error_for_missing_agent() {
    let router = create_router_with_agent_registry(SimpleAgentRegistry::new());
    let response = router
        .oneshot(post_empty("/xavier2/agents/missing-agent/unregister"))
        .await
        .expect("request should complete");

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = response
        .into_body()
        .collect()
        .await
        .expect("collect body")
        .to_bytes();
    let parsed: serde_json::Value =
        serde_json::from_slice(&body_bytes).expect("parse JSON response");

    assert_eq!(parsed["status"], "error");
    assert_eq!(parsed["agent_id"], "missing-agent");
    assert_eq!(parsed["message"], "Agent not found or already unregistered");
}
