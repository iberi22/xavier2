//! CLI utility functions

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

pub fn estimate_tokens(text: &str) -> usize {
    (text.len() / 4).max(1)
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
