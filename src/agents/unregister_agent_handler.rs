use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Json,
};

use crate::adapters::inbound::http::state::AppState;
use crate::ports::inbound::AgentLifecyclePort;

pub async fn unregister_agent_handler(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
) -> Json<serde_json::Value> {
    let success = state.agent_lifecycle.unregister(&agent_id).await;

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
    use crate::adapters::inbound::http::state::AppState;
    use crate::coordination::SimpleAgentRegistry;
    use crate::ports::inbound::AgentLifecyclePort;
    use axum::{
        extract::{Path, State},
        Json,
    };
    use serde_json::json;
    use std::sync::Arc;

    struct SessionSyncMock;

    #[async_trait::async_trait]
    impl crate::ports::inbound::SessionSyncPort for SessionSyncMock {
        async fn check(&self) -> anyhow::Result<crate::tasks::session_sync_task::SyncCheckResult> {
            Ok(Default::default())
        }
        async fn last_result(&self) -> crate::tasks::session_sync_task::SyncCheckResult {
            Default::default()
        }
    }

    struct SessionMock;

    #[async_trait::async_trait]
    impl crate::ports::inbound::SessionPort for SessionMock {
        async fn handle_event(&self, _event: crate::session::types::SessionEvent) -> bool {
            true
        }
        async fn handle_and_index_event(
            &self,
            event: crate::session::types::SessionEvent,
        ) -> anyhow::Result<crate::ports::inbound::session_port::SessionEventResult> {
            Ok(crate::ports::inbound::session_port::SessionEventResult {
                status: "ok".to_string(),
                session_id: event.session_id,
                memory_id: None,
                mapped: true,
            })
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

        let state = AppState {
            memory: Arc::new(crate::app::qmd_memory_adapter::QmdMemoryAdapter::new(
                Arc::new(crate::memory::qmd_memory::QmdMemory::new(Arc::new(
                    tokio::sync::RwLock::new(Vec::new()),
                ))),
            )),
            security: Arc::new(crate::app::security_service::SecurityService::new()),
            security_scan: Arc::new(crate::app::security_service::SecurityService::new()),
            time_metrics: Arc::new(
                crate::adapters::inbound::http::time_metrics_adapter::TimeMetricsAdapter::new(
                    Arc::new(crate::time::TimeMetricsStore::new(Arc::new(
                        parking_lot::Mutex::new(rusqlite::Connection::open_in_memory().unwrap()),
                    ))),
                ),
            ),
            agent_lifecycle: registry.clone(),
            health: Arc::new(crate::app::health_service::HealthService::new()),
            verification: Arc::new(crate::app::verification_service::VerificationService::new()),
            session_sync: Arc::new(SessionSyncMock),
            session: Arc::new(SessionMock),
            workspace_id: "test".to_string(),
            auth_token: "test-token".to_string(),
            code_db: Arc::new(code_graph::db::CodeGraphDB::in_memory().unwrap()),
            code_indexer: Arc::new(code_graph::indexer::Indexer::new(Arc::new(
                code_graph::db::CodeGraphDB::in_memory().unwrap(),
            ))),
            code_query: Arc::new(code_graph::query::QueryEngine::new(Arc::new(
                code_graph::db::CodeGraphDB::in_memory().unwrap(),
            ))),
        };

        let Json(payload) = unregister_agent_handler(State(state), Path("agent-delete-1".to_string()))
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
        let registry = SimpleAgentRegistry::new();
        let state = AppState {
            memory: Arc::new(crate::app::qmd_memory_adapter::QmdMemoryAdapter::new(
                Arc::new(crate::memory::qmd_memory::QmdMemory::new(Arc::new(
                    tokio::sync::RwLock::new(Vec::new()),
                ))),
            )),
            security: Arc::new(crate::app::security_service::SecurityService::new()),
            security_scan: Arc::new(crate::app::security_service::SecurityService::new()),
            time_metrics: Arc::new(
                crate::adapters::inbound::http::time_metrics_adapter::TimeMetricsAdapter::new(
                    Arc::new(crate::time::TimeMetricsStore::new(Arc::new(
                        parking_lot::Mutex::new(rusqlite::Connection::open_in_memory().unwrap()),
                    ))),
                ),
            ),
            agent_lifecycle: registry.clone(),
            health: Arc::new(crate::app::health_service::HealthService::new()),
            verification: Arc::new(crate::app::verification_service::VerificationService::new()),
            session_sync: Arc::new(SessionSyncMock),
            session: Arc::new(SessionMock),
            workspace_id: "test".to_string(),
            auth_token: "test-token".to_string(),
            code_db: Arc::new(code_graph::db::CodeGraphDB::in_memory().unwrap()),
            code_indexer: Arc::new(code_graph::indexer::Indexer::new(Arc::new(
                code_graph::db::CodeGraphDB::in_memory().unwrap(),
            ))),
            code_query: Arc::new(code_graph::query::QueryEngine::new(Arc::new(
                code_graph::db::CodeGraphDB::in_memory().unwrap(),
            ))),
        };

        let Json(payload) = unregister_agent_handler(State(state), Path("missing-agent".to_string()))
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
