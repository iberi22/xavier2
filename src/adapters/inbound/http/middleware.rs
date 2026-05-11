use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
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

/// Axum middleware that requires a valid X-Xavier-Token on all protected routes.
pub async fn auth_middleware(req: Request<Body>, next: Next) -> Response {
    let expected_token = match std::env::var("XAVIER_TOKEN") {
        Ok(token) => token,
        Err(_) => {
            return json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({"status":"error","message":"XAVIER_TOKEN is not configured"}),
            );
        }
    };

    let provided_token = req
        .headers()
        .get("X-Xavier-Token")
        .and_then(|value| value.to_str().ok());

    if provided_token != Some(expected_token.as_str()) {
        return json_response(
            StatusCode::UNAUTHORIZED,
            serde_json::json!({"status":"error","message":"Unauthorized"}),
        );
    }

    next.run(req).await
}
