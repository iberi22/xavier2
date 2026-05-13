use axum::{
    extract::{State, Json},
    response::{IntoResponse, Response},
    http::StatusCode,
};
use serde::Deserialize;
use crate::cli::state::CliState;
use xavier::agents::provider::{ModelProviderClient, ModelProviderConfig};
use xavier::agents::router::{load_routing_policy, RouteCategory, Router};
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

    // 2. Resolve Model and apply cost ceilings
    let mut requested_model = req.model.clone();
    let router = Router::new();

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

    let policy = load_routing_policy();
    let decision = router.classify(user_msg);

    if decision.category == RouteCategory::Direct || decision.category == RouteCategory::Retrieved {
        if let Some(ref p) = policy {
            let quality_model = p.models.quality.as_ref().map(|m| m.name.clone());
            let fast_model = p.models.fast.as_ref().map(|m| m.name.clone());

            if let (Some(quality), Some(fast)) = (quality_model, fast_model) {
                if requested_model == quality {
                    info!("Routing category {:?} detected. Enforcing cost ceiling: overriding {} with fast model {}", decision.category, quality, fast);
                    requested_model = fast;
                }
            }
        }
    }

    // 3. Execute Request
    let config = ModelProviderConfig::for_provider(&provider_name).with_model_override(Some(requested_model.clone()));
    let client = ModelProviderClient::new(config);

    match client.generate_text(system_msg, user_msg).await {
        Ok(text) => {
            // 4. Track Usage and Cost
            let prompt_tokens = user_msg.len() / 4;
            let completion_tokens = text.len() / 4;
            let total_tokens = prompt_tokens + completion_tokens;

            let mut cost_usd = 0.0;
            if let Some(ref p) = policy {
                let matched_policy = if p.models.fast.as_ref().map_or(false, |m| m.name == requested_model) {
                    p.models.fast.as_ref()
                } else if p.models.quality.as_ref().map_or(false, |m| m.name == requested_model) {
                    p.models.quality.as_ref()
                } else {
                    None
                };

                if let Some(mp) = matched_policy {
                    let input_rate = mp.cost_per_input_token.unwrap_or(0.0) as f64;
                    let output_rate = mp.cost_per_output_token.unwrap_or(0.0) as f64;
                    cost_usd = (prompt_tokens as f64 * input_rate) + (completion_tokens as f64 * output_rate);
                }
            }

            if let Err(e) = state.rate_manager.track_request(&provider_name, total_tokens, 200, cost_usd).await {
                warn!("Failed to track request usage: {}", e);
            }

            (StatusCode::OK, Json(serde_json::json!({
                "id": format!("chatcmpl-{}", ulid::Ulid::new()),
                "object": "chat.completion",
                "created": chrono::Utc::now().timestamp(),
                "model": requested_model,
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": text
                    },
                    "finish_reason": "stop"
                }],
                "usage": {
                    "prompt_tokens": prompt_tokens,
                    "completion_tokens": completion_tokens,
                    "total_tokens": total_tokens
                }
            }))).into_response()
        }
        Err(e) => {
            warn!("Provider {} failed: {}", provider_name, e);
            if let Err(track_err) = state.rate_manager.track_request(&provider_name, 0, 500, 0.0).await {
                warn!("Failed to track failed request: {}", track_err);
            }
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Provider error: {}", e)).into_response()
        }
    }
}
