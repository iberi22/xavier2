use axum::{routing::get, Router};

pub fn create_router() -> Router {
    Router::new().route("/health", get(health_handler))
}

async fn health_handler() -> &'static str {
    "ok"
}
