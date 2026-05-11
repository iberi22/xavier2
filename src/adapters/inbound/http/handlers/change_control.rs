use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use crate::adapters::inbound::http::state::AppState;
use crate::domain::change_control::{
    ChangeLease, ChangeTask, ConflictReport, LeaseClaimResponse, LeaseMode, MergePlan,
    ValidationResult,
};

#[derive(Debug, Deserialize)]
pub struct CreateTaskRequest {
    pub agent_id: String,
    pub title: String,
    pub intent: String,
    pub scope: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct CompleteTaskRequest {
    pub task_id: String,
    pub result: String,
}

#[derive(Debug, Deserialize)]
pub struct ClaimLeaseRequest {
    pub agent_id: String,
    pub task_id: String,
    pub patterns: Vec<String>,
    pub mode: LeaseMode,
    pub ttl_seconds: u64,
}

#[derive(Debug, Deserialize)]
pub struct ReleaseLeaseRequest {
    pub lease_id: String,
}

#[derive(Debug, Deserialize)]
pub struct CheckConflictsRequest {
    pub task_id: String,
    pub scope: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ValidateRequest {
    pub task_id: String,
}

pub async fn create_task_handler(
    State(state): State<AppState>,
    Json(payload): Json<CreateTaskRequest>,
) -> Result<Json<ChangeTask>, (StatusCode, Json<serde_json::Value>)> {
    match state
        .change_control
        .create_task(payload.agent_id, payload.title, payload.intent, payload.scope)
        .await
    {
        Ok(task) => Ok(Json(task)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )),
    }
}

pub async fn check_conflicts_handler(
    State(state): State<AppState>,
    Json(payload): Json<CheckConflictsRequest>,
) -> Result<Json<ConflictReport>, (StatusCode, Json<serde_json::Value>)> {
    match state
        .change_control
        .check_conflicts(payload.task_id, payload.scope)
        .await
    {
        Ok(report) => Ok(Json(report)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )),
    }
}

pub async fn validate_handler(
    State(state): State<AppState>,
    Json(payload): Json<ValidateRequest>,
) -> Result<Json<ValidationResult>, (StatusCode, Json<serde_json::Value>)> {
    match state.change_control.validate_task(payload.task_id).await {
        Ok(result) => Ok(Json(result)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )),
    }
}

pub async fn merge_plan_handler(
    State(state): State<AppState>,
) -> Result<Json<MergePlan>, (StatusCode, Json<serde_json::Value>)> {
    match state.change_control.get_merge_plan().await {
        Ok(plan) => Ok(Json(plan)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )),
    }
}

pub async fn claim_lease_handler(
    State(state): State<AppState>,
    Json(payload): Json<ClaimLeaseRequest>,
) -> Result<Json<LeaseClaimResponse>, (StatusCode, Json<serde_json::Value>)> {
    match state
        .change_control
        .claim_lease(
            payload.agent_id,
            payload.task_id,
            payload.patterns,
            payload.mode,
            payload.ttl_seconds,
        )
        .await
    {
        Ok(response) => Ok(Json(response)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )),
    }
}

pub async fn release_lease_handler(
    State(state): State<AppState>,
    Json(payload): Json<ReleaseLeaseRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match state.change_control.release_lease(payload.lease_id).await {
        Ok(success) => Ok(Json(serde_json::json!({ "success": success }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )),
    }
}

pub async fn active_leases_handler(
    State(state): State<AppState>,
) -> Result<Json<Vec<ChangeLease>>, (StatusCode, Json<serde_json::Value>)> {
    match state.change_control.get_active_leases().await {
        Ok(leases) => Ok(Json(leases)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )),
    }
}

pub async fn get_task_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ChangeTask>, (StatusCode, Json<serde_json::Value>)> {
    match state.change_control.get_task(id).await {
        Ok(Some(task)) => Ok(Json(task)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Task not found" })),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )),
    }
}

pub async fn complete_task_handler(
    State(state): State<AppState>,
    Json(payload): Json<CompleteTaskRequest>,
) -> Result<Json<ChangeTask>, (StatusCode, Json<serde_json::Value>)> {
    match state
        .change_control
        .complete_task(payload.task_id, payload.result)
        .await
    {
        Ok(task) => Ok(Json(task)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )),
    }
}
