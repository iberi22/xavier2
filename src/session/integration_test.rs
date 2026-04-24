#[cfg(test)]
mod tests {
    use axum::{
        routing::post,
        Router,
    };
    use std::net::SocketAddr;
    use crate::session::{handle_session_event, SessionEvent};
    use serde_json::json;

    #[tokio::test]
    async fn test_session_webhook_endpoint() {
        let app = Router::new().route("/xavier2/events/session", post(handle_session_event));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let client = reqwest::Client::new();
        let res = client.post(format!("http://{}/xavier2/events/session", addr))
            .json(&json!({
                "session_id": "test-session",
                "event_type": "test-event",
                "payload": { "data": "hello" }
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(res.status(), 200);
        let body: serde_json::Value = res.json().await.unwrap();
        assert_eq!(body["status"], "received");
        assert_eq!(body["session_id"], "test-session");
    }

    #[tokio::test]
    async fn test_event_mapper_to_indexer_flow() {
        use crate::memory::file_indexer::{FileIndexer, FileIndexerConfig};
        use crate::session::{EventMapper, SessionEvent};
        use std::sync::Arc;

        let config = FileIndexerConfig::default();
        let indexer = FileIndexer::new(config, None);
        let mapper = EventMapper::new(indexer);

        let event = SessionEvent {
            session_id: "session-123".to_string(),
            event_type: "user_message".to_string(),
            payload: json!({ "text": "Testing flow" }),
        };

        mapper.map_and_index(event).await.expect("Failed to map and index");

        // The mock indexer we added doesn't persist to QmdMemory in this simple version
        // but we can verify it returned success.
        // In a real scenario, EventMapper would probably use QmdMemory directly.
    }

    #[tokio::test]
    async fn test_auto_verifier_save_retrieve_cycle() {
        use crate::checkpoint::CheckpointManager;
        use crate::session::AutoVerifier;
        use std::sync::Arc;

        let manager = Arc::new(CheckpointManager::new());
        let verifier = AutoVerifier::new(manager);

        let session_id = "session-verify-456";
        let verification_data = json!({ "verified": true, "score": 0.95 });

        verifier.verify_and_save(session_id, verification_data.clone()).await.expect("Failed to save verification");

        let retrieved = verifier.retrieve_verification(session_id).await.expect("Failed to retrieve verification");
        assert_eq!(retrieved, Some(verification_data));
    }

    #[tokio::test]
    async fn test_bidirectional_agent_registration() {
        use crate::agents::registry::{AgentRegistry, AgentCard};

        let registry = AgentRegistry::new();

        let agent_a = AgentCard {
            id: "agent-a".to_string(),
            name: "Agent A".to_string(),
            capabilities: vec!["chat".to_string()],
        };

        let agent_b = AgentCard {
            id: "agent-b".to_string(),
            name: "Agent B".to_string(),
            capabilities: vec!["summarize".to_string()],
        };

        // Register A
        registry.register(agent_a.clone()).await;
        // Register B
        registry.register(agent_b.clone()).await;

        // Verify A can find B
        let found_b = registry.get_agent("agent-b").await.expect("Agent B not found");
        assert_eq!(found_b.name, "Agent B");

        // Verify B can find A
        let found_a = registry.get_agent("agent-a").await.expect("Agent A not found");
        assert_eq!(found_a.name, "Agent A");

        // List all
        let all_agents = registry.list_agents().await;
        assert_eq!(all_agents.len(), 2);
    }
}
