use std::sync::Arc;
use parking_lot::Mutex;
use std::collections::HashMap;
use tracing::{info, warn};
use sha2::{Digest, Sha256};

use crate::domain::proxy::{ChatChoice, ChatCompletion, ChatMessage, ProxyChatCommand, ProxyError, Usage};
use crate::agents::rate_limit::RateLimitManager;
use crate::agents::router::{load_routing_policy, RouteCategory, Router};
use crate::agents::provider::{ModelProviderClient, ModelProviderConfig};

pub struct ProxyUseCase {
    pub rate_manager: Arc<RateLimitManager>,
    pub prompt_cache: Arc<Mutex<HashMap<String, Vec<String>>>>,
    pub router: Router,
}

impl ProxyUseCase {
    pub fn new(
        rate_manager: Arc<RateLimitManager>,
        prompt_cache: Arc<Mutex<HashMap<String, Vec<String>>>>,
    ) -> Self {
        Self {
            rate_manager,
            prompt_cache,
            router: Router::new(),
        }
    }

    pub async fn execute(&self, cmd: ProxyChatCommand) -> Result<ChatCompletion, ProxyError> {
        // 1. Resolve Provider based on Rate Limits
        let providers = [
            "opencode-go",
            "deepseek",
            "groq",
            "openrouter",
            "google",
            "openai",
            "anthropic",
        ];
        let mut selected_provider = None;

        for provider in providers {
            match self.rate_manager.get_status(provider).await {
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
            None => return Err(ProxyError::RateLimited),
        };

        info!("Proxying request to provider: {}", provider_name);

        // 2. Resolve Model and apply cost ceilings
        let mut requested_model = cmd.model.clone();

        // Prompt Cache Detection
        let system_msg = cmd
            .messages
            .iter()
            .find(|m| m["role"] == "system")
            .and_then(|m| m["content"].as_str())
            .unwrap_or("You are a helpful assistant.");

        let mut hasher = Sha256::new();
        hasher.update(system_msg.as_bytes());
        let system_hash = hex::encode(hasher.finalize());

        let is_cache_hit = {
            let mut cache = self.prompt_cache.lock();
            let hashes = cache.entry(provider_name.clone()).or_insert_with(Vec::new);
            let hit = hashes.contains(&system_hash);
            if !hit {
                hashes.push(system_hash);
                if hashes.len() > 5 {
                    hashes.remove(0);
                }
            }
            hit
        };

        if is_cache_hit {
            info!("Prompt cache hit for provider {}", provider_name);
        }

        let user_msg = cmd
            .messages
            .iter()
            .filter(|m| m["role"] == "user")
            .last()
            .and_then(|m| m["content"].as_str())
            .unwrap_or("");

        let policy = load_routing_policy();
        let decision = self.router.classify(user_msg);

        if decision.category == RouteCategory::Direct || decision.category == RouteCategory::Retrieved {
            if let Some(ref p) = policy {
                let quality_model = p.models.quality.first().map(|m| m.name.clone());
                let fast_model = p.models.fast.first().map(|m| m.name.clone());

                if let (Some(quality), Some(fast)) = (quality_model, fast_model) {
                    if requested_model == quality {
                        info!("Routing category {:?} detected. Enforcing cost ceiling: overriding {} with fast model {}", decision.category, quality, fast);
                        requested_model = fast;
                    }
                }
            }
        }

        // 3. Execute Request
        let config = ModelProviderConfig::for_provider(&provider_name)
            .with_model_override(Some(requested_model.clone()));
        let client = ModelProviderClient::new(config);

        match client
            .generate_text_with_cache(system_msg, user_msg, is_cache_hit)
            .await
        {
            Ok(text) => {
                // 4. Track Usage and Cost
                let prompt_tokens = user_msg.len() / 4;
                let completion_tokens = text.len() / 4;
                let total_tokens = prompt_tokens + completion_tokens;

                let mut cost_usd = 0.0;
                if let Some(ref p) = policy {
                    let matched_policy = if p.models.fast.iter().any(|m| m.name == requested_model) {
                        p.models.fast.first()
                    } else if p.models.quality.iter().any(|m| m.name == requested_model) {
                        p.models.quality.first()
                    } else {
                        None
                    };

                    if let Some(mp) = matched_policy {
                        let input_rate = mp.cost_per_input_token.unwrap_or(0.0) as f64;
                        let output_rate = mp.cost_per_output_token.unwrap_or(0.0) as f64;
                        cost_usd = (prompt_tokens as f64 * input_rate)
                            + (completion_tokens as f64 * output_rate);
                    }
                }

                if let Err(e) = self
                    .rate_manager
                    .track_request(&provider_name, total_tokens, 200, cost_usd, is_cache_hit)
                    .await
                {
                    warn!("Failed to track request usage: {}", e);
                }

                Ok(ChatCompletion {
                    id: format!("chatcmpl-{}", ulid::Ulid::new()),
                    object: "chat.completion".to_string(),
                    created: chrono::Utc::now().timestamp(),
                    model: requested_model,
                    choices: vec![ChatChoice {
                        index: 0,
                        message: ChatMessage {
                            role: "assistant".to_string(),
                            content: text,
                        },
                        finish_reason: "stop".to_string(),
                    }],
                    usage: Usage {
                        prompt_tokens,
                        completion_tokens,
                        total_tokens,
                    },
                })
            }
            Err(e) => {
                warn!("Provider {} failed: {}", provider_name, e);
                if let Err(track_err) = self
                    .rate_manager
                    .track_request(&provider_name, 0, 500, 0.0, false)
                    .await
                {
                    warn!("Failed to track failed request: {}", track_err);
                }
                Err(ProxyError::ProviderError(e.to_string()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use crate::agents::rate_limit::RateLimitManager;

    #[tokio::test]
    async fn test_proxy_use_case_rate_limited() {
        let conn = Connection::open_in_memory().unwrap();
        RateLimitManager::init_schema(&conn).unwrap();
        let rate_manager = Arc::new(RateLimitManager::new(Arc::new(Mutex::new(conn))));
        let prompt_cache = Arc::new(Mutex::new(HashMap::new()));

        // Mark all providers as rate limited
        let providers = [
            "opencode-go", "deepseek", "groq", "openrouter", "google", "openai", "anthropic",
        ];
        for p in providers {
            rate_manager.report_429(p, 10).await.unwrap();
        }

        let use_case = ProxyUseCase::new(rate_manager, prompt_cache);
        let cmd = ProxyChatCommand {
            model: "test-model".into(),
            messages: vec![serde_json::json!({"role": "user", "content": "hello"})],
            temperature: None,
            max_tokens: None,
        };

        let result = use_case.execute(cmd).await;
        assert!(matches!(result, Err(ProxyError::RateLimited)));
    }
}
