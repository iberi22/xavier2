//! HTTP handlers for the Change Control API (ADR-006).
//!
//! These handlers use axum's `State<Arc<dyn ChangeControlPort>>` extractor.
//! The routes are registered as a separate router that gets its state from
//! `cli.rs` via `.with_state(state.change_control.clone() as Arc<dyn ChangeControlPort>)`.
//! The outer app router merges this sub-router alongside the protected routes.

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::domain::change_control::{
    AgentTask, AgentTaskStatus, ChangeScope, ConflictReport, FileLease, LeaseMode, RiskLevel,
};
use crate::ports::inbound::change_control_port::{
    ChangeControlPort, LeaseResponse, MergePlan, TaskCompletionResult, ValidationResult,
};

// ---------------------------------------------------------------------------
// Request DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct CreateTaskRequest {
    pub agent_id: String,
    pub title: String,
    pub intent: String,
    pub scope: ChangeScope,
}

#[derive(Debug, Deserialize)]
pub struct ClaimLeaseRequest {
    pub agent_id: String,
    pub task_id: String,
    pub patterns: Vec<String>,
    pub mode: String,
    pub ttl_seconds: i64,
}

#[derive(Debug, Deserialize)]
pub struct ReleaseLeaseRequest {
    pub lease_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ConflictCheckRequest {
    pub task_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ValidateRequest {
    pub scope: ChangeScope,
}

#[derive(Debug, Deserialize)]
pub struct CompleteTaskRequest {
    pub task_id: String,
    pub result: serde_json::Value,
}

// ---------------------------------------------------------------------------
// Response DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct CreateTaskResponse {
    pub task_id: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct ReleaseLeaseResponse {
    pub status: String,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// POST /change/tasks
pub async fn create_task_handler(
    State(service): State<Arc<dyn ChangeControlPort>>,
    Json(payload): Json<CreateTaskRequest>,
) -> Result<Json<CreateTaskResponse>, (StatusCode, Json<serde_json::Value>)> {
    let task = AgentTask {
        id: String::new(),
        title: payload.title,
        capability: "change_control".to_string(),
        agent_id: payload.agent_id,
        status: AgentTaskStatus::Draft,
        intent: payload.intent,
        scope: payload.scope,
        risk_level: RiskLevel::Low,
        dependencies: Vec::new(),
        memory_refs: Vec::new(),
        created_at: 0,
        updated_at: 0,
    };

    match service.create_task(task).await {
        Ok(task_id) => Ok(Json(CreateTaskResponse {
            task_id,
            status: "created".to_string(),
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "status": "error",
                "message": e.to_string(),
            })),
        )),
    }
}

/// GET /change/tasks/:id
pub async fn get_task_handler(
    State(service): State<Arc<dyn ChangeControlPort>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match service.get_task(&id).await {
        Ok(Some(task)) => Ok(Json(serde_json::to_value(task).unwrap_or_default())),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "status": "not_found",
                "message": format!("Task '{}' not found", id),
            })),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "status": "error",
                "message": e.to_string(),
            })),
        )),
    }
}

/// POST /change/leases/claim
pub async fn claim_lease_handler(
    State(service): State<Arc<dyn ChangeControlPort>>,
    Json(payload): Json<ClaimLeaseRequest>,
) -> Result<Json<LeaseResponse>, (StatusCode, Json<serde_json::Value>)> {
    let mode = match payload.mode.to_lowercase().as_str() {
        "read" => LeaseMode::Read,
        "write" => LeaseMode::Write,
        "block" => LeaseMode::Block,
        other => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "status": "error",
                    "message": format!(
                        "Invalid lease mode '{}'. Use 'read', 'write', or 'block'.",
                        other
                    ),
                })),
            ));
        }
    };

    match service
        .claim_lease(
            &payload.agent_id,
            &payload.task_id,
            payload.patterns,
            mode,
            payload.ttl_seconds,
        )
        .await
    {
        Ok(response) => Ok(Json(response)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "status": "error",
                "message": e.to_string(),
            })),
        )),
    }
}

/// POST /change/leases/release
pub async fn release_lease_handler(
    State(service): State<Arc<dyn ChangeControlPort>>,
    Json(payload): Json<ReleaseLeaseRequest>,
) -> Result<Json<ReleaseLeaseResponse>, (StatusCode, Json<serde_json::Value>)> {
    match service.release_lease(&payload.lease_id).await {
        Ok(()) => Ok(Json(ReleaseLeaseResponse {
            status: "released".to_string(),
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "status": "error",
                "message": e.to_string(),
            })),
        )),
    }
}

/// GET /change/leases/active
pub async fn active_leases_handler(
    State(service): State<Arc<dyn ChangeControlPort>>,
) -> Result<Json<Vec<FileLease>>, (StatusCode, Json<serde_json::Value>)> {
    match service.active_leases().await {
        Ok(leases) => Ok(Json(leases)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "status": "error",
                "message": e.to_string(),
            })),
        )),
    }
}

/// POST /change/conflicts/check
pub async fn check_conflicts_handler(
    State(service): State<Arc<dyn ChangeControlPort>>,
    Json(payload): Json<ConflictCheckRequest>,
) -> Result<Json<Vec<ConflictReport>>, (StatusCode, Json<serde_json::Value>)> {
    match service.check_conflicts(&payload.task_id).await {
        Ok(reports) => Ok(Json(reports)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "status": "error",
                "message": e.to_string(),
            })),
        )),
    }
}

/// POST /change/validate
pub async fn validate_handler(
    State(service): State<Arc<dyn ChangeControlPort>>,
    Json(payload): Json<ValidateRequest>,
) -> Result<Json<ValidationResult>, (StatusCode, Json<serde_json::Value>)> {
    match service.validate_change(&payload.scope).await {
        Ok(result) => Ok(Json(result)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "status": "error",
                "message": e.to_string(),
            })),
        )),
    }
}

/// POST /change/complete
pub async fn complete_task_handler(
    State(service): State<Arc<dyn ChangeControlPort>>,
    Json(payload): Json<CompleteTaskRequest>,
) -> Result<Json<TaskCompletionResult>, (StatusCode, Json<serde_json::Value>)> {
    match service
        .complete_task(&payload.task_id, payload.result)
        .await
    {
        Ok(result) => Ok(Json(result)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "status": "error",
                "message": e.to_string(),
            })),
        )),
    }
}

/// GET /change/merge-plan
pub async fn merge_plan_handler(
    State(service): State<Arc<dyn ChangeControlPort>>,
) -> Result<Json<MergePlan>, (StatusCode, Json<serde_json::Value>)> {
    match service.merge_plan().await {
        Ok(plan) => Ok(Json(plan)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "status": "error",
                "message": e.to_string(),
            })),
        )),
    }
}
