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
