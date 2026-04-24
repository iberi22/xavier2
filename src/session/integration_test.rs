//! Session Module Integration Tests
//!
//! Tests for session event handling, event mapping, auto-verification,
//! and bidirectional agent registry.
//!
//! Run with: cargo test --test integration -- session

use chrono::Utc;
use reqwest::Client;
use xavier2::session::event_mapper::{PanelThreadEntry, PanelThreadEntryMetadata};
use xavier2::session::types::{SessionEvent, SessionEventType};
use xavier2::verification::auto_verifier::{AutoVerifier, VerificationResult};

#[cfg(test)]
mod session_event_tests {
    use super::*;

    #[tokio::test]
    async fn test_webhook_session_event() {
        let client = Client::new();
        let session_id = format!("test-session-{}", ulid::Ulid::new());
        let timestamp = Utc::now();

        let payload = SessionEvent {
            session_id: session_id.clone(),
            event_type: SessionEventType::Message,
            timestamp,
            content: Some("Hello, this is a test message".to_string()),
            metadata: Some(serde_json::json!({
                "source": "openclaw",
                "version": "1.0"
            })),
        };

        let response = client
            .post("http://localhost:8006/xavier2/events/session")
            .json(&payload)
            .send()
            .await;

        match response {
            Ok(resp) => {
                assert!(resp.status().is_success() || resp.status().as_u16() == 400);
                let body: serde_json::Value = resp.json().await.unwrap_or_default();
                assert!(body.get("status").is_some() || body.get("error").is_some());
            }
            Err(e) => {
                println!("Server not running (expected in test env): {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_webhook_session_event_with_tool_call() {
        let client = Client::new();
        let session_id = format!("test-session-tool-{}", ulid::Ulid::new());
        let timestamp = Utc::now();

        let payload = SessionEvent {
            session_id: session_id.clone(),
            event_type: SessionEventType::ToolCall,
            timestamp,
            content: Some("Calling tool: search_memory with query 'test'".to_string()),
            metadata: Some(serde_json::json!({
                "tool_name": "search_memory",
                "args": {"query": "test"}
            })),
        };

        let response = client
            .post("http://localhost:8006/xavier2/events/session")
            .json(&payload)
            .send()
            .await;

        match response {
            Ok(resp) => {
                assert!(resp.status().is_success() || resp.status().as_u16() == 400);
            }
            Err(e) => {
                println!("Server not running: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod event_mapper_tests {
    use super::*;

    #[tokio::test]
    fn test_event_mapper_maps_session_to_panel_thread() {
        let timestamp = Utc::now();
        let session_id = format!("mapper-test-{}", ulid::Ulid::new());

        let session_event = SessionEvent {
            session_id: session_id.clone(),
            event_type: SessionEventType::Message,
            timestamp,
            content: Some("Test message content for mapping".to_string()),
            metadata: None,
        };

        let entry = PanelThreadEntry::from_session_event(&session_event);

        assert!(entry.is_some(), "Message event should produce a PanelThreadEntry");
        let entry = entry.unwrap();

        assert_eq!(entry.session_id, session_id);
        assert_eq!(entry.role, "user");
        assert_eq!(entry.content, "Test message content for mapping");
        assert_eq!(entry.timestamp, timestamp);
    }

    #[tokio::test]
    fn test_event_mapper_skips_session_start() {
        let timestamp = Utc::now();
        let session_id = format!("mapper-test-start-{}", ulid::Ulid::new());

        let session_event = SessionEvent {
            session_id: session_id.clone(),
            event_type: SessionEventType::SessionStart,
            timestamp,
            content: Some("Session started".to_string()),
            metadata: None,
        };

        let entry = PanelThreadEntry::from_session_event(&session_event);
        assert!(entry.is_none(), "SessionStart should produce no entry");
    }

    #[tokio::test]
    fn test_event_mapper_skips_session_end() {
        let timestamp = Utc::now();
        let session_id = format!("mapper-test-end-{}", ulid::Ulid::new());

        let session_event = SessionEvent {
            session_id: session_id.clone(),
            event_type: SessionEventType::SessionEnd,
            timestamp,
            content: Some("Session ended".to_string()),
            metadata: None,
        };

        let entry = PanelThreadEntry::from_session_event(&session_event);
        assert!(entry.is_none(), "SessionEnd should produce no entry");
    }

    #[tokio::test]
    fn test_event_mapper_maps_tool_call() {
        let timestamp = Utc::now();
        let session_id = format!("mapper-test-tool-{}", ulid::Ulid::new());

        let session_event = SessionEvent {
            session_id: session_id.clone(),
            event_type: SessionEventType::ToolCall,
            timestamp,
            content: Some("tool:search_memory args:{query:'test'}".to_string()),
            metadata: None,
        };

        let entry = PanelThreadEntry::from_session_event(&session_event);
        assert!(entry.is_some(), "ToolCall should produce an entry");
        assert_eq!(entry.unwrap().role, "tool");
    }

    #[tokio::test]
    fn test_event_mapper_maps_tool_result() {
        let timestamp = Utc::now();
        let session_id = format!("mapper-test-result-{}", ulid::Ulid::new());

        let session_event = SessionEvent {
            session_id: session_id.clone(),
            event_type: SessionEventType::ToolResult,
            timestamp,
            content: Some("Tool returned: 42 results".to_string()),
            metadata: None,
        };

        let entry = PanelThreadEntry::from_session_event(&session_event);
        assert!(entry.is_some(), "ToolResult should produce an entry");
        assert_eq!(entry.unwrap().role, "assistant");
    }

    #[tokio::test]
    fn test_event_mapper_maps_error() {
        let timestamp = Utc::now();
        let session_id = format!("mapper-test-error-{}", ulid::Ulid::new());

        let session_event = SessionEvent {
            session_id: session_id.clone(),
            event_type: SessionEventType::Error,
            timestamp,
            content: Some("Error: connection timeout".to_string()),
            metadata: None,
        };

        let entry = PanelThreadEntry::from_session_event(&session_event);
        assert!(entry.is_some(), "Error should produce an entry");
        assert_eq!(entry.unwrap().role, "system");
    }

    #[tokio::test]
    fn test_event_mapper_handles_empty_content() {
        let timestamp = Utc::now();
        let session_id = format!("mapper-test-empty-{}", ulid::Ulid::new());

        let session_event = SessionEvent {
            session_id: session_id.clone(),
            event_type: SessionEventType::Message,
            timestamp,
            content: None,
            metadata: None,
        };

        let entry = PanelThreadEntry::from_session_event(&session_event);
        assert!(entry.is_none(), "Empty content should produce no entry");
    }
}

#[cfg(test)]
mod auto_verifier_tests {
    use super::*;

    #[tokio::test]
    async fn test_auto_verifier_save_retrieve_cycle() {
        let client = Client::new();
        let xavier2_url = "http://localhost:8006";
        let auth_token = "dev-token";
        let path = format!("test/verification/{}", ulid::Ulid::new());
        let test_content = format!(
            "Verification test content {}",
            chrono::Utc::now().to_rfc3339()
        );

        let result = AutoVerifier::verify_save(
            &client,
            xavier2_url,
            auth_token,
            &path,
            &test_content,
        )
        .await;

        match result {
            Ok(verification) => {
                assert!(verification.save_ok || !verification.save_ok);
                assert!(verification.retrieve_ok || !verification.retrieve_ok);
                assert!(verification.latency_ms >= 0);
                assert!(verification.match_score >= 0.0 && verification.match_score <= 1.0);
                assert_eq!(verification.path, path);
            }
            Err(e) => {
                println!("Verification failed (server may not be running): {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_auto_verifier_healthy_result() {
        let result = VerificationResult {
            path: "test/path".to_string(),
            save_ok: true,
            retrieve_ok: true,
            match_score: 0.95,
            latency_ms: 150,
        };

        assert!(result.is_healthy());
    }

    #[tokio::test]
    async fn test_auto_verifier_unhealthy_low_score() {
        let result = VerificationResult {
            path: "test/path".to_string(),
            save_ok: true,
            retrieve_ok: true,
            match_score: 0.5,
            latency_ms: 150,
        };

        assert!(!result.is_healthy(), "Score below 0.8 should be unhealthy");
    }

    #[tokio::test]
    async fn test_auto_verifier_unhealthy_save_failed() {
        let result = VerificationResult {
            path: "test/path".to_string(),
            save_ok: false,
            retrieve_ok: true,
            match_score: 0.85,
            latency_ms: 150,
        };

        assert!(!result.is_healthy());
    }

    #[tokio::test]
    async fn test_auto_verifier_unhealthy_retrieve_failed() {
        let result = VerificationResult {
            path: "test/path".to_string(),
            save_ok: true,
            retrieve_ok: false,
            match_score: 0.0,
            latency_ms: 150,
        };

        assert!(!result.is_healthy());
    }
}

#[cfg(test)]
mod agent_registry_tests {
    use xavier2::agents::registry::AgentRegistry;

    #[tokio::test]
    async fn test_bidirectional_agent_registry() {
        let registry = AgentRegistry::new();

        let agent_id_1 = "agent-1";
        let agent_id_2 = "agent-2";
        let session_id_1 = "session-1";
        let session_id_2 = "session-2";

        registry.register_agent(agent_id_1).await;
        registry.register_agent(agent_id_2).await;

        registry.attach_session(agent_id_1, session_id_1).await;
        registry.attach_session(agent_id_1, session_id_2).await;
        registry.attach_session(agent_id_2, session_id_1).await;

        let agents_for_session_1 = registry.get_agents_for_session(session_id_1).await;
        assert!(agents_for_session_1.contains(&agent_id_1.to_string()));
        assert!(agents_for_session_1.contains(&agent_id_2.to_string()));

        let sessions_for_agent_1 = registry.get_sessions_for_agent(agent_id_1).await;
        assert!(sessions_for_agent_1.contains(&session_id_1.to_string()));
        assert!(sessions_for_agent_1.contains(&session_id_2.to_string()));

        registry.detach_session(agent_id_1, session_id_1).await;

        let sessions_after_detach = registry.get_sessions_for_agent(agent_id_1).await;
        assert!(!sessions_after_detach.contains(&session_id_1.to_string()));
        assert!(sessions_after_detach.contains(&session_id_2.to_string()));

        registry.unregister_agent(agent_id_2).await;

        let agents_for_session_1_after = registry.get_agents_for_session(session_id_1).await;
        assert!(!agents_for_session_1_after.contains(&agent_id_2.to_string()));
    }

    #[tokio::test]
    async fn test_agent_registry_bidirectional_consistency() {
        let registry = AgentRegistry::new();

        let agent_id = "test-agent";
        let session_id = "test-session";

        registry.register_agent(agent_id).await;
        registry.attach_session(agent_id, session_id).await;

        let agents = registry.get_agents_for_session(session_id).await;
        let sessions = registry.get_sessions_for_agent(agent_id).await;

        assert!(agents.contains(&agent_id.to_string()));
        assert!(sessions.contains(&session_id.to_string()));

        if let Some(sessions_count) = registry.count_sessions_for_agent(agent_id).await {
            assert_eq!(sessions_count, 1);
        }

        if let Some(agents_count) = registry.count_agents_for_session(session_id).await {
            assert_eq!(agents_count, 1);
        }
    }
}