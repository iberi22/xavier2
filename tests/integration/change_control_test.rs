use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use std::sync::Arc;
use xavier::ports::inbound::change_control_port::ChangeControlPort;
use xavier::app::change_control_service::ChangeControlService;
use xavier::adapters::inbound::http::handlers::change_control;
use axum::{Router, routing::{get, post}, middleware};
use xavier::cli::auth_middleware;
use xavier::cli::auth_middleware;


struct TestServer {
    base_url: String,
    client: Client,
    _handle: JoinHandle<()>,
    token: String,
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self._handle.abort();
    }
}

async fn spawn_test_server() -> TestServer {
    let token = "test-token-change-control".to_string();
    std::env::set_var("XAVIER_TOKEN", &token);

    let change_control_port = Arc::new(ChangeControlService::new()) as Arc<dyn ChangeControlPort>;

    let change_control_routes = Router::new()
        .route("/change/tasks", post(change_control::create_task_handler))
        .route("/change/tasks/{id}", get(change_control::get_task_handler))
        .route("/change/leases/claim", post(change_control::claim_lease_handler))
        .route("/change/leases/release", post(change_control::release_lease_handler))
        .route("/change/leases/active", get(change_control::active_leases_handler))
        .route("/change/conflicts/check", post(change_control::check_conflicts_handler))
        .route("/change/validate", post(change_control::validate_handler))
        .route("/change/complete", post(change_control::complete_task_handler))
        .route("/change/merge-plan", get(change_control::merge_plan_handler))
        .layer(middleware::from_fn(auth_middleware))
        .with_state(change_control_port);

    let app = Router::new().merge(change_control_routes);

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind random test port");
    let addr = listener.local_addr().expect("read local address");
    let base_url = format!("http://{addr}");
    let _handle = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("test server should serve");
    });

    let client = Client::new();
    TestServer {
        base_url,
        client,
        _handle,
        token,
    }
}

#[tokio::test]
async fn test_change_control_workflow() {
    let server = spawn_test_server().await;

    // 1. Create a task
    let create_task_payload = json!({
        "agent_id": "test-agent-01",
        "title": "Refactor Memory",
        "intent": "Refactor semantic cache for better performance",
        "scope": {
            "allowed_read": ["src/memory/**"],
            "allowed_write": ["src/memory/semantic_cache.rs"],
            "blocked": ["src/domain/**"],
            "contracts_affected": ["MemoryPort"]
        }
    });

    let resp = server.client
        .post(format!("{}/change/tasks", server.base_url))
        .header("X-Xavier-Token", &server.token)
        .json(&create_task_payload)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    let task_id = body["task_id"].as_str().unwrap().to_string();
    assert_eq!(body["status"], "created");

    // 2. Claim a lease
    let claim_lease_payload = json!({
        "agent_id": "test-agent-01",
        "task_id": task_id,
        "patterns": ["src/memory/semantic_cache.rs"],
        "mode": "write",
        "ttl_seconds": 3600
    });

    let resp = server.client
        .post(format!("{}/change/leases/claim", server.base_url))
        .header("X-Xavier-Token", &server.token)
        .json(&claim_lease_payload)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "granted");
    let lease_id = body["lease_id"].as_str().unwrap().to_string();
    assert!(body["required_checks"].as_array().unwrap().len() > 0);

    // 3. List active leases
    let resp = server.client
        .get(format!("{}/change/leases/active", server.base_url))
        .header("X-Xavier-Token", &server.token)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    let leases = body.as_array().unwrap();
    assert!(leases.iter().any(|l| l["id"] == lease_id));

    // 4. Complete task
    let complete_payload = json!({
        "task_id": task_id,
        "result": {
            "files_changed": 1,
            "tests_passed": true
        }
    });

    let resp = server.client
        .post(format!("{}/change/complete", server.base_url))
        .header("X-Xavier-Token", &server.token)
        .json(&complete_payload)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["task_id"], task_id);
    assert!(body["summary"].as_str().unwrap().contains("completed") || body["summary"].as_str().unwrap().contains("modified"));
}

#[tokio::test]
async fn test_change_control_auth() {
    let server = spawn_test_server().await;

    let resp = server.client
        .get(format!("{}/change/leases/active", server.base_url))
        .header("X-Xavier-Token", "wrong-token")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
