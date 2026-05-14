//! CLI utility functions

use axum::{
    body::Body,
    http::StatusCode,
    response::{IntoResponse, Response},
};

pub fn json_response(status: StatusCode, body: serde_json::Value) -> Response {
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .header("x-request-id", uuid::Uuid::new_v4().to_string())
        .body(Body::from(body.to_string()))
        .unwrap_or_else(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({"status":"error"}).to_string(),
            )
                .into_response()
        })
}

pub fn estimate_tokens(text: &str) -> usize {
    (text.len() / 4).max(1)
}

pub fn load_skill(skill_name: &str) -> Option<String> {
    let paths = [
        format!("skills/{}/SKILL.md", skill_name),
        format!("skills/{}.md", skill_name),
        format!(".agents/skills/{}/SKILL.md", skill_name),
        format!(".agents/skills/{}.md", skill_name),
    ];

    for path in paths {
        if let Ok(content) = std::fs::read_to_string(path) {
            return Some(content);
        }
    }
    None
}

pub struct ProxyErrorWrapper(pub xavier::domain::proxy::ProxyError);

impl IntoResponse for ProxyErrorWrapper {
    fn into_response(self) -> Response {
        let (status, message) = match self.0 {
            xavier::domain::proxy::ProxyError::RateLimited => {
                (StatusCode::TOO_MANY_REQUESTS, self.0.to_string())
            }
            xavier::domain::proxy::ProxyError::ProviderError(e) => {
                (StatusCode::INTERNAL_SERVER_ERROR, e)
            }
            xavier::domain::proxy::ProxyError::Internal(e) => {
                (StatusCode::INTERNAL_SERVER_ERROR, e)
            }
        };
        (status, message).into_response()
    }
}
