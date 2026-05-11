use crate::adapters::inbound::http::AppState;
use axum::{extract::State, Json};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SecurityScanPayload {
    pub input: String,
}

pub async fn security_scan_handler(
    State(state): State<AppState>,
    Json(payload): Json<SecurityScanPayload>,
) -> Json<serde_json::Value> {
    let result = match state.security.process_input(&payload.input).await {
        Ok(res) => res,
        Err(e) => {
            return Json(serde_json::json!({
                "status": "error",
                "message": format!("Security scan error: {}", e),
            }));
        }
    };

    Json(serde_json::json!({
        "status": if result.allowed { "allowed" } else { "blocked" },
        "allowed": result.allowed,
        "detection": {
            "is_injection": result.is_injection,
            "confidence": result.detection_confidence,
            "attack_type": result.attack_type,
        },
        "sanitized_input": result.sanitized_input,
        "original_input": result.original_input,
    }))
}
