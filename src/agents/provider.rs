use anyhow::{anyhow, bail, Context, Result};
use reqwest::Client;
use serde::Serialize;
use std::time::Duration;

use crate::agents::system1::RetrievedDocument;

const DEFAULT_LOCAL_BASE_URL: &str = "http://localhost:11434/v1";
const DEFAULT_LOCAL_ANTHROPIC_BASE_URL: &str = "http://localhost:11434";
const DEFAULT_LOCAL_MODEL: &str = "qwen3-coder";
const DEFAULT_OPENAI_BASE_URL: &str = "https://api.openai.com/v1";
const DEFAULT_ANTHROPIC_BASE_URL: &str = "https://api.anthropic.com/v1";
const DEFAULT_DEEPSEEK_BASE_URL: &str = "https://api.deepseek.com";
const DEFAULT_MINIMAX_BASE_URL: &str = "https://api.minimax.chat/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderMode {
    Local,
    Cloud,
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiFlavor {
    OpenAICompatible,
    AnthropicCompatible,
}

impl ApiFlavor {
    fn from_env(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "openai" | "openai-compatible" => Some(Self::OpenAICompatible),
            "anthropic" | "anthropic-compatible" => Some(Self::AnthropicCompatible),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::OpenAICompatible => "openai-compatible",
            Self::AnthropicCompatible => "anthropic-compatible",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProviderTarget {
    GenericOpenAICompatible,
    AnthropicMessages,
    GeminiLegacy,
    MiniMaxLegacy,
}

#[derive(Debug, Clone)]
pub struct ModelProviderConfig {
    pub provider_mode: ProviderMode,
    pub api_flavor: ApiFlavor,
    pub provider_label: String,
    pub model: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    target: ProviderTarget,
}

impl ModelProviderConfig {
    pub fn from_env() -> Self {
        match std::env::var("XAVIER2_MODEL_PROVIDER")
            .ok()
            .map(|value| value.trim().to_ascii_lowercase())
            .as_deref()
        {
            Some("local") => Self::local_from_env(),
            Some("cloud") => Self::cloud_from_env(),
            Some("disabled") => Self::disabled(),
            Some("anthropic") => Self::anthropic_cloud_from_env(),
            Some("openai") => Self::openai_cloud_from_env(),
            Some("deepseek") => Self::deepseek_cloud_from_env(),
            Some("minimax") => Self::minimax_cloud_from_env(),
            Some("gemini") => Self::gemini_cloud_from_env(),
            _ => Self::local_from_env(),
        }
    }

    fn local_from_env() -> Self {
        let api_flavor = std::env::var("XAVIER2_API_FLAVOR")
            .ok()
            .and_then(|value| ApiFlavor::from_env(&value))
            .unwrap_or(ApiFlavor::OpenAICompatible);

        match api_flavor {
            ApiFlavor::OpenAICompatible => Self {
                provider_mode: ProviderMode::Local,
                api_flavor,
                provider_label: "local".to_string(),
                model: std::env::var("XAVIER2_LOCAL_LLM_MODEL")
                    .or_else(|_| std::env::var("XAVIER2_LLM_MODEL"))
                    .unwrap_or_else(|_| DEFAULT_LOCAL_MODEL.to_string()),
                api_key: std::env::var("XAVIER2_LOCAL_LLM_API_KEY")
                    .ok()
                    .or_else(|| Some("ollama".to_string())),
                base_url: Some(
                    std::env::var("XAVIER2_LOCAL_LLM_URL")
                        .unwrap_or_else(|_| DEFAULT_LOCAL_BASE_URL.to_string()),
                ),
                target: ProviderTarget::GenericOpenAICompatible,
            },
            ApiFlavor::AnthropicCompatible => Self {
                provider_mode: ProviderMode::Local,
                api_flavor,
                provider_label: "local".to_string(),
                model: std::env::var("XAVIER2_LOCAL_LLM_MODEL")
                    .or_else(|_| std::env::var("XAVIER2_LLM_MODEL"))
                    .unwrap_or_else(|_| DEFAULT_LOCAL_MODEL.to_string()),
                api_key: std::env::var("XAVIER2_LOCAL_LLM_API_KEY")
                    .ok()
                    .or_else(|| Some("ollama".to_string())),
                base_url: Some(
                    std::env::var("XAVIER2_LOCAL_ANTHROPIC_URL")
                        .or_else(|_| std::env::var("XAVIER2_LOCAL_LLM_URL"))
                        .unwrap_or_else(|_| DEFAULT_LOCAL_ANTHROPIC_BASE_URL.to_string()),
                ),
                target: ProviderTarget::AnthropicMessages,
            },
        }
    }

    fn cloud_from_env() -> Self {
        let api_flavor = std::env::var("XAVIER2_API_FLAVOR")
            .ok()
            .and_then(|value| ApiFlavor::from_env(&value))
            .unwrap_or(ApiFlavor::OpenAICompatible);

        match api_flavor {
            ApiFlavor::OpenAICompatible => Self {
                provider_mode: ProviderMode::Cloud,
                api_flavor,
                provider_label: "cloud".to_string(),
                model: std::env::var("XAVIER2_CLOUD_LLM_MODEL")
                    .or_else(|_| std::env::var("XAVIER2_LLM_MODEL"))
                    .unwrap_or_else(|_| "gpt-4o-mini".to_string()),
                api_key: std::env::var("XAVIER2_LLM_API_KEY")
                    .ok()
                    .or_else(|| std::env::var("OPENAI_API_KEY").ok()),
                base_url: Some(
                    std::env::var("XAVIER2_CLOUD_LLM_URL")
                        .or_else(|_| std::env::var("OPENAI_BASE_URL"))
                        .unwrap_or_else(|_| DEFAULT_OPENAI_BASE_URL.to_string()),
                ),
                target: ProviderTarget::GenericOpenAICompatible,
            },
            ApiFlavor::AnthropicCompatible => Self::anthropic_cloud_from_env(),
        }
    }

    fn openai_cloud_from_env() -> Self {
        Self {
            provider_mode: ProviderMode::Cloud,
            api_flavor: ApiFlavor::OpenAICompatible,
            provider_label: "openai".to_string(),
            model: std::env::var("XAVIER2_LLM_MODEL")
                .or_else(|_| std::env::var("OPENAI_MODEL"))
                .unwrap_or_else(|_| "gpt-4o-mini".to_string()),
            api_key: std::env::var("OPENAI_API_KEY").ok(),
            base_url: Some(
                std::env::var("OPENAI_BASE_URL")
                    .unwrap_or_else(|_| DEFAULT_OPENAI_BASE_URL.to_string()),
            ),
            target: ProviderTarget::GenericOpenAICompatible,
        }
    }

    fn deepseek_cloud_from_env() -> Self {
        Self {
            provider_mode: ProviderMode::Cloud,
            api_flavor: ApiFlavor::OpenAICompatible,
            provider_label: "deepseek".to_string(),
            model: std::env::var("XAVIER2_LLM_MODEL")
                .or_else(|_| std::env::var("DEEPSEEK_MODEL"))
                .unwrap_or_else(|_| "deepseek-chat".to_string()),
            api_key: std::env::var("DEEPSEEK_API_KEY").ok(),
            base_url: Some(
                std::env::var("DEEPSEEK_BASE_URL")
                    .unwrap_or_else(|_| DEFAULT_DEEPSEEK_BASE_URL.to_string()),
            ),
            target: ProviderTarget::GenericOpenAICompatible,
        }
    }

    fn anthropic_cloud_from_env() -> Self {
        Self {
            provider_mode: ProviderMode::Cloud,
            api_flavor: ApiFlavor::AnthropicCompatible,
            provider_label: "anthropic".to_string(),
            model: std::env::var("XAVIER2_LLM_MODEL")
                .or_else(|_| std::env::var("ANTHROPIC_MODEL"))
                .unwrap_or_else(|_| "claude-3-5-sonnet-latest".to_string()),
            api_key: std::env::var("ANTHROPIC_API_KEY")
                .ok()
                .or_else(|| std::env::var("XAVIER2_LLM_API_KEY").ok()),
            base_url: Some(
                std::env::var("ANTHROPIC_BASE_URL")
                    .or_else(|_| std::env::var("XAVIER2_CLOUD_LLM_URL"))
                    .unwrap_or_else(|_| DEFAULT_ANTHROPIC_BASE_URL.to_string()),
            ),
            target: ProviderTarget::AnthropicMessages,
        }
    }

    fn minimax_cloud_from_env() -> Self {
        Self {
            provider_mode: ProviderMode::Cloud,
            api_flavor: ApiFlavor::OpenAICompatible,
            provider_label: "minimax".to_string(),
            model: std::env::var("XAVIER2_LLM_MODEL")
                .or_else(|_| std::env::var("MINIMAX_MODEL"))
                .unwrap_or_else(|_| "MiniMax-Text-01".to_string()),
            api_key: std::env::var("MINIMAX_API_KEY").ok(),
            base_url: Some(
                std::env::var("MINIMAX_BASE_URL")
                    .unwrap_or_else(|_| DEFAULT_MINIMAX_BASE_URL.to_string()),
            ),
            target: ProviderTarget::MiniMaxLegacy,
        }
    }

    fn gemini_cloud_from_env() -> Self {
        Self {
            provider_mode: ProviderMode::Cloud,
            api_flavor: ApiFlavor::OpenAICompatible,
            provider_label: "gemini".to_string(),
            model: std::env::var("XAVIER2_LLM_MODEL")
                .or_else(|_| std::env::var("GEMINI_MODEL"))
                .unwrap_or_else(|_| "gemini-2.0-flash".to_string()),
            api_key: std::env::var("GEMINI_API_KEY").ok(),
            base_url: None,
            target: ProviderTarget::GeminiLegacy,
        }
    }

    fn disabled() -> Self {
        Self {
            provider_mode: ProviderMode::Disabled,
            api_flavor: ApiFlavor::OpenAICompatible,
            provider_label: "disabled".to_string(),
            model: "disabled".to_string(),
            api_key: None,
            base_url: None,
            target: ProviderTarget::GenericOpenAICompatible,
        }
    }

    pub fn is_configured(&self) -> bool {
        match self.provider_mode {
            ProviderMode::Disabled => false,
            ProviderMode::Local => self
                .base_url
                .as_ref()
                .is_some_and(|value| !value.trim().is_empty()),
            ProviderMode::Cloud => {
                self.base_url
                    .as_ref()
                    .is_some_and(|value| !value.trim().is_empty())
                    && self
                        .api_key
                        .as_ref()
                        .is_some_and(|value| !value.trim().is_empty())
            }
        }
    }

    pub fn get_all_configured() -> Vec<Self> {
        let mut configured = Vec::new();
        for config in [
            Self::local_from_env(),
            Self::openai_cloud_from_env(),
            Self::anthropic_cloud_from_env(),
            Self::deepseek_cloud_from_env(),
            Self::minimax_cloud_from_env(),
            Self::gemini_cloud_from_env(),
        ] {
            if config.is_configured() {
                configured.push(config);
            }
        }
        configured
    }

    pub fn with_model_override(mut self, model_override: Option<String>) -> Self {
        if let Some(model) = model_override
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
        {
            self.model = model;
        }
        self
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelProviderStatus {
    pub provider: String,
    pub model: String,
    pub configured: bool,
}

#[derive(Clone)]
pub struct ModelProviderClient {
    client: Client,
    config: ModelProviderConfig,
}

impl ModelProviderClient {
    pub fn from_env() -> Self {
        Self::from_model_override(None)
    }

    pub fn from_model_override(model_override: Option<String>) -> Self {
        Self {
            client: Client::builder()
                .connect_timeout(Duration::from_secs(2))
                .timeout(Duration::from_secs(8))
                .build()
                .expect("model provider HTTP client"),
            config: ModelProviderConfig::from_env().with_model_override(model_override),
        }
    }

    pub fn status(&self) -> ModelProviderStatus {
        ModelProviderStatus {
            provider: if self.config.provider_mode == ProviderMode::Disabled {
                "disabled".to_string()
            } else {
                format!(
                    "{}:{}",
                    self.config.provider_label,
                    self.config.api_flavor.as_str()
                )
            },
            model: self.config.model.clone(),
            configured: self.config.is_configured(),
        }
    }

    pub async fn generate_response(
        &self,
        query: &str,
        context: &[RetrievedDocument],
    ) -> Result<String> {
        let system_prompt = "You are a helpful AI assistant part of the Xavier2 memory system. Use the provided memory context accurately. If the context is insufficient, say so clearly. Be concise but informative.";
        let context_text = context
            .iter()
            .map(|doc| format!("- {}\n  Source: {}", doc.content, doc.path))
            .collect::<Vec<_>>()
            .join("\n\n");
        let user_prompt = format!(
            "Context from memory:\n{}\n\nUser question: {}",
            context_text, query
        );

        self.generate_text(system_prompt, &user_prompt).await
    }

    pub async fn generate_hypothetical_document(&self, query: &str) -> Result<String> {
        let system_prompt = "You are an expert knowledge system. Generate a hypothetical, highly plausible document snippet or answer that directly addresses the user's query. Do not include introductory or concluding remarks. Write only the factual content as if it were a real, authoritative reference document.";
        self.generate_text(system_prompt, query).await
    }

    pub async fn evaluate_context(
        &self,
        query: &str,
        context: &[RetrievedDocument],
    ) -> Result<f32> {
        if !self.config.is_configured() || self.config.provider_mode == ProviderMode::Disabled {
            return Ok(1.0);
        }

        let system_prompt = "You are a critical evaluator for a RAG system. Read the context and the user query. Evaluate if the context contains sufficient and accurate information to fully answer the query. Return ONLY a valid JSON object in this exact format: {\"confidence\": 0.95} where confidence is a float between 0.0 (useless) and 1.0 (perfect).";

        let context_text = context
            .iter()
            .map(|doc| format!("- {}", doc.content))
            .collect::<Vec<_>>()
            .join("\n\n");

        let user_prompt = format!("Context:\n{}\n\nQuery: {}", context_text, query);
        let response = self.generate_text(system_prompt, &user_prompt).await?;

        let normalized = response.replace("```json", "").replace("```", "");
        let result: serde_json::Value = serde_json::from_str(normalized.trim())
            .unwrap_or_else(|_| serde_json::json!({"confidence": 1.0}));

        Ok(result["confidence"].as_f64().unwrap_or(1.0) as f32)
    }

    pub async fn generate_text(&self, system_prompt: &str, user_prompt: &str) -> Result<String> {
        if !self.config.is_configured() || self.config.provider_mode == ProviderMode::Disabled {
            bail!("no LLM provider configured");
        }

        match self.config.target {
            ProviderTarget::GenericOpenAICompatible => {
                self.generate_openai_compatible(system_prompt, user_prompt)
                    .await
            }
            ProviderTarget::AnthropicMessages => {
                self.generate_anthropic_compatible(system_prompt, user_prompt)
                    .await
            }
            ProviderTarget::GeminiLegacy => {
                self.generate_gemini_legacy(system_prompt, user_prompt)
                    .await
            }
            ProviderTarget::MiniMaxLegacy => {
                self.generate_minimax_legacy(system_prompt, user_prompt)
                    .await
            }
        }
    }

    async fn generate_openai_compatible(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String> {
        let base_url = self
            .config
            .base_url
            .as_ref()
            .context("missing OpenAI-compatible base URL")?;
        let endpoint = openai_chat_endpoint(base_url);
        let mut request = self
            .client
            .post(endpoint)
            .header("Content-Type", "application/json");

        if let Some(api_key) = self
            .config
            .api_key
            .as_ref()
            .filter(|value| !value.trim().is_empty())
        {
            request = request.bearer_auth(api_key);
        }

        let response = request
            .json(&serde_json::json!({
                "model": self.config.model,
                "messages": [
                    {"role": "system", "content": system_prompt},
                    {"role": "user", "content": user_prompt}
                ],
                "temperature": 0.2,
                "max_tokens": 500
            }))
            .send()
            .await
            .context("failed to call OpenAI-compatible API")?
            .error_for_status()
            .context("OpenAI-compatible API returned an error")?;

        let payload: serde_json::Value = response
            .json()
            .await
            .context("failed to decode OpenAI-compatible response")?;
        payload["choices"]
            .as_array()
            .and_then(|choices| choices.first())
            .and_then(|choice| choice["message"]["content"].as_str())
            .map(|text| text.to_string())
            .ok_or_else(|| anyhow!("OpenAI-compatible response did not contain text"))
    }

    async fn generate_anthropic_compatible(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String> {
        let base_url = self
            .config
            .base_url
            .as_ref()
            .context("missing Anthropic-compatible base URL")?;
        let api_key = self
            .config
            .api_key
            .as_ref()
            .context("missing Anthropic-compatible API key")?;
        let endpoint = anthropic_messages_endpoint(base_url);

        let response = self
            .client
            .post(endpoint)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&serde_json::json!({
                "model": self.config.model,
                "system": system_prompt,
                "max_tokens": 500,
                "temperature": 0.2,
                "messages": [
                    {"role": "user", "content": user_prompt}
                ]
            }))
            .send()
            .await
            .context("failed to call Anthropic-compatible API")?
            .error_for_status()
            .context("Anthropic-compatible API returned an error")?;

        let payload: serde_json::Value = response
            .json()
            .await
            .context("failed to decode Anthropic-compatible response")?;
        payload["content"]
            .as_array()
            .and_then(|items| items.iter().find(|item| item["type"] == "text"))
            .and_then(|item| item["text"].as_str())
            .map(|text| text.to_string())
            .ok_or_else(|| anyhow!("Anthropic-compatible response did not contain text"))
    }

    async fn generate_gemini_legacy(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String> {
        let api_key = self
            .config
            .api_key
            .as_ref()
            .context("missing Gemini API key")?;
        let endpoint = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.config.model, api_key
        );
        let response = self
            .client
            .post(endpoint)
            .json(&serde_json::json!({
                "system_instruction": {
                    "parts": [{"text": system_prompt}]
                },
                "contents": [{
                    "role": "user",
                    "parts": [{"text": user_prompt}]
                }],
                "generationConfig": {
                    "temperature": 0.2,
                    "maxOutputTokens": 500
                }
            }))
            .send()
            .await
            .context("failed to call Gemini API")?
            .error_for_status()
            .context("Gemini API returned an error")?;
        let payload: serde_json::Value = response
            .json()
            .await
            .context("failed to decode Gemini response")?;
        payload["candidates"]
            .as_array()
            .and_then(|candidates| candidates.first())
            .and_then(|candidate| candidate["content"]["parts"].as_array())
            .and_then(|parts| parts.first())
            .and_then(|part| part["text"].as_str())
            .map(|text| text.to_string())
            .ok_or_else(|| anyhow!("Gemini response did not contain text"))
    }

    async fn generate_minimax_legacy(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String> {
        let api_key = self
            .config
            .api_key
            .as_ref()
            .context("missing MiniMax API key")?;
        let base_url = self
            .config
            .base_url
            .as_ref()
            .context("missing MiniMax base URL")?
            .trim_end_matches('/');
        let endpoint = format!("{}/text/chatcompletion_pro", base_url);
        let response = self
            .client
            .post(endpoint)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "messages": [
                    {"role": "system", "content": system_prompt},
                    {"role": "user", "content": user_prompt}
                ],
                "model": self.config.model,
                "temperature": 0.2,
                "max_tokens": 500
            }))
            .send()
            .await
            .context("failed to call MiniMax API")?
            .error_for_status()
            .context("MiniMax API returned an error")?;
        let payload: serde_json::Value = response
            .json()
            .await
            .context("failed to decode MiniMax response")?;
        payload["choices"]
            .as_array()
            .and_then(|choices| choices.first())
            .and_then(|choice| choice["message"]["content"].as_str())
            .map(|text| text.to_string())
            .ok_or_else(|| anyhow!("MiniMax response did not contain text"))
    }
}

fn openai_chat_endpoint(base_url: &str) -> String {
    let trimmed = base_url.trim_end_matches('/');
    if trimmed.ends_with("/chat/completions") {
        trimmed.to_string()
    } else if trimmed.ends_with("/v1") {
        format!("{trimmed}/chat/completions")
    } else {
        format!("{trimmed}/v1/chat/completions")
    }
}

fn anthropic_messages_endpoint(base_url: &str) -> String {
    let trimmed = base_url.trim_end_matches('/');
    if trimmed.ends_with("/messages") {
        trimmed.to_string()
    } else if trimmed.ends_with("/v1") {
        format!("{trimmed}/messages")
    } else {
        format!("{trimmed}/v1/messages")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn test_local_provider_config() {
        let _guard = env_lock().lock().unwrap();
        std::env::set_var("XAVIER2_LOCAL_LLM_MODEL", "test-model");
        std::env::remove_var("XAVIER2_LLM_MODEL");
        std::env::set_var("XAVIER2_LOCAL_LLM_URL", "http://test-url/v1");
        std::env::remove_var("XAVIER2_API_FLAVOR");

        let config = ModelProviderConfig::local_from_env();

        assert_eq!(config.model, "test-model");
        assert_eq!(config.base_url, Some("http://test-url/v1".to_string()));
        assert_eq!(config.provider_mode, ProviderMode::Local);
        assert_eq!(config.api_flavor, ApiFlavor::OpenAICompatible);
    }

    #[test]
    fn test_local_provider_defaults() {
        let _guard = env_lock().lock().unwrap();
        std::env::remove_var("XAVIER2_LOCAL_LLM_MODEL");
        std::env::remove_var("XAVIER2_LLM_MODEL");
        std::env::remove_var("XAVIER2_LOCAL_LLM_URL");
        std::env::remove_var("XAVIER2_API_FLAVOR");

        let config = ModelProviderConfig::local_from_env();

        assert_eq!(config.model, DEFAULT_LOCAL_MODEL);
        assert_eq!(config.base_url, Some(DEFAULT_LOCAL_BASE_URL.to_string()));
        assert_eq!(config.api_key.as_deref(), Some("ollama"));
    }

    #[test]
    fn test_local_anthropic_flavor_uses_ollama_base() {
        let _guard = env_lock().lock().unwrap();
        std::env::set_var("XAVIER2_API_FLAVOR", "anthropic-compatible");
        std::env::remove_var("XAVIER2_LOCAL_ANTHROPIC_URL");
        std::env::remove_var("XAVIER2_LOCAL_LLM_URL");

        let config = ModelProviderConfig::local_from_env();

        assert_eq!(config.api_flavor, ApiFlavor::AnthropicCompatible);
        assert_eq!(
            config.base_url,
            Some(DEFAULT_LOCAL_ANTHROPIC_BASE_URL.to_string())
        );

        std::env::remove_var("XAVIER2_API_FLAVOR");
    }
}
