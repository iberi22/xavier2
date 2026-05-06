//! HTTP API Integration Tests
//!
//! Tests the Xavier2 HTTP API endpoints using a real server spawned
//! on a random port, accessed via reqwest.

use reqwest::Client;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::{net::TcpListener, task::JoinHandle};
use xavier2::adapters::inbound::http::routes::create_router_with_agent_registry;
use xavier2::coordination::SimpleAgentRegistry;

// ─── Test Helpers ──────────────────────────────────────────────────────────

struct TestServer {
    base_url: String,
    client: Client,
    registry: Arc<SimpleAgentRegistry>,
    handle: JoinHandle<()>,
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

async fn spawn_test_server() -> TestServer {
    let registry = SimpleAgentRegistry::new();
    let app = create_router_with_agent_registry(registry.clone());
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind random test port");
    let addr = listener.local_addr().expect("read local address");
    let base_url = format!("http://{addr}");
    let handle = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("test server should serve");
    });

    let client = Client::new();
    wait_for_health(&client, &base_url).await;

    TestServer {
        base_url,
        client,
        registry,
        handle,
    }
}

async fn wait_for_health(client: &Client, base_url: &str) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if let Ok(response) = client.get(format!("{base_url}/health")).send().await {
            if response.status().is_success() {
                return;
            }
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "test server did not become healthy within 5s"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

// ─── Health Endpoint ───────────────────────────────────────────────────────

#[tokio::test]
async fn test_health_endpoint() {
    let server = spawn_test_server().await;

    let response = server
        .client
        .get(format!("{}/health", server.base_url))
        .send()
        .await
        .expect("health request");

    assert!(response.status().is_success(), "health should return 2xx");
    let body = response.text().await.expect("read health body");
    assert_eq!(body.trim(), "ok");
}

// ─── Session Event Endpoint ────────────────────────────────────────────────

#[tokio::test]
async fn test_session_event_endpoint() {
    let server = spawn_test_server().await;

    let response = server
        .client
        .post(format!("{}/xavier2/events/session", server.base_url))
        .json(&json!({
            "session_id": "http-api-test-session",
            "event_type": "message",
            "content": "Integration test message",
            "timestamp": "2026-05-06T10:00:00Z",
            "metadata": {
                "source": "http-api-integration-test"
            }
        }))
        .send()
        .await
        .expect("session event request");

    assert!(
        response.status().is_success(),
        "session event should succeed"
    );
    let body: Value = response.json().await.expect("parse session response");
    assert_eq!(body["status"], "ok");
    assert_eq!(body["session_id"], "http-api-test-session");
}

#[tokio::test]
async fn test_session_event_with_injection() {
    let server = spawn_test_server().await;

    // SQL injection attempt should be blocked
    let response = server
        .client
        .post(format!("{}/xavier2/events/session", server.base_url))
        .json(&json!({
            "session_id": "injection-test",
            "event_type": "message",
            "content": "'; DROP TABLE memories; --",
            "timestamp": "2026-05-06T10:00:00Z",
            "metadata": {}
        }))
        .send()
        .await
        .expect("session event request");

    assert!(
        response.status().is_success(),
        "session event should still return 200"
    );
    let body: Value = response.json().await.expect("parse session response");
    // The security service should detect and block this
    assert!(
        body["status"] == "blocked" || body["mapped"] == false,
        "injection should be blocked: {:?}",
        body
    );
}

// ─── Time Metric Endpoint ──────────────────────────────────────────────────

#[tokio::test]
async fn test_time_metric_endpoint() {
    let server = spawn_test_server().await;

    let response = server
        .client
        .post(format!("{}/xavier2/time/metric", server.base_url))
        .json(&json!({
            "metric_type": "agent_execution",
            "agent_id": "http-api-agent",
            "task_id": "http-api-task-001",
            "started_at": "2026-05-06T10:00:00Z",
            "completed_at": "2026-05-06T10:00:05Z",
            "duration_ms": 5000,
            "status": "success",
            "error_message": null,
            "provider": "test",
            "model": "test-model",
            "tokens_used": 100,
            "task_category": "integration-test",
            "metadata": {}
        }))
        .send()
        .await
        .expect("time metric request");

    assert!(
        response.status().is_success(),
        "time metric should succeed"
    );
    let body: Value = response.json().await.expect("parse time metric response");
    assert!(matches!(body["status"].as_str(), Some("ok" | "saved")));
    assert_eq!(body["metric_type"], "agent_execution");
    assert_eq!(body["agent_id"], "http-api-agent");
}

// ─── Sync Check Endpoint ───────────────────────────────────────────────────

#[tokio::test]
async fn test_sync_check_endpoint() {
    let server = spawn_test_server().await;

    let response = server
        .client
        .post(format!("{}/xavier2/sync/check", server.base_url))
        .send()
        .await
        .expect("sync check request");

    assert!(
        response.status().is_success(),
        "sync check should succeed"
    );
    let body: Value = response.json().await.expect("parse sync response");
    assert!(body["status"].is_string());
    assert!(body["lag_ms"].is_number());
    assert!(body["save_ok_rate"].is_number());
    assert!(body["match_score"].is_number());
    assert!(body["active_agents"].is_number());
    assert!(body["timestamp_ms"].is_number());
    // alerts should be present as an array (may be empty)
    assert!(body["alerts"].is_array());
}

// ─── Agent Unregister Endpoint ─────────────────────────────────────────────

#[tokio::test]
async fn test_agent_unregister_existing() {
    let server = spawn_test_server().await;

    // Register an agent via the registry directly
    use xavier2::coordination::agent_registry::AgentMetadata;
    assert!(
        server
            .registry
            .register(
                "http-api-agent-1".to_string(),
                "http-api-session".to_string(),
                AgentMetadata {
                    name: Some("http-api-agent-1".to_string()),
                    capabilities: vec!["memory".to_string()],
                    role: Some("worker".to_string()),
                    endpoint: None,
                },
            )
            .await
    );
    assert!(server.registry.heartbeat("http-api-agent-1").await);

    // Now unregister via HTTP
    let response = server
        .client
        .post(format!(
            "{}/xavier2/agents/http-api-agent-1/unregister",
            server.base_url
        ))
        .send()
        .await
        .expect("unregister request");

    assert!(
        response.status().is_success(),
        "unregister should succeed"
    );
    let body: Value = response.json().await.expect("parse unregister response");
    assert_eq!(body["status"], "ok");
    assert_eq!(body["agent_id"], "http-api-agent-1");
    assert!(server.registry.get("http-api-agent-1").await.is_none());
}

#[tokio::test]
async fn test_agent_unregister_missing() {
    let server = spawn_test_server().await;

    let response = server
        .client
        .post(format!(
            "{}/xavier2/agents/nonexistent-agent/unregister",
            server.base_url
        ))
        .send()
        .await
        .expect("unregister request");

    assert!(
        response.status().is_success(),
        "unregister missing agent should return 200"
    );
    let body: Value = response.json().await.expect("parse unregister response");
    assert_eq!(body["status"], "error");
    assert_eq!(body["agent_id"], "nonexistent-agent");
    assert_eq!(body["message"], "Agent not found or already unregistered");
}

// ─── Verify/Save Endpoint (without env vars → graceful error) ─────────────

#[tokio::test]
async fn test_verify_save_without_env_vars() {
    let server = spawn_test_server().await;

    let response = server
        .client
        .post(format!("{}/xavier2/verify/save", server.base_url))
        .json(&json!({
            "path": "integration-test/path",
            "content": "Test content for verify/save"
        }))
        .send()
        .await
        .expect("verify/save request");

    assert!(
        response.status().is_success(),
        "verify/save should return 200 even without env vars"
    );
    let body: Value = response.json().await.expect("parse verify/save response");
    assert!(body["save_ok"].is_boolean());
    assert!(body["latency_ms"].is_number());
    assert!(body["match_score"].is_number());
}

// ─── 404 Handling ──────────────────────────────────────────────────────────

#[tokio::test]
async fn test_nonexistent_endpoint_returns_404() {
    let server = spawn_test_server().await;

    let response = server
        .client
        .get(format!("{}/nonexistent/route", server.base_url))
        .send()
        .await
        .expect("request to nonexistent route");

    assert_eq!(
        response.status(),
        reqwest::StatusCode::NOT_FOUND,
        "nonexistent route should return 404"
    );
}

#[tokio::test]
async fn test_memory_endpoint_not_found_in_minimal_router() {
    let server = spawn_test_server().await;

    // The minimal router (create_router_with_agent_registry) does NOT include
    // /memory/* endpoints — those are in the CLI HTTP server only.
    let response = server
        .client
        .post(format!("{}/memory/search", server.base_url))
        .json(&json!({"query": "test", "limit": 5}))
        .send()
        .await
        .expect("request to memory/search");

    assert_eq!(
        response.status(),
        reqwest::StatusCode::NOT_FOUND,
        "/memory/search is not in the minimal router"
    );
}

// ─── Concurrent Requests ───────────────────────────────────────────────────

#[tokio::test]
async fn test_concurrent_requests() {
    let server = spawn_test_server().await;

    let mut handles = Vec::new();
    for i in 0..10 {
        let url = format!("{}/health", server.base_url);
        let client = server.client.clone();
        handles.push(tokio::spawn(async move {
            let resp = client.get(&url).send().await.expect("concurrent health");
            (i, resp.status().is_success())
        }));
    }

    for handle in handles {
        let (i, ok) = handle.await.expect("concurrent task");
        assert!(ok, "concurrent health request {} failed", i);
    }
}

// ─── Multi-step Workflow ───────────────────────────────────────────────────

#[tokio::test]
async fn test_multi_step_workflow() {
    let server = spawn_test_server().await;

    // 1. Health check
    let health = server
        .client
        .get(format!("{}/health", server.base_url))
        .send()
        .await
        .expect("health");
    assert!(health.status().is_success());

    // 2. Session event
    let session = server
        .client
        .post(format!("{}/xavier2/events/session", server.base_url))
        .json(&json!({
            "session_id": "workflow-session",
            "event_type": "message",
            "content": "Workflow test",
            "timestamp": "2026-05-06T10:00:00Z",
            "metadata": {"workflow": true}
        }))
        .send()
        .await
        .expect("session");
    assert!(session.status().is_success());
    let session_body: Value = session.json().await.expect("session body");
    assert_eq!(session_body["session_id"], "workflow-session");

    // 3. Time metric
    let metric = server
        .client
        .post(format!("{}/xavier2/time/metric", server.base_url))
        .json(&json!({
            "metric_type": "workflow",
            "agent_id": "workflow-agent",
            "task_id": "workflow-task",
            "started_at": "2026-05-06T10:00:00Z",
            "completed_at": "2026-05-06T10:00:01Z",
            "duration_ms": 1000,
            "status": "success",
            "error_message": null,
            "provider": null,
            "model": null,
            "tokens_used": null,
            "task_category": "integration",
            "metadata": {}
        }))
        .send()
        .await
        .expect("metric");
    assert!(metric.status().is_success());
    let metric_body: Value = metric.json().await.expect("metric body");
    assert_eq!(metric_body["agent_id"], "workflow-agent");

    // 4. Sync check
    let sync = server
        .client
        .post(format!("{}/xavier2/sync/check", server.base_url))
        .send()
        .await
        .expect("sync");
    assert!(sync.status().is_success());
    let sync_body: Value = sync.json().await.expect("sync body");
    assert!(sync_body["status"].is_string());
}
