use crate::cli::state::CliState;
use crate::cli::utils::ProxyErrorWrapper;
use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use tracing::warn;
use xavier::domain::proxy::ProxyChatCommand;

#[derive(Debug, Deserialize)]
pub struct ProxyChatRequest {
    pub model: String,
    pub messages: Vec<serde_json::Value>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<usize>,
}

impl From<ProxyChatRequest> for ProxyChatCommand {
    fn from(req: ProxyChatRequest) -> Self {
        Self {
            model: req.model,
            messages: req.messages,
            temperature: req.temperature,
            max_tokens: req.max_tokens,
        }
    }
}

pub async fn chat_proxy(
    State(state): State<CliState>,
    Json(req): Json<ProxyChatRequest>,
) -> Response {
    match state.proxy_use_case.execute(req.into()).await {
        Ok(resp) => Json(resp).into_response(),
        Err(e) => ProxyErrorWrapper(e).into_response(),
    }
}

pub async fn chat_batch_proxy(
    State(state): State<CliState>,
    Json(requests): Json<Vec<ProxyChatRequest>>,
) -> Response {
    let mut results = vec![serde_json::json!(null); requests.len()];
    let mut join_set = tokio::task::JoinSet::new();

    for (idx, req) in requests.into_iter().enumerate() {
        let use_case = state.proxy_use_case.clone();
        join_set.spawn(async move {
            let res = use_case.execute(req.into()).await;
            (idx, res)
        });
    }

    while let Some(res) = join_set.join_next().await {
        match res {
            Ok((idx, Ok(val))) => {
                results[idx] = serde_json::to_value(val).unwrap_or(serde_json::json!(null));
            }
            Ok((idx, Err(e))) => {
                results[idx] = serde_json::json!({
                    "error": e.to_string(),
                    "status": match e {
                        xavier::domain::proxy::ProxyError::RateLimited => 429,
                        _ => 500,
                    }
                });
            }
            Err(e) => {
                warn!("Batch task failed: {e}");
            }
        }
    }

    (StatusCode::OK, Json(results)).into_response()
}
