//! Xavier integration test suite.
//!
//! Run with: cargo test --test integration

#[path = "integration/a2a_test.rs"]
mod a2a_test;
#[path = "integration/agents_test.rs"]
mod agents_test;
#[path = "integration/belief_graph_test.rs"]
mod belief_graph_test;
#[path = "integration/checkpoint_test.rs"]
mod checkpoint_test;
#[path = "integration/cli.rs"]
mod cli;
#[path = "integration/coordination_test.rs"]
mod coordination_test;
#[path = "integration/hierarchical_curation_test.rs"]
mod hierarchical_curation_test;
#[path = "integration/impact_analysis_test.rs"]
mod impact_analysis_test;
#[path = "integration/http_api.rs"]
mod http_api;
#[path = "integration/internal_benchmark_test.rs"]
mod internal_benchmark_test;
#[path = "integration/memory_test.rs"]
mod memory_test;
#[path = "integration/scheduler_test.rs"]
mod scheduler_test;
#[path = "integration/security_hardening_test.rs"]
mod security_hardening_test;
#[path = "integration/security_test.rs"]
mod security_test;
#[path = "integration/server_test.rs"]
mod server_test;
#[path = "sevier_stress_test.rs"]
mod sevier_stress_test;
#[path = "integration/tasks_test.rs"]
mod tasks_test;

mod integration {
    use reqwest::Client;
    use serde_json::{json, Value};
    use tokio::{net::TcpListener, task::JoinHandle, time::Duration};
    use xavier::{
        adapters::inbound::http::routes::create_router_with_agent_registry,
        coordination::{agent_registry::AgentMetadata, SimpleAgentRegistry},
    };

    struct TestServer {
        base_url: String,
        client: Client,
        registry: std::sync::Arc<SimpleAgentRegistry>,
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
                "test server did not become healthy"
            );
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }

    async fn post_json(server: &TestServer, path: &str, payload: Value) -> reqwest::Response {
        server
            .client
            .post(format!("{}{}", server.base_url, path))
            .json(&payload)
            .send()
            .await
            .expect("request should reach test server")
    }

    #[tokio::test]
    async fn test_time_metrics_endpoint() {
        let server = spawn_test_server().await;
        let metric = json!({
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

        let response = post_json(&server, "/xavier/time/metric", metric).await;

        assert!(response.status().is_success());
        let body: Value = response.json().await.expect("parse time metric response");
        assert!(matches!(body["status"].as_str(), Some("ok" | "saved")));
        assert_eq!(body["metric_type"], "agent_execution");
        assert_eq!(body["agent_id"], "test-agent-001");
    }

    #[tokio::test]
    async fn test_verify_save_endpoint() {
        let server = spawn_test_server().await;
        std::env::set_var("XAVIER_URL", &server.base_url);
        std::env::set_var("X-CORTEX-TOKEN", "dev-token");

        let response = post_json(
            &server,
            "/xavier/verify/save",
            json!({
                "path": "verification/integration-test",
                "content": "Xavier verification test content"
            }),
        )
        .await;

        assert!(response.status().is_success());
        let body: Value = response.json().await.expect("parse verify response");
        assert!(body["save_ok"].is_boolean());
        assert!(body["latency_ms"].is_number());
        assert!(body["match_score"].is_number());
    }

    #[tokio::test]
    async fn test_session_event_endpoint() {
        let server = spawn_test_server().await;

        let response = post_json(
            &server,
            "/xavier/events/session",
            json!({
                "session_id": "session-test-001",
                "event_type": "message",
                "content": "Test message from integration test",
                "timestamp": "2026-04-24T10:00:00Z",
                "metadata": {
                    "source": "integration-test"
                }
            }),
        )
        .await;

        assert!(response.status().is_success());
        let body: Value = response.json().await.expect("parse session response");
        assert_eq!(body["status"], "ok");
        assert_eq!(body["session_id"], "session-test-001");
        assert_eq!(body["mapped"], true);
    }

    #[tokio::test]
    async fn test_sync_check_endpoint() {
        let server = spawn_test_server().await;

        let response = server
            .client
            .post(format!("{}/xavier/sync/check", server.base_url))
            .send()
            .await
            .expect("sync check request should reach test server");

        assert!(response.status().is_success());
        let body: Value = response.json().await.expect("parse sync response");
        assert!(body["status"].is_string());
        assert!(body["lag_ms"].is_number());
        assert!(body["save_ok_rate"].is_number());
        assert!(body["match_score"].is_number());
        assert!(body["active_agents"].is_number());
        assert!(body["timestamp_ms"].is_number());
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let server = spawn_test_server().await;

        let response = server
            .client
            .get(format!("{}/health", server.base_url))
            .send()
            .await
            .expect("health request should reach test server");

        assert!(response.status().is_success());
        assert_eq!(response.text().await.expect("read body").trim(), "ok");
    }

    #[tokio::test]
    async fn test_full_memory_workflow() {
        let server = spawn_test_server().await;

        let session_response = post_json(
            &server,
            "/xavier/events/session",
            json!({
                "session_id": "workflow-session",
                "event_type": "message",
                "content": "Workflow event with durable context",
                "timestamp": "2026-04-24T10:00:00Z",
                "metadata": {"workflow": true}
            }),
        )
        .await;
        assert!(session_response.status().is_success());
        let session_body: Value = session_response
            .json()
            .await
            .expect("parse workflow session response");
        assert_eq!(session_body["mapped"], true);

        let metric_response = post_json(
            &server,
            "/xavier/time/metric",
            json!({
                "metric_type": "workflow_step",
                "agent_id": "workflow-agent",
                "task_id": "workflow-task",
                "started_at": "2026-04-24T10:00:00Z",
                "completed_at": "2026-04-24T10:00:01Z",
                "duration_ms": 1000,
                "status": "success",
                "error_message": null,
                "provider": null,
                "model": null,
                "tokens_used": null,
                "task_category": "integration",
                "metadata": {"session_id": "workflow-session"}
            }),
        )
        .await;
        assert!(metric_response.status().is_success());
        let metric_body: Value = metric_response
            .json()
            .await
            .expect("parse workflow metric response");
        assert_eq!(metric_body["agent_id"], "workflow-agent");

        let sync_response = server
            .client
            .post(format!("{}/xavier/sync/check", server.base_url))
            .send()
            .await
            .expect("workflow sync check should reach test server");
        assert!(sync_response.status().is_success());
        let sync_body: Value = sync_response
            .json()
            .await
            .expect("parse workflow sync response");
        assert!(sync_body["status"].is_string());
    }

    #[tokio::test]
    async fn test_agent_memory_interaction() {
        let server = spawn_test_server().await;
        let metadata = AgentMetadata {
            name: Some("integration-agent".to_string()),
            capabilities: vec!["memory".to_string(), "coordination".to_string()],
            role: Some("worker".to_string()),
            endpoint: None,
        };

        assert!(
            server
                .registry
                .register(
                    "agent-memory-1".to_string(),
                    "session-memory-1".to_string(),
                    metadata,
                )
                .await
        );
        assert!(server.registry.heartbeat("agent-memory-1").await);

        let response = server
            .client
            .post(format!(
                "{}/xavier/agents/agent-memory-1/unregister",
                server.base_url
            ))
            .send()
            .await
            .expect("unregister request should reach test server");

        assert!(response.status().is_success());
        let body: Value = response.json().await.expect("parse unregister response");
        assert_eq!(body["status"], "ok");
        assert_eq!(body["agent_id"], "agent-memory-1");
        assert!(server.registry.get("agent-memory-1").await.is_none());
    }

    #[tokio::test]
    async fn test_distributed_coordination() {
        let server = spawn_test_server().await;

        for (agent_id, session_id, role) in [
            ("coordinator-1", "session-coordinator", "coordinator"),
            ("worker-1", "session-worker", "worker"),
        ] {
            assert!(
                server
                    .registry
                    .register(
                        agent_id.to_string(),
                        session_id.to_string(),
                        AgentMetadata {
                            name: Some(agent_id.to_string()),
                            capabilities: vec!["sync".to_string()],
                            role: Some(role.to_string()),
                            endpoint: None,
                        },
                    )
                    .await
            );
        }

        assert!(server.registry.heartbeat("coordinator-1").await);
        assert!(server.registry.heartbeat("worker-1").await);
        let active_before = server.registry.get_active_agents().await;
        assert_eq!(active_before.len(), 2);

        let response = server
            .client
            .post(format!(
                "{}/xavier/agents/worker-1/unregister",
                server.base_url
            ))
            .send()
            .await
            .expect("unregister request should reach test server");
        assert!(response.status().is_success());

        let active_after = server.registry.get_active_agents().await;
        assert_eq!(active_after.len(), 1);
        assert_eq!(active_after[0].agent_id, "coordinator-1");
        assert!(server.registry.get("worker-1").await.is_none());
    }
}
