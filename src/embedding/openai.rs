use std::fmt;
use std::time::Duration;

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::embedding::{Embedder, EmbeddingError};

pub struct OpenAICompatibleEmbedder {
    client: Client,
    api_key: Option<String>,
    model: String,
    endpoint: String,
    dimension: usize,
}

impl fmt::Debug for OpenAICompatibleEmbedder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAICompatibleEmbedder")
            .field("api_key", &self.api_key.as_ref().map(|_| "<redacted>"))
            .field("model", &self.model)
            .field("endpoint", &self.endpoint)
            .field("dimension", &self.dimension)
            .finish()
    }
}

impl OpenAICompatibleEmbedder {
    pub fn new(
        api_key: Option<String>,
        model: String,
        endpoint: String,
        dimension: usize,
    ) -> Result<Self, EmbeddingError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|error| EmbeddingError::Network(error.to_string()))?;

        Ok(Self {
            client,
            api_key: api_key.filter(|value| !value.trim().is_empty()),
            model,
            endpoint: endpoint.trim_end_matches('/').to_string(),
            dimension,
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
    #[serde(default)]
    data: Vec<EmbeddingData>,
    embedding: Option<Vec<f32>>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

impl EmbeddingResponse {
    fn first_embedding(self) -> Option<Vec<f32>> {
        self.embedding
            .or_else(|| self.data.into_iter().next().map(|item| item.embedding))
    }
}

#[async_trait::async_trait]
impl Embedder for OpenAICompatibleEmbedder {
    async fn encode(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        let mut request = self
            .client
            .post(&self.endpoint)
            .header("Content-Type", "application/json");

        if let Some(api_key) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request
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

        Ok(body.first_embedding().unwrap_or_default())
    }

    fn dimension(&self) -> usize {
        self.dimension
    }
}
