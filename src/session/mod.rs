pub mod integration_test;

use serde::{Deserialize, Serialize};
use axum::{Json, response::IntoResponse};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionEvent {
    pub session_id: String,
    pub event_type: String,
    pub payload: serde_json::Value,
}

pub async fn handle_session_event(
    Json(event): Json<SessionEvent>,
) -> impl IntoResponse {
    tracing::info!("Received session event: {:?}", event);
    Json(serde_json::json!({ "status": "received", "session_id": event.session_id }))
}

pub struct EventMapper {
    pub indexer: crate::memory::file_indexer::FileIndexer,
}

impl EventMapper {
    pub fn new(indexer: crate::memory::file_indexer::FileIndexer) -> Self {
        Self { indexer }
    }

    pub async fn map_and_index(&self, event: SessionEvent) -> anyhow::Result<()> {
        let content = serde_json::to_string(&event.payload)?;
        let path = format!("events/{}/{}", event.session_id, event.event_type);
        self.indexer.index_file_content(&path, &content).await?;
        Ok(())
    }
}

pub struct AutoVerifier {
    pub checkpoint_manager: std::sync::Arc<crate::checkpoint::CheckpointManager>,
}

impl AutoVerifier {
    pub fn new(checkpoint_manager: std::sync::Arc<crate::checkpoint::CheckpointManager>) -> Self {
        Self { checkpoint_manager }
    }

    pub async fn verify_and_save(&self, session_id: &str, data: serde_json::Value) -> anyhow::Result<()> {
        let checkpoint = crate::checkpoint::Checkpoint::new(
            session_id.to_string(),
            "auto_verify".to_string(),
            data,
        );
        self.checkpoint_manager.save(checkpoint).await?;
        Ok(())
    }

    pub async fn retrieve_verification(&self, session_id: &str) -> anyhow::Result<Option<serde_json::Value>> {
        let checkpoint = self.checkpoint_manager.load(session_id.to_string(), "auto_verify".to_string()).await?;
        Ok(checkpoint.map(|c| c.data))
    }
}
