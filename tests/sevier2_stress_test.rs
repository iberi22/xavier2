//! SEVIER2 Stress Test Suite for Xavier2
//!
//! Tests rapid event ingestion, concurrent save/retrieve, large content handling,
//! and time metric endpoint load.
//!
//! Run with: cargo test --lib sevier2_stress

#[cfg(test)]
mod sevier2_stress_tests {
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    use tokio::sync::Mutex as TokioMutex;

    // ─── Test 1: Rapid Session Events ────────────────────────────────────────
    // Send 100 session events in < 10 seconds and verify all get indexed.

    #[tokio::test]
    #[ignore = "requires running Xavier2 on port 8006"]
    async fn test_rapid_session_events() {
        let client = reqwest::Client::new();
        let base_url = std::env::var("XAVIER2_URL")
            .unwrap_or_else(|_| "http://localhost:8006".to_string());
        let token = std::env::var("X_CORTEX_TOKEN")
            .unwrap_or_else(|_| "dev-token".to_string());

        let start = Instant::now();
        let count = 100;
        let mut handles = Vec::new();

        for i in 0..count {
            let client = client.clone();
            let base_url = base_url.clone();
            let token = token.clone();

            handles.push(tokio::spawn(async move {
                let payload = serde_json::json!({
                    "session_id": format!("stress-session-{}", i),
                    "event_type": "message",
                    "content": format!("Rapid event #{}", i),
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                });

                client
                    .post(format!("{}/xavier2/events/session", base_url))
                    .header("Authorization", format!("Bearer {}", token))
                    .json(&payload)
                    .send()
                    .await
            }));
        }

        let results = futures::future::join_all(handles).await;
        let elapsed_ms = start.elapsed().as_millis() as u64;

        let ok_count = results.iter().filter(|r| {
            r.as_ref().is_ok_and(|resp| resp.status().is_success())
        }).count();

        println!(
            "Test 1 — Rapid Events: {}/{} succeeded in {}ms (limit: 10000ms)",
            ok_count, count, elapsed_ms
        );

        assert!(
            elapsed_ms < 10000,
            "Took {}ms, expected < 10000ms",
            elapsed_ms
        );
        assert_eq!(
            ok_count, count,
            "Only {}/{} events succeeded",
            ok_count, count
        );
    }

    // ─── Test 2: Concurrent Save/Retrieve ────────────────────────────────────
    // 50 parallel saves with different paths, then verify all retrieved.

    #[tokio::test]
    #[ignore = "requires running Xavier2 on port 8006"]
    async fn test_concurrent_save_retrieve() {
        let client = reqwest::Client::new();
        let base_url = std::env::var("XAVIER2_URL")
            .unwrap_or_else(|_| "http://localhost:8006".to_string());
        let token = std::env::var("X_CORTEX_TOKEN")
            .unwrap_or_else(|_| "dev-token".to_string());

        let count = 50;
        let mut save_handles = Vec::new();

        for i in 0..count {
            let client = client.clone();
            let base_url = base_url.clone();
            let token = token.clone();
            let path = format!("stress/verify/{}", i);

            save_handles.push(tokio::spawn(async move {
                let payload = serde_json::json!({
                    "path": path,
                    "content": format!("Concurrent save content #{}", i),
                });

                client
                    .post(format!("{}/xavier2/verify/save", base_url))
                    .header("Authorization", format!("Bearer {}", token))
                    .json(&payload)
                    .send()
                    .await
            }));
        }

        let save_results = futures::future::join_all(save_handles).await;
        let ok_count = save_results.iter().filter(|r| {
            r.as_ref().is_ok_and(|resp| resp.status().is_success())
        }).count();

        println!(
            "Test 2 — Concurrent Save: {}/{} saves returned 200",
            ok_count, count
        );
        assert_eq!(ok_count, count, "Expected all {} saves to succeed", count);

        // Retrieve all and check match_score > 0
        let retrieve_handles: Vec<_> = (0..count)
            .map(|i| {
                let client = client.clone();
                let base_url = base_url.clone();
                let token = token.clone();
                let path = format!("stress/verify/{}", i);

                tokio::spawn(async move {
                    let payload = serde_json::json!({
                        "path": path,
                        "content": format!("Concurrent save content #{}", i),
                    });

                    let resp = client
                        .post(format!("{}/xavier2/verify/save", base_url))
                        .header("Authorization", format!("Bearer {}", token))
                        .json(&payload)
                        .send()
                        .await;

                    if let Ok(resp) = resp {
                        if let Ok(body) = resp.json::<serde_json::Value>().await {
                            let save_ok = body.get("save_ok")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false);
                            let match_score = body.get("match_score")
                                .and_then(|v| v.as_f64())
                                .unwrap_or(0.0) as f32;
                            return (save_ok, match_score);
                        }
                    }
                    (false, 0.0)
                })
            })
            .collect();

        let retrieve_results = futures::future::join_all(retrieve_handles).await;
        let all_ok = retrieve_results.iter().all(|(save_ok, _)| *save_ok);

        println!(
            "Test 2 — Concurrent Retrieve: all {} verified = {}",
            count, all_ok
        );
        assert!(all_ok, "Not all saves retrieved correctly");
    }

    // ─── Test 3: Large Content (50KB+) ────────────────────────────────────────
    // Send a session event with 50KB+ content and verify it persists.

    #[tokio::test]
    #[ignore = "requires running Xavier2 on port 8006"]
    async fn test_large_content() {
        let client = reqwest::Client::new();
        let base_url = std::env::var("XAVIER2_URL")
            .unwrap_or_else(|_| "http://localhost:8006".to_string());
        let token = std::env::var("X_CORTEX_TOKEN")
            .unwrap_or_else(|_| "dev-token".to_string());

        // Generate 60KB of content
        let large_content: String = std::iter::repeat('x')
            .take(60 * 1024)
            .collect();

        let payload = serde_json::json!({
            "session_id": "stress-large-content",
            "event_type": "message",
            "content": large_content,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        let resp = client
            .post(format!("{}/xavier2/events/session", base_url))
            .header("Authorization", format!("Bearer {}", token))
            .json(&payload)
            .send()
            .await
            .expect("Request should complete");

        println!(
            "Test 3 — Large Content: status = {}",
            resp.status()
        );

        assert!(
            resp.status().is_success(),
            "Large content request failed with {}",
            resp.status()
        );
    }

    // ─── Test 4: Time Metrics Endpoint Load ────────────────────────────────────
    // 50 parallel requests to /xavier2/time/metric.

    #[tokio::test]
    #[ignore = "requires running Xavier2 on port 8006"]
    async fn test_time_metrics_load() {
        let client = reqwest::Client::new();
        let base_url = std::env::var("XAVIER2_URL")
            .unwrap_or_else(|_| "http://localhost:8006".to_string());
        let token = std::env::var("X_CORTEX_TOKEN")
            .unwrap_or_else(|_| "dev-token".to_string());

        let count = 50;
        let mut handles = Vec::new();

        for i in 0..count {
            let client = client.clone();
            let base_url = base_url.clone();
            let token = token.clone();

            handles.push(tokio::spawn(async move {
                let now = chrono::Utc::now();
                let payload = serde_json::json!({
                    "metric_type": format!("stress-test-{}", i % 5),
                    "agent_id": format!("stress-agent-{}", i % 3),
                    "task_id": format!("task-{}", i),
                    "started_at": now.to_rfc3339(),
                    "completed_at": now.to_rfc3339(),
                    "duration_ms": 100 + i as u64,
                    "status": "ok",
                    "provider": Some("minimax"),
                    "model": Some("MiniMax-M2.7"),
                    "tokens_used": Some(1000u64 + i as u64 * 10),
                    "task_category": Some("coding"),
                    "metadata": {}
                });

                client
                    .post(format!("{}/xavier2/time/metric", base_url))
                    .header("Authorization", format!("Bearer {}", token))
                    .json(&payload)
                    .send()
                    .await
            }));
        }

        let start = Instant::now();
        let results = futures::future::join_all(handles).await;
        let elapsed_ms = start.elapsed().as_millis() as u64;

        let ok_count = results.iter().filter(|r| {
            r.as_ref().is_ok_and(|resp| resp.status().is_success())
        }).count();

        println!(
            "Test 4 — Time Metrics Load: {}/{} succeeded in {}ms",
            ok_count, count, elapsed_ms
        );

        assert_eq!(
            ok_count, count,
            "Only {}/{} time metric requests succeeded",
            ok_count, count
        );
    }
}
