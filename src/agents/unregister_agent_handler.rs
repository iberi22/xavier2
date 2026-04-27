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
