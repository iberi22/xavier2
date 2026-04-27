use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Json,
};

use crate::coordination::SimpleAgentRegistry;

pub async fn unregister_agent_handler(
    State(registry): State<Arc<SimpleAgentRegistry>>,
    Path(agent_id): Path<String>,
) -> Json<serde_json::Value> {
    let success = registry.unregister(&agent_id).await;

    Json(serde_json::json!({
        "status": if success { "ok" } else { "error" },
        "agent_id": agent_id,
        "message": if success {
            "Agent unregistered"
        } else {
            "Agent not found or already unregistered"
        },
    }))
}

#[cfg(test)]
mod tests {
    use super::unregister_agent_handler;
    use crate::coordination::SimpleAgentRegistry;
    use axum::{extract::{Path, State}, Json};
    use serde_json::json;

    #[tokio::test]
    async fn unregister_existing_agent_returns_success_payload() {
        let registry = SimpleAgentRegistry::new();
        registry
            .register(
                "agent-delete-1".to_string(),
                "session-delete-1".to_string(),
                Default::default(),
            )
            .await;

        let Json(payload) = unregister_agent_handler(
            State(registry.clone()),
            Path("agent-delete-1".to_string()),
        )
        .await;

        assert_eq!(
            payload,
            json!({
                "status": "ok",
                "agent_id": "agent-delete-1",
                "message": "Agent unregistered",
            })
        );
        assert!(registry.get("agent-delete-1").await.is_none());
    }

    #[tokio::test]
    async fn unregister_missing_agent_returns_error_payload() {
        let Json(payload) = unregister_agent_handler(
            State(SimpleAgentRegistry::new()),
            Path("missing-agent".to_string()),
        )
        .await;

        assert_eq!(
            payload,
            json!({
                "status": "error",
                "agent_id": "missing-agent",
                "message": "Agent not found or already unregistered",
            })
        );
    }
}
