use axum::{
    extract::ws::{Message, WebSocket},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message as WsProtocolMessage};
use xavier::memory::sqlite_vec_store::{VecSqliteMemoryStore, VecSqliteStoreConfig};
use xavier::memory::store::MemoryRecord;
use xavier::memory::store::MemoryStore;
use xavier::server::events::{RealtimeEvent, WsEvent, WsMessage};

/// Inline app state for the websocket integration test.
/// Avoids coupling with the production CliState or AppState.
#[derive(Clone)]
struct TestState {
    store: Arc<VecSqliteMemoryStore>,
    workspace_id: String,
    event_tx: broadcast::Sender<RealtimeEvent>,
}

/// POST /memory/add — insert a record and broadcast the event.
async fn test_add_handler(
    axum::extract::State(state): axum::extract::State<TestState>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let content = payload
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let metadata = payload
        .get("metadata")
        .cloned()
        .unwrap_or(serde_json::json!({}));
    let path = format!("memory/{}", chrono::Utc::now().timestamp());

    let event = RealtimeEvent {
        workspace_id: state.workspace_id.clone(),
        event_id: uuid::Uuid::new_v4().to_string(),
        agent_id: metadata
            .get("_audit")
            .and_then(|a| a.get("agent_id"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        project_id: None,
        event_type: metadata
            .get("_audit")
            .and_then(|a| a.get("operation"))
            .and_then(|v| v.as_str())
            .unwrap_or("memory.add")
            .to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        payload: serde_json::json!({ "path": path, "content": content }),
    };

    let record = MemoryRecord::from_document(
        &state.workspace_id,
        &xavier::memory::qmd_memory::MemoryDocument {
            id: Some(event.event_type.clone()),
            path: path.clone(),
            content: content.to_string(),
            metadata: metadata.clone(),
            content_vector: None,
            embedding: vec![],
        },
        true,
        None,
    );

    let _ = state.store.put(record).await;
    let _ = state.event_tx.send(event);

    (StatusCode::OK, Json(serde_json::json!({"status": "ok"})))
}

/// GET /xavier/events/stream — WebSocket upgrade.
async fn test_ws_handler(
    ws: axum::extract::ws::WebSocketUpgrade,
    axum::extract::State(state): axum::extract::State<TestState>,
) -> impl IntoResponse {
    let rx = state.event_tx.subscribe();
    ws.on_upgrade(move |socket| handle_test_ws(socket, rx))
}

async fn handle_test_ws(mut socket: WebSocket, mut event_rx: broadcast::Receiver<RealtimeEvent>) {
    let mut subscribed = false;

    loop {
        tokio::select! {
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text)))
                        if text.contains("Subscribe") || text.contains("subscribe") =>
                    {
                        let confirm = WsEvent::SubscriptionConfirmed;
                        let _ = socket.send(Message::Text(
                            serde_json::to_string(&confirm).expect("test assertion").into(),
                        )).await;
                        subscribed = true;
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
            event = event_rx.recv() => {
                match event {
                    Ok(e) if subscribed => {
                        let ws_event = WsEvent::Event(e);
                        let _ = socket.send(Message::Text(
                            serde_json::to_string(&ws_event).expect("test assertion").into(),
                        )).await;
                    }
                    _ => {}
                }
            }
        }
    }
}

#[tokio::test]
async fn test_websocket_streaming() {
    let temp = tempfile::tempdir().expect("test assertion");
    let db_path = temp.path().join("test_ws.db");

    let mut store_inner = VecSqliteMemoryStore::new(VecSqliteStoreConfig {
        path: db_path,
        embedding_dimensions: 3,
    })
    .await
    .expect("test assertion");

    let (event_tx, _) = broadcast::channel(100);
    store_inner.set_event_tx(event_tx.clone());
    let store = Arc::new(store_inner);

    let test_state = TestState {
        store: store.clone(),
        workspace_id: "test_ws_workspace".to_string(),
        event_tx: event_tx.clone(),
    };

    let app = Router::new()
        .route("/memory/add", post(test_add_handler))
        .route("/xavier/events/stream", get(test_ws_handler))
        .with_state(test_state);

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("test assertion");
    let addr = listener.local_addr().expect("test assertion");

    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("test assertion");
    });

    let ws_url = format!("ws://{}/xavier/events/stream", addr);
    let (mut ws_stream, _) = connect_async(ws_url).await.expect("test assertion");

    // Subscribe to events
    let sub_msg = WsMessage::Subscribe {
        agent_id: Some("test_agent".to_string()),
        project_id: None,
        event_type: None,
    };
    ws_stream
        .send(WsProtocolMessage::Text(
            serde_json::to_string(&sub_msg).expect("test assertion").into(),
        ))
        .await
        .expect("test assertion");

    let msg = ws_stream.next().await.expect("test assertion").expect("test assertion");
    let conf: WsEvent = serde_json::from_str(msg.to_text().expect("test assertion")).expect("test assertion");
    assert!(matches!(conf, WsEvent::SubscriptionConfirmed));

    // Add a memory record via HTTP
    let client = reqwest::Client::new();
    let add_res = client
        .post(format!("http://{}/memory/add", addr))
        .json(&serde_json::json!({
            "content": "Hello real-time world",
            "metadata": {
                "_audit": {
                    "agent_id": "test_agent",
                    "operation": "memory.add"
                }
            }
        }))
        .send()
        .await
        .expect("test assertion");

    assert_eq!(add_res.status(), StatusCode::OK);

    // Wait for the event
    let msg = ws_stream.next().await.expect("test assertion").expect("test assertion");
    let event: WsEvent = serde_json::from_str(msg.to_text().expect("test assertion")).expect("test assertion");

    if let WsEvent::Event(e) = event {
        assert_eq!(e.agent_id, "test_agent");
        assert_eq!(e.event_type, "memory.add");
        assert!(e.payload["path"].as_str().is_some());
    } else {
        panic!("Expected WsEvent::Event, got {:?}", event);
    }
}
