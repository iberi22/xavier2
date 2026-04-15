use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const DEFAULT_EMBEDDING_URL: &str = "http://localhost:11434";
const DEFAULT_EMBEDDING_MODEL: &str = "nomic-embed-text";

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EmbedderProvider {
    Ollama,
    OpenAI,
    Legacy,
}

#[derive(Debug, Clone)]
pub struct EmbeddingClient {
    client: Client,
    base_url: String,
    model: String,
    provider: EmbedderProvider,
}

impl EmbeddingClient {
    pub fn from_env() -> Result<Self> {
        let base_url = std::env::var("XAVIER2_EMBEDDING_URL")
            .unwrap_or_else(|_| DEFAULT_EMBEDDING_URL.to_string());
        let model = std::env::var("XAVIER2_EMBEDDING_MODEL")
            .unwrap_or_else(|_| DEFAULT_EMBEDDING_MODEL.to_string());

        let provider = if base_url.contains("localhost:11434") || base_url.contains("ollama") {
            EmbedderProvider::Ollama
        } else if base_url.contains("8002") || model.contains("pplx") {
            EmbedderProvider::Legacy
        } else {
            EmbedderProvider::OpenAI
        };

        Self::new(base_url, model, provider)
    }

    pub fn new(
        base_url: impl Into<String>,
        model: impl Into<String>,
        provider: EmbedderProvider,
    ) -> Result<Self> {
        let base_url = base_url.into();
        if !base_url.starts_with("http://") && !base_url.starts_with("https://") {
            return Err(anyhow!(
                "embedding base URL must start with http:// or https://"
            ));
        }

        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .context("failed to build embedding HTTP client")?;

        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.into(),
            provider,
        })
    }

    pub fn provider(&self) -> EmbedderProvider {
        self.provider
    }

    pub async fn embed(&self, input: &str) -> Result<Vec<f32>> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }

        match self.provider {
            EmbedderProvider::Ollama => self.embed_ollama(trimmed).await,
            EmbedderProvider::OpenAI => self.embed_openai_compatible(trimmed).await,
            EmbedderProvider::Legacy => {
                if let Ok(vector) = self.embed_openai_compatible(trimmed).await {
                    Ok(vector)
                } else {
                    self.embed_legacy(trimmed).await
                }
            }
        }
    }

    pub async fn health(&self) -> Result<bool> {
        Ok(!self.embed("health check").await?.is_empty())
    }

    async fn embed_ollama(&self, input: &str) -> Result<Vec<f32>> {
        let response = self
            .client
            .post(format!("{}/api/embeddings", self.base_url))
            .json(&OllamaEmbeddingRequest {
                model: &self.model,
                prompt: input,
            })
            .send()
            .await
            .with_context(|| format!("failed to call Ollama embeddings at {}", self.base_url))?
            .error_for_status()
            .context("Ollama embeddings service returned an error")?;

        let payload: OllamaEmbeddingResponse = response
            .json()
            .await
            .context("failed to decode Ollama embeddings response")?;

        Ok(payload.embedding)
    }

    async fn embed_openai_compatible(&self, input: &str) -> Result<Vec<f32>> {
        let response = self
            .client
            .post(format!("{}/v1/embeddings", self.base_url))
            .json(&EmbeddingRequest {
                input,
                model: &self.model,
            })
            .send()
            .await
            .with_context(|| format!("failed to call embeddings service at {}", self.base_url))?
            .error_for_status()
            .context("embeddings service returned an error")?;

        let payload: EmbeddingResponse = response
            .json()
            .await
            .context("failed to decode embeddings response")?;

        payload
            .first_embedding()
            .ok_or_else(|| anyhow!("embeddings response did not contain a vector"))
    }

    async fn embed_legacy(&self, input: &str) -> Result<Vec<f32>> {
        let response = self
            .client
            .post(format!("{}/embed", self.base_url))
            .json(&serde_json::json!({ "text": input }))
            .send()
            .await
            .with_context(|| {
                format!(
                    "failed to call legacy embeddings service at {}",
                    self.base_url
                )
            })?
            .error_for_status()
            .context("legacy embeddings service returned an error")?;

        let payload: serde_json::Value = response
            .json()
            .await
            .context("failed to decode legacy embeddings response")?;

        payload["embeddings"]
            .as_array()
            .and_then(|items| items.first())
            .and_then(|vector| vector.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|value| value.as_f64().map(|number| number as f32))
                    .collect::<Vec<_>>()
            })
            .filter(|items| !items.is_empty())
            .ok_or_else(|| anyhow!("legacy embeddings response did not contain a vector"))
    }
}

#[derive(Debug, Serialize)]
struct OllamaEmbeddingRequest<'a> {
    model: &'a str,
    prompt: &'a str,
}

#[derive(Debug, Deserialize)]
struct OllamaEmbeddingResponse {
    embedding: Vec<f32>,
}

#[derive(Debug, Serialize)]
struct EmbeddingRequest<'a> {
    input: &'a str,
    model: &'a str,
}

#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    #[serde(default)]
    data: Vec<EmbeddingDatum>,
    embedding: Option<Vec<f32>>,
}

impl EmbeddingResponse {
    fn first_embedding(self) -> Option<Vec<f32>> {
        self.embedding
            .or_else(|| self.data.into_iter().next().map(|datum| datum.embedding))
    }
}

#[derive(Debug, Deserialize)]
struct EmbeddingDatum {
    embedding: Vec<f32>,
}

use crate::ports::outbound::EmbeddingPort;
use async_trait::async_trait;

pub struct EmbeddingAdapter {
    client: Option<EmbeddingClient>,
}

impl Default for EmbeddingAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl EmbeddingAdapter {
    pub fn new() -> Self {
        Self { client: None }
    }

    pub fn with_client(client: EmbeddingClient) -> Self {
        Self {
            client: Some(client),
        }
    }
}

#[async_trait]
impl EmbeddingPort for EmbeddingAdapter {
    async fn embed(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        match &self.client {
            Some(client) => client.embed(text).await,
            None => Err(anyhow!("EmbeddingAdapter not configured - no client")),
        }
    }
}
