use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::{get, post},
    Router,
};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use xavier2::adapters::outbound::vec::pattern_adapter::PatternAdapter;
use xavier2::agents::RuntimeConfig;
use xavier2::app::qmd_memory_adapter::QmdMemoryAdapter;
use xavier2::app::security_service::SecurityService;
use xavier2::coordination::SimpleAgentRegistry;
use xavier2::memory::file_indexer::{FileIndexer, FileIndexerConfig};
use xavier2::memory::qmd_memory::{MemoryDocument, QmdMemory};
use xavier2::memory::sqlite_vec_store::{VecSqliteMemoryStore, VecSqliteStoreConfig};
use xavier2::memory::surreal_store::MemoryStore;
use xavier2::ports::inbound::MemoryQueryPort;
use xavier2::server::events::{WsEvent, WsMessage};
use xavier2::server::http::{add_handler, ws_events_handler};
use xavier2::workspace::{WorkspaceConfig, WorkspaceRegistry, WorkspaceState};
use xavier2::AppState;

#[tokio::test]
async fn test_websocket_streaming() {
    let temp = tempfile::tempdir().unwrap();
    let db_path = temp.path().join("test_ws.db");

    let mut store_inner = VecSqliteMemoryStore::new(VecSqliteStoreConfig {
        path: db_path,
        embedding_dimensions: 3,
    })
    .await
    .unwrap();

    let (event_tx, _) = tokio::sync::broadcast::channel(100);
    store_inner.set_event_tx(event_tx);
    let store = Arc::new(store_inner);

    let workspace_id = "test_ws_workspace";
    let memory = Arc::new(QmdMemory::new_with_workspace(
        Arc::new(RwLock::new(vec![])),
        workspace_id.to_string(),
    ));
    memory
        .set_store(store.clone() as Arc<dyn MemoryStore>)
        .await;
    memory.init().await.unwrap();
    let memory_port =
        Arc::new(QmdMemoryAdapter::new(Arc::clone(&memory))) as Arc<dyn MemoryQueryPort>;

    let code_db_path = temp.path().join("code_graph.db");
    let code_db = Arc::new(code_graph::db::CodeGraphDB::new(&code_db_path).unwrap());
    let code_indexer = Arc::new(code_graph::indexer::Indexer::new(Arc::clone(&code_db)));
    let code_query = Arc::new(code_graph::query::QueryEngine::new(Arc::clone(&code_db)));

    let state = xavier2::cli::CliState {
        memory: memory_port,
        store: store.clone(),
        workspace_id: workspace_id.to_string(),
        code_db,
        code_indexer,
        code_query,
        security: Arc::new(SecurityService::new()),
        time_store: None,
        agent_registry: Arc::new(SimpleAgentRegistry::new()),
    };

    let app = Router::new()
        .route("/memory/add", post(add_handler))
        .route("/xavier2/events/stream", get(ws_events_handler))
        .with_state(state);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let ws_url = format!("ws://{}/xavier2/events/stream", addr);
    let (mut ws_stream, _) = connect_async(ws_url).await.unwrap();

    // Subscribe to events
    let sub_msg = WsMessage::Subscribe {
        agent_id: Some("test_agent".to_string()),
        project_id: None,
        event_type: None,
    };
    ws_stream
        .send(Message::Text(
            serde_json::to_string(&sub_msg).unwrap().into(),
        ))
        .await
        .unwrap();

    let msg = ws_stream.next().await.unwrap().unwrap();
    let conf: WsEvent = serde_json::from_str(msg.to_text().unwrap()).unwrap();
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
        .unwrap();

    assert_eq!(add_res.status(), StatusCode::OK);

    // Wait for the event
    let msg = ws_stream.next().await.unwrap().unwrap();
    let event: WsEvent = serde_json::from_str(msg.to_text().unwrap()).unwrap();

    if let WsEvent::Event(e) = event {
        assert_eq!(e.agent_id, "test_agent");
        assert_eq!(e.event_type, "memory.add");
        assert!(e.payload["path"].as_str().is_some());
    } else {
        panic!("Expected WsEvent::Event, got {:?}", event);
    }
}
