use anyhow::{anyhow, bail, Context, Result};
use reqwest::Client;
use serde::Serialize;
use std::time::Duration;

use crate::agents::system1::RetrievedDocument;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelProviderKind {
    Gemini,
    OpenAI,
    MiniMax,
    DeepSeek,
    Anthropic,
    Local,
    Disabled,
}

impl ModelProviderKind {
    fn from_env(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "gemini" => Some(Self::Gemini),
            "openai" => Some(Self::OpenAI),
            "minimax" => Some(Self::MiniMax),
            "deepseek" => Some(Self::DeepSeek),
            "anthropic" => Some(Self::Anthropic),
            "local" => Some(Self::Local),
            "disabled" => Some(Self::Disabled),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Gemini => "gemini",
            Self::OpenAI => "openai",
            Self::MiniMax => "minimax",
            Self::DeepSeek => "deepseek",
            Self::Anthropic => "anthropic",
            Self::Local => "local",
            Self::Disabled => "disabled",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ModelProviderConfig {
    pub kind: ModelProviderKind,
    pub model: String,
    pub api_key: Option<String>,
    pub url: Option<String>,
}

impl ModelProviderConfig {
    pub fn from_env() -> Self {
        if let Some(kind) = std::env::var("XAVIER2_MODEL_PROVIDER")
            .ok()
            .and_then(|value| ModelProviderKind::from_env(&value))
        {
            return Self::from_explicit_kind(kind);
        }

        for kind in [
            ModelProviderKind::Local,
            ModelProviderKind::OpenAI,
            ModelProviderKind::Anthropic,
            ModelProviderKind::DeepSeek,
            ModelProviderKind::MiniMax,
            ModelProviderKind::Gemini,
        ] {
            let config = Self::from_explicit_kind(kind);
            if config.is_configured() {
                return config;
            }
        }

        Self::from_explicit_kind(ModelProviderKind::Disabled)
    }

    fn from_explicit_kind(kind: ModelProviderKind) -> Self {
        match kind {
            ModelProviderKind::Gemini => Self {
                kind,
                model: std::env::var("XAVIER2_LLM_MODEL")
                    .or_else(|_| std::env::var("GEMINI_MODEL"))
                    .unwrap_or_else(|_| "gemini-2.0-flash".to_string()),
                api_key: std::env::var("GEMINI_API_KEY").ok(),
                url: None,
            },
            ModelProviderKind::OpenAI => Self {
                kind,
                model: std::env::var("XAVIER2_LLM_MODEL")
                    .or_else(|_| std::env::var("OPENAI_MODEL"))
                    .unwrap_or_else(|_| "gpt-4o-mini".to_string()),
                api_key: std::env::var("OPENAI_API_KEY").ok(),
                url: None,
            },
            ModelProviderKind::MiniMax => Self {
                kind,
                model: std::env::var("XAVIER2_LLM_MODEL")
                    .or_else(|_| std::env::var("MINIMAX_MODEL"))
                    .unwrap_or_else(|_| "MiniMax-Text-01".to_string()),
                api_key: std::env::var("MINIMAX_API_KEY").ok(),
                url: None,
            },
            ModelProviderKind::DeepSeek => Self {
                kind,
                model: std::env::var("XAVIER2_LLM_MODEL")
                    .or_else(|_| std::env::var("DEEPSEEK_MODEL"))
                    .unwrap_or_else(|_| "deepseek-chat".to_string()),
                api_key: std::env::var("DEEPSEEK_API_KEY").ok(),
                url: None,
            },
            ModelProviderKind::Anthropic => Self {
                kind,
                model: std::env::var("XAVIER2_LLM_MODEL")
                    .or_else(|_| std::env::var("ANTHROPIC_MODEL"))
                    .unwrap_or_else(|_| "claude-3-5-sonnet-latest".to_string()),
                api_key: std::env::var("ANTHROPIC_API_KEY").ok(),
                url: None,
            },
            ModelProviderKind::Local => Self {
                kind,
                model: std::env::var("XAVIER2_LOCAL_LLM_MODEL")
                    .or_else(|_| std::env::var("XAVIER2_LLM_MODEL"))
                    .unwrap_or_else(|_| "llama3".to_string()),
                api_key: None,
                url: Some(
                    std::env::var("XAVIER2_LOCAL_LLM_URL")
                        .unwrap_or_else(|_| "http://localhost:11434/v1".to_string()),
                ),
            },
            ModelProviderKind::Disabled => Self {
                kind,
                model: "disabled".to_string(),
                api_key: None,
                url: None,
            },
        }
    }

    pub fn is_configured(&self) -> bool {
        if self.kind == ModelProviderKind::Local {
            return self
                .url
                .as_ref()
                .is_some_and(|value| !value.trim().is_empty());
        }
        self.api_key
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty())
    }

    pub fn get_all_configured() -> Vec<Self> {
        let mut configured = Vec::new();
        for kind in [
            ModelProviderKind::Gemini,
            ModelProviderKind::OpenAI,
            ModelProviderKind::MiniMax,
            ModelProviderKind::DeepSeek,
            ModelProviderKind::Anthropic,
            ModelProviderKind::Local,
        ] {
            let config = Self::from_explicit_kind(kind);
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
            provider: self.config.kind.as_str().to_string(),
            model: self.config.model.clone(),
            configured: self.config.is_configured(),
        }
    }

    pub async fn generate_response(
        &self,
        query: &str,
        context: &[RetrievedDocument],
    ) -> Result<String> {
        if !self.config.is_configured() || self.config.kind == ModelProviderKind::Disabled {
            bail!("no LLM provider configured");
        }

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

        match self.config.kind {
            ModelProviderKind::Gemini => self.generate_gemini(system_prompt, &user_prompt).await,
            ModelProviderKind::OpenAI => self.generate_openai(system_prompt, &user_prompt).await,
            ModelProviderKind::MiniMax => self.generate_minimax(system_prompt, &user_prompt).await,
            ModelProviderKind::DeepSeek => self.generate_deepseek(system_prompt, &user_prompt).await,
            ModelProviderKind::Local => self.generate_local(system_prompt, &user_prompt).await,
            ModelProviderKind::Anthropic => {
                Err(anyhow!("anthropic provider is not implemented yet"))
            }
            ModelProviderKind::Disabled => Err(anyhow!("LLM provider is disabled")),
        }
    }

    pub async fn generate_hypothetical_document(&self, query: &str) -> Result<String> {
        if !self.config.is_configured() || self.config.kind == ModelProviderKind::Disabled {
            bail!("no LLM provider configured");
        }

        let system_prompt = "You are an expert knowledge system. Generate a hypothetical, highly plausible document snippet or answer that directly addresses the user's query. Do not include introductory or concluding remarks. Write only the factual content as if it were a real, authoritative reference document.";
        let user_prompt = query.to_string();

        match self.config.kind {
            ModelProviderKind::Gemini => self.generate_gemini(system_prompt, &user_prompt).await,
            ModelProviderKind::OpenAI => self.generate_openai(system_prompt, &user_prompt).await,
            ModelProviderKind::MiniMax => self.generate_minimax(system_prompt, &user_prompt).await,
            ModelProviderKind::DeepSeek => self.generate_deepseek(system_prompt, &user_prompt).await,
            ModelProviderKind::Local => self.generate_local(system_prompt, &user_prompt).await,
            ModelProviderKind::Anthropic => {
                Err(anyhow!("anthropic provider is not implemented yet"))
            }
            ModelProviderKind::Disabled => Err(anyhow!("LLM provider is disabled")),
        }
    }

    pub async fn evaluate_context(
        &self,
        query: &str,
        context: &[RetrievedDocument],
    ) -> Result<f32> {
        if !self.config.is_configured() || self.config.kind == ModelProviderKind::Disabled {
            return Ok(1.0); // Bypass if no LLM
        }

        let system_prompt = "You are a critical evaluator for a RAG system. Read the context and the user query. Evaluate if the context contains sufficient and accurate information to fully answer the query. Return ONLY a valid JSON object in this exact format: {\"confidence\": 0.95} where confidence is a float between 0.0 (useless) and 1.0 (perfect).";

        let context_text = context
            .iter()
            .map(|doc| format!("- {}", doc.content))
            .collect::<Vec<_>>()
            .join("\n\n");

        let user_prompt = format!("Context:\n{}\n\nQuery: {}", context_text, query);

        let response = match self.config.kind {
            ModelProviderKind::Gemini => self.generate_gemini(system_prompt, &user_prompt).await?,
            ModelProviderKind::OpenAI => self.generate_openai(system_prompt, &user_prompt).await?,
            ModelProviderKind::MiniMax => {
                self.generate_minimax(system_prompt, &user_prompt).await?
            }
            ModelProviderKind::DeepSeek => {
                self.generate_deepseek(system_prompt, &user_prompt).await?
            }
            ModelProviderKind::Local => self.generate_local(system_prompt, &user_prompt).await?,
            ModelProviderKind::Anthropic => bail!("anthropic not implemented"),
            ModelProviderKind::Disabled => return Ok(1.0),
        };

        let normalized = response.replace("```json", "").replace("```", "");
        let result: serde_json::Value = serde_json::from_str(normalized.trim())
            .unwrap_or_else(|_| serde_json::json!({"confidence": 1.0}));

        Ok(result["confidence"].as_f64().unwrap_or(1.0) as f32)
    }

    async fn generate_gemini(&self, system_prompt: &str, user_prompt: &str) -> Result<String> {
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

    async fn generate_openai(&self, system_prompt: &str, user_prompt: &str) -> Result<String> {
        let api_key = self
            .config
            .api_key
            .as_ref()
            .context("missing OpenAI API key")?;
        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .bearer_auth(api_key)
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
            .context("failed to call OpenAI API")?
            .error_for_status()
            .context("OpenAI API returned an error")?;
        let payload: serde_json::Value = response
            .json()
            .await
            .context("failed to decode OpenAI response")?;
        payload["choices"]
            .as_array()
            .and_then(|choices| choices.first())
            .and_then(|choice| choice["message"]["content"].as_str())
            .map(|text| text.to_string())
            .ok_or_else(|| anyhow!("OpenAI response did not contain text"))
    }

    async fn generate_minimax(&self, system_prompt: &str, user_prompt: &str) -> Result<String> {
        let api_key = self
            .config
            .api_key
            .as_ref()
            .context("missing MiniMax API key")?;
        let response = self
            .client
            .post("https://api.minimax.chat/v1/text/chatcompletion_pro")
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

    async fn generate_deepseek(&self, system_prompt: &str, user_prompt: &str) -> Result<String> {
        let api_key = self
            .config
            .api_key
            .as_ref()
            .context("missing DeepSeek API key")?;
        let response = self
            .client
            .post("https://api.deepseek.com/chat/completions")
            .bearer_auth(api_key)
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
            .context("failed to call DeepSeek API")?
            .error_for_status()
            .context("DeepSeek API returned an error")?;
        let payload: serde_json::Value = response
            .json()
            .await
            .context("failed to decode DeepSeek response")?;
        payload["choices"]
            .as_array()
            .and_then(|choices| choices.first())
            .and_then(|choice| choice["message"]["content"].as_str())
            .map(|text| text.to_string())
            .ok_or_else(|| anyhow!("DeepSeek response did not contain text"))
    }

    async fn generate_local(&self, system_prompt: &str, user_prompt: &str) -> Result<String> {
        let base_url = self
            .config
            .url
            .as_ref()
            .context("missing Local LLM URL")?
            .trim_end_matches('/');

        let endpoint = format!("{}/chat/completions", base_url);

        let response = self
            .client
            .post(endpoint)
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
            .context("failed to call Local LLM API")?
            .error_for_status()
            .context("Local LLM API returned an error")?;

        let payload: serde_json::Value = response
            .json()
            .await
            .context("failed to decode Local LLM response")?;

        payload["choices"]
            .as_array()
            .and_then(|choices| choices.first())
            .and_then(|choice| choice["message"]["content"].as_str())
            .map(|text| text.to_string())
            .ok_or_else(|| anyhow!("Local LLM response did not contain text"))
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

        let kind = ModelProviderKind::Local;
        let config = ModelProviderConfig::from_explicit_kind(kind);

        assert_eq!(config.model, "test-model");
        assert_eq!(config.url, Some("http://test-url/v1".to_string()));
    }

    #[test]
    fn test_local_provider_defaults() {
        let _guard = env_lock().lock().unwrap();
        std::env::remove_var("XAVIER2_LOCAL_LLM_MODEL");
        std::env::remove_var("XAVIER2_LLM_MODEL");
        std::env::remove_var("XAVIER2_LOCAL_LLM_URL");

        let kind = ModelProviderKind::Local;
        let config = ModelProviderConfig::from_explicit_kind(kind);

        assert_eq!(config.model, "llama3");
        assert_eq!(config.url, Some("http://localhost:11434/v1".to_string()));
    }
}
