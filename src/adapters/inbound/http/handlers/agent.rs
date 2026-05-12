use crate::adapters::inbound::http::AppState;
use crate::domain::agent::AgentMetadata;
use axum::{extract::State, Json};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AgentRegisterPayload {
    pub agent_id: String,
    pub session_id: String,
    pub name: Option<String>,
    pub capabilities: Option<Vec<String>>,
    pub role: Option<String>,
}

pub async fn agent_register_handler(
    State(state): State<AppState>,
    Json(payload): Json<AgentRegisterPayload>,
) -> Json<serde_json::Value> {
    let metadata = AgentMetadata {
        name: payload.name,
        capabilities: payload.capabilities.unwrap_or_default(),
        role: payload.role,
        endpoint: None,
    };

    let success = state
        .agent_lifecycle
        .register(
            payload.agent_id.clone(),
            payload.session_id.clone(),
            metadata,
        )
        .await;

    Json(serde_json::json!({
        "status": if success { "ok" } else { "error" },
        "agent_id": payload.agent_id,
        "session_id": payload.session_id,
        "message": if success { "Agent registered successfully" } else { "Registration failed" },
    }))
}

pub async fn agent_heartbeat_handler(
    State(state): State<AppState>,
    axum::extract::Path(agent_id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let success = state.agent_lifecycle.heartbeat(&agent_id).await;

    Json(serde_json::json!({
        "status": if success { "ok" } else { "error" },
        "agent_id": agent_id,
        "message": if success { "Heartbeat recorded" } else { "Agent not found" },
    }))
}

pub async fn agent_active_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    let active = state.agent_lifecycle.get_active_agents().await;

    Json(serde_json::json!({
        "status": "ok",
        "active_agents": active.len(),
        "agents": active,
    }))
}

pub async fn agent_unregister_handler(
    State(state): State<AppState>,
    axum::extract::Path(agent_id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let success = state.agent_lifecycle.unregister(&agent_id).await;

    Json(serde_json::json!({
        "status": if success { "ok" } else { "error" },
        "agent_id": agent_id,
        "message": if success { "Agent unregistered" } else { "Agent not found" },
    }))
}

#[derive(Debug, Deserialize)]
pub struct AgentPushContextPayload {
    pub content: String,
    pub importance: Option<f32>,
    pub tags: Option<Vec<String>>,
}

pub async fn agent_push_context_handler(
    State(_state): State<AppState>,
    axum::extract::Path(agent_id): axum::extract::Path<String>,
    Json(payload): Json<AgentPushContextPayload>,
) -> Json<serde_json::Value> {
    // In a real implementation, this would use a port to add context to the memory store
    // tied to the agent's session.
    let importance = payload.importance.unwrap_or(0.5);

    // Placeholder logic for now, similar to what might be in cli.rs
    Json(serde_json::json!({
        "status": "ok",
        "agent_id": agent_id,
        "message": "Context pushed (placeholder)",
        "importance": importance,
    }))
}
