use axum::{
    extract::{Path, State},
    Json,
};

use crate::AppState;

pub async fn unregister_agent_handler(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
) -> Json<serde_json::Value> {
    let success = state.agent_registry.unregister(&agent_id).await;

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
    use crate::{
        adapters::outbound::vec::pattern_adapter::PatternAdapter,
        coordination::SimpleAgentRegistry,
        memory::file_indexer::{FileIndexer, FileIndexerConfig},
        ports::inbound::NoopTimeMetricsPort,
        workspace::WorkspaceRegistry,
        AppState,
    };
    use axum::{
        extract::{Path, State},
        Json,
    };
    use serde_json::json;
    use std::sync::Arc;

    fn test_state(registry: Arc<SimpleAgentRegistry>) -> AppState {
        let code_db = Arc::new(code_graph::db::CodeGraphDB::in_memory().unwrap());
        let code_indexer = Arc::new(code_graph::indexer::Indexer::new(Arc::clone(&code_db)));
        let code_query = Arc::new(code_graph::query::QueryEngine::new(Arc::clone(&code_db)));

        AppState {
            workspace_id: "test".to_string(),
            workspace_registry: Arc::new(WorkspaceRegistry::new()),
            indexer: FileIndexer::new(FileIndexerConfig::default(), Some(code_indexer.clone())),
            code_indexer,
            code_query,
            code_db,
            pattern_adapter: Arc::new(PatternAdapter::new()),
            security_service: Arc::new(crate::app::security_service::SecurityService::new()),
            time_metrics: Arc::new(NoopTimeMetricsPort),
            agent_registry: registry,
        }
    }

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
            State(test_state(registry.clone())),
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
            State(test_state(SimpleAgentRegistry::new())),
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
