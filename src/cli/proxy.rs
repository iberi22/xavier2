use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use std::collections::HashMap;
use tracing::{info, warn};

use crate::cli::state::CliState;
use xavier::agents::provider::{ModelProviderClient, ModelProviderConfig};

#[derive(Debug, Deserialize)]
pub struct ProxyChatRequest {
    pub model: String,
    pub messages: Vec<serde_json::Value>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<usize>,
}

pub async fn chat_proxy(
    State(state): State<CliState>,
    Json(req): Json<ProxyChatRequest>,
) -> Response {
    // 1. Resolve Provider based on Rate Limits
    let provider_name = match select_provider(&state).await {
        Some(p) => p,
        None => {
            return (StatusCode::TOO_MANY_REQUESTS, "All providers are rate-limited").into_response()
        }
    };

    let result = perform_proxy_request(&state, provider_name, req).await;

    if let Some(error) = result.get("error") {
        let status = result
            .get("status")
            .and_then(|s| s.as_u64())
            .unwrap_or(500) as u16;
        return (
            StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            error.as_str().unwrap_or("Unknown error").to_string(),
        )
            .into_response();
    }

    (StatusCode::OK, Json(result)).into_response()
}

pub async fn chat_batch_proxy(
    State(state): State<CliState>,
    Json(requests): Json<Vec<ProxyChatRequest>>,
) -> Response {
    let mut results = vec![serde_json::json!(null); requests.len()];
    let mut provider_assignments: HashMap<String, Vec<(usize, ProxyChatRequest)>> = HashMap::new();

    // 1. Resolve Providers for all requests
    for (idx, req) in requests.into_iter().enumerate() {
        let provider = select_provider(&state)
            .await
            .unwrap_or_else(|| "none".to_string());
        provider_assignments
            .entry(provider)
            .or_insert_with(Vec::new)
            .push((idx, req));
    }

    let mut join_set = tokio::task::JoinSet::new();

    // 2. Execute requests in parallel per provider
    for (provider, reqs) in provider_assignments {
        if provider == "none" {
            for (idx, _) in reqs {
                results[idx] = serde_json::json!({
                    "error": "All providers are rate-limited",
                    "status": 429
                });
            }
            continue;
        }

        for (idx, req) in reqs {
            let state_clone = state.clone();
            let provider_clone = provider.clone();
            join_set.spawn(async move {
                let res = perform_proxy_request(&state_clone, provider_clone, req).await;
                (idx, res)
            });
        }
    }

    // 3. Collect results in order
    while let Some(res) = join_set.join_next().await {
        match res {
            Ok((idx, val)) => {
                results[idx] = val;
            }
            Err(e) => {
                warn!("Batch task failed: {}", e);
            }
        }
    }

    (StatusCode::OK, Json(results)).into_response()
}

async fn select_provider(state: &CliState) -> Option<String> {
    // Order of priority as per AGENTS.md
    let providers = ["opencode-go", "deepseek", "groq", "openai", "anthropic"];
    for provider in providers {
        match state.rate_manager.get_status(provider).await {
            Ok(status) => {
                let now = chrono::Utc::now();
                if status.rate_limited_until.map_or(true, |until| until < now) {
                    return Some(provider.to_string());
                }
            }
            Err(e) => {
                warn!("Failed to check rate limit for {}: {}", provider, e);
            }
        }
    }
    None
}

async fn perform_proxy_request(
    state: &CliState,
    provider_name: String,
    req: ProxyChatRequest,
) -> serde_json::Value {
    info!("Proxying request to provider: {}", provider_name);

    // 2. Execute Request
    let config = ModelProviderConfig::for_provider(&provider_name).with_model_override(Some(req.model.clone()));
    let client = ModelProviderClient::new(config);

    // Extraction logic for Axum Proxy
    let system_msg = req
        .messages
        .iter()
        .find(|m| m["role"] == "system")
        .and_then(|m| m["content"].as_str())
        .unwrap_or("You are a helpful assistant.");

    let user_msg = req
        .messages
        .iter()
        .filter(|m| m["role"] == "user")
        .last()
        .and_then(|m| m["content"].as_str())
        .unwrap_or("");

    match client.generate_text(system_msg, user_msg).await {
        Ok(text) => {
            // 3. Track Usage
            // Rough token estimation (1 token ~= 4 chars)
            let tokens = (text.len() + user_msg.len()) / 4;
            if let Err(e) = state
                .rate_manager
                .track_request(&provider_name, tokens, 200)
                .await
            {
                warn!("Failed to track request usage: {}", e);
            }

            serde_json::json!({
                "id": format!("chatcmpl-{}", ulid::Ulid::new()),
                "object": "chat.completion",
                "created": chrono::Utc::now().timestamp(),
                "model": req.model,
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": text
                    },
                    "finish_reason": "stop"
                }],
                "usage": {
                    "prompt_tokens": user_msg.len() / 4,
                    "completion_tokens": text.len() / 4,
                    "total_tokens": tokens
                }
            })
        }
        Err(e) => {
            warn!("Provider {} failed: {}", provider_name, e);
            if let Err(track_err) = state.rate_manager.track_request(&provider_name, 0, 500).await {
                warn!("Failed to track failed request: {}", track_err);
            }
            serde_json::json!({
                "error": format!("Provider error: {}", e),
                "status": 500
            })
        }
    }
}
