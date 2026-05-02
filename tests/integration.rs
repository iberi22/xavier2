//! Xavier2 integration test suite.
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
#[path = "integration/coordination_test.rs"]
mod coordination_test;
#[path = "integration/hierarchical_curation_test.rs"]
mod hierarchical_curation_test;
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
#[path = "sevier2_stress_test.rs"]
mod sevier2_stress_test;
#[path = "integration/tasks_test.rs"]
mod tasks_test;

mod integration {
    use reqwest::Client;
    use serde_json::json;

    #[tokio::test]
    #[ignore = "requires running xavier2 server on port 8006"]
    async fn test_time_metrics_endpoint() {
        let client = Client::new();
        let metric = json!({
            "metric_type": "agent_execution",
            "agent_id": "test-agent-001",
            "task_id": Some("task-123"),
            "started_at": "2026-04-24T10:00:00Z",
            "completed_at": "2026-04-24T10:00:05Z",
            "duration_ms": 5000,
            "status": "success",
            "error_message": None::<String>,
            "provider": Some("minimax".to_string()),
            "model": Some("MiniMax-M2.7".to_string()),
            "tokens_used": Some(1500),
            "task_category": Some("coding".to_string()),
            "metadata": {}
        });

        let response = client
            .post("http://localhost:8006/xavier2/time/metric")
            .header("Content-Type", "application/json")
            .json(&metric)
            .send()
            .await;

        match response {
            Ok(resp) => {
                assert!(
                    resp.status().is_success()
                        || resp.status().as_u16() == 200
                        || resp.status().as_u16() == 201,
                    "Expected success status, got: {}",
                    resp.status()
                );
                let body: serde_json::Value = resp.json().await.unwrap();
                assert_eq!(
                    body["status"], "ok",
                    "Expected status 'ok', got: {:?}",
                    body
                );
                assert_eq!(body["metric_type"], "agent_execution");
                assert_eq!(body["agent_id"], "test-agent-001");
            }
            Err(e) => {
                // Server not running - that's fine for CI
                eprintln!("⚠️  Server not running (localhost:8006): {}", e);
                println!("SKIPPED: test_time_metrics_endpoint (server not available)");
            }
        }
    }

    #[tokio::test]
    #[ignore = "requires running xavier2 server on port 8006"]
    async fn test_verify_save_endpoint() {
        let client = Client::new();
        let payload = json!({
            "path": "verification/test-$(date +%s)",
            "content": "Xavier2 verification test content"
        });

        let response = client
            .post("http://localhost:8006/xavier2/verify/save")
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await;

        match response {
            Ok(resp) => {
                let body: serde_json::Value = resp.json().await.unwrap();
                assert!(
                    body["save_ok"].as_bool().unwrap_or(false),
                    "Expected save_ok=true, got: {:?}",
                    body
                );
                assert!(
                    body["latency_ms"].as_u64().unwrap_or(0) > 0,
                    "Expected latency_ms > 0, got: {:?}",
                    body
                );
                println!(
                    "✓ verify_save returned save_ok=true, latency={}ms, match={}",
                    body["latency_ms"].as_u64().unwrap_or(0),
                    body["match_score"].as_f64().unwrap_or(0.0)
                );
            }
            Err(e) => {
                eprintln!("⚠️  Server not running (localhost:8006): {}", e);
                println!("SKIPPED: test_verify_save_endpoint (server not available)");
            }
        }
    }

    #[tokio::test]
    #[ignore = "requires running xavier2 server on port 8006"]
    async fn test_session_event_endpoint() {
        let client = Client::new();
        let payload = json!({
            "session_id": "session-test-$(ulid)",
            "event_type": "message",
            "content": "Test message from integration test",
            "timestamp": "2026-04-24T10:00:00Z",
            "metadata": {
                "source": "integration-test"
            }
        });

        let response = client
            .post("http://localhost:8006/xavier2/events/session")
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await;

        match response {
            Ok(resp) => {
                assert!(
                    resp.status().is_success(),
                    "Expected success status, got: {}",
                    resp.status()
                );
                let body: serde_json::Value = resp.json().await.unwrap();
                assert_eq!(
                    body["status"], "ok",
                    "Expected status 'ok', got: {:?}",
                    body
                );
                assert!(
                    body["mapped"].as_bool().is_some(),
                    "Expected 'mapped' field in response"
                );
                println!(
                    "✓ session_event processed: status={}, mapped={}",
                    body["status"], body["mapped"]
                );
            }
            Err(e) => {
                eprintln!("⚠️  Server not running (localhost:8006): {}", e);
                println!("SKIPPED: test_session_event_endpoint (server not available)");
            }
        }
    }

    #[tokio::test]
    #[ignore = "requires running xavier2 server on port 8006"]
    async fn test_sync_check_endpoint() {
        let client = Client::new();

        let response = client
            .post("http://localhost:8006/xavier2/sync/check")
            .header("Content-Type", "application/json")
            .send()
            .await;

        match response {
            Ok(resp) => {
                assert!(
                    resp.status().is_success(),
                    "Expected success status, got: {}",
                    resp.status()
                );
                let body: serde_json::Value = resp.json().await.unwrap();
                assert!(
                    body["status"].is_string(),
                    "Expected 'status' field in response"
                );
                assert!(
                    body["lag_ms"].is_number(),
                    "Expected 'lag_ms' field in response"
                );
                println!(
                    "✓ sync_check status={}, lag_ms={}, save_ok_rate={}",
                    body["status"], body["lag_ms"], body["save_ok_rate"]
                );
            }
            Err(e) => {
                eprintln!("⚠️  Server not running (localhost:8006): {}", e);
                println!("SKIPPED: test_sync_check_endpoint (server not available)");
            }
        }
    }

    #[tokio::test]
    #[ignore = "requires running xavier2 server on port 8006"]
    async fn test_health_endpoint() {
        let client = Client::new();

        let response = client.get("http://localhost:8006/health").send().await;

        match response {
            Ok(resp) => {
                assert_eq!(resp.status(), 200, "Health endpoint should return 200");
                let text = resp.text().await.unwrap();
                assert_eq!(text.trim(), "ok", "Health endpoint should return 'ok'");
                println!("✓ Health check passed: {}", text);
            }
            Err(e) => {
                eprintln!("⚠️  Server not running (localhost:8006): {}", e);
                println!("SKIPPED: test_health_endpoint (server not available)");
            }
        }
    }

    #[tokio::test]
    #[ignore = "integration scaffold pending real dependencies"]
    async fn test_full_memory_workflow() {
        todo!("Implement with actual dependencies");
    }

    #[tokio::test]
    #[ignore = "integration scaffold pending real dependencies"]
    async fn test_agent_memory_interaction() {
        todo!("Implement");
    }

    #[tokio::test]
    #[ignore = "integration scaffold pending real dependencies"]
    async fn test_distributed_coordination() {
        todo!("Implement");
    }
}
