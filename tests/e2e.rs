use std::net::TcpListener;
use std::process::{Child, Stdio};
use std::time::Duration;

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_health_endpoint_via_xavier_binary() {
    let port = TcpListener::bind("127.0.0.1:0")
        .expect("bind ephemeral port")
        .local_addr()
        .expect("local addr")
        .port();
    let url = format!("http://127.0.0.1:{port}");
    let _child = ChildGuard(
        std::process::Command::new(env!("CARGO_BIN_EXE_xavier"))
            .env("XAVIER_HOST", "127.0.0.1")
            .env("XAVIER_PORT", port.to_string())
            .env("XAVIER_TOKEN", "test-token")
            .env(
                "XAVIER_CODE_GRAPH_DB_PATH",
                format!("data/e2e-code-graph-{port}.db"),
            )
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("failed to start xavier binary"),
    );

    let client = reqwest::Client::new();
    let health_url = format!("{url}/health");
    let readiness_url = format!("{url}/readiness");
    let protected_url = format!("{url}/v1/account/usage");
    let mut healthy = false;
    let mut readiness_checked = false;
    let mut auth_checked = false;

    for _ in 0..30 {
        match client.get(&health_url).send().await {
            Ok(response) if response.status().is_success() => {
                assert!(response.headers().contains_key("x-request-id"));
                let body = response.text().await.expect("health body");
                assert!(body.contains("\"status\":\"ok\""));
                healthy = true;

                let readiness = client
                    .get(&readiness_url)
                    .send()
                    .await
                    .expect("readiness response");
                assert!(readiness.status().is_success());
                assert!(readiness.headers().contains_key("x-request-id"));
                let readiness_body = readiness.text().await.expect("readiness body");
                assert!(readiness_body.contains("\"service\":\"xavier\""));
                assert!(
                    readiness_body.contains("\"status\":\"ok\"")
                        || readiness_body.contains("\"status\":\"degraded\"")
                );
                readiness_checked = true;

                let protected = client
                    .get(&protected_url)
                    .send()
                    .await
                    .expect("protected response");
                assert_eq!(protected.status(), reqwest::StatusCode::UNAUTHORIZED);
                assert!(protected.headers().contains_key("x-request-id"));

                let authorized = client
                    .get(&protected_url)
                    .header("X-Xavier-Token", "test-token")
                    .send()
                    .await
                    .expect("authorized response");
                assert!(authorized.status().is_success());
                assert!(authorized.headers().contains_key("x-request-id"));
                let usage: serde_json::Value = authorized.json().await.expect("usage json");
                assert!(usage.get("optimization").is_some());
                assert!(usage["optimization"].get("router_direct_count").is_some());
                assert!(usage["optimization"].get("semantic_cache_hits").is_some());
                auth_checked = true;
                break;
            }
            _ => tokio::time::sleep(Duration::from_millis(500)).await,
        }
    }

    assert!(healthy, "xavier did not expose a healthy /health endpoint");
    assert!(
        readiness_checked,
        "xavier did not expose a valid /readiness endpoint"
    );
    assert!(
        auth_checked,
        "xavier did not enforce authentication on protected routes"
    );
}
