use std::fmt;
use std::time::Duration;

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::embedding::{Embedder, EmbeddingError};

pub struct OpenAIEmbedder {
    client: Client,
    api_key: String,
    model: String,
    endpoint: String,
}

impl fmt::Debug for OpenAIEmbedder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAIEmbedder")
            .field("api_key", &"<redacted>")
            .field("model", &self.model)
            .field("endpoint", &self.endpoint)
            .finish()
    }
}

impl OpenAIEmbedder {
    pub fn new(api_key: String, model: String, endpoint: String) -> Result<Self, EmbeddingError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|error| EmbeddingError::Network(error.to_string()))?;

        Ok(Self {
            client,
            api_key,
            model,
            endpoint: endpoint.trim_end_matches('/').to_string(),
        })
    }
}

#[derive(Debug, Serialize)]
struct EmbeddingRequest<'a> {
    input: &'a str,
    model: &'a str,
}

#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

#[async_trait::async_trait]
impl Embedder for OpenAIEmbedder {
    async fn encode(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        let response = self
            .client
            .post(&self.endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&EmbeddingRequest {
                input: text,
                model: &self.model,
            })
            .send()
            .await
            .map_err(|error| EmbeddingError::Network(error.to_string()))?
            .error_for_status()
            .map_err(|error| EmbeddingError::Network(error.to_string()))?;

        let body: EmbeddingResponse = response
            .json()
            .await
            .map_err(|error| EmbeddingError::Parse(error.to_string()))?;

        Ok(body
            .data
            .into_iter()
            .next()
            .map(|item| item.embedding)
            .unwrap_or_default())
    }

    fn dimension(&self) -> usize {
        match self.model.as_str() {
            "text-embedding-3-large" => 3072,
            "text-embedding-ada-002" => 1536,
            _ => 1536,
        }
    }
}
