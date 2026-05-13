use axum::{
    extract::{State, Json},
    response::{IntoResponse, Response},
    http::StatusCode,
};
use serde::Deserialize;
use crate::cli::state::CliState;
use xavier::agents::provider::{ModelProviderClient, ModelProviderConfig};
use tracing::{info, warn};

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
    // Order of priority as per AGENTS.md
    let providers = ["opencode-go", "deepseek", "groq", "openai", "anthropic"];
    let mut selected_provider = None;

    for provider in providers {
        match state.rate_manager.get_status(provider).await {
            Ok(status) => {
                let now = chrono::Utc::now();
                if status.rate_limited_until.map_or(true, |until| until < now) {
                    selected_provider = Some(provider.to_string());
                    break;
                }
            }
            Err(e) => {
                warn!("Failed to check rate limit for {}: {}", provider, e);
            }
        }
    }

    let provider_name = match selected_provider {
        Some(p) => p,
        None => return (StatusCode::TOO_MANY_REQUESTS, "All providers are rate-limited").into_response(),
    };

    info!("Proxying request to provider: {}", provider_name);

    // 2. Execute Request
    let config = ModelProviderConfig::for_provider(&provider_name).with_model_override(Some(req.model.clone()));
    let client = ModelProviderClient::new(config);

    // Extraction logic for Axum Proxy
    let system_msg = req.messages.iter()
        .find(|m| m["role"] == "system")
        .and_then(|m| m["content"].as_str())
        .unwrap_or("You are a helpful assistant.");
    
    let user_msg = req.messages.iter()
        .filter(|m| m["role"] == "user")
        .last()
        .and_then(|m| m["content"].as_str())
        .unwrap_or("");

    match client.generate_text(system_msg, user_msg).await {
        Ok(text) => {
            // 3. Track Usage
            // Rough token estimation (1 token ~= 4 chars)
            let tokens = (text.len() + user_msg.len()) / 4;
            if let Err(e) = state.rate_manager.track_request(&provider_name, tokens, 200).await {
                warn!("Failed to track request usage: {}", e);
            }

            (StatusCode::OK, Json(serde_json::json!({
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
            }))).into_response()
        }
        Err(e) => {
            warn!("Provider {} failed: {}", provider_name, e);
            if let Err(track_err) = state.rate_manager.track_request(&provider_name, 0, 500).await {
                warn!("Failed to track failed request: {}", track_err);
            }
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Provider error: {}", e)).into_response()
        }
    }
}
