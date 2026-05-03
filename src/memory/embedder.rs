use std::sync::Arc;

use anyhow::{anyhow, Result};

use crate::embedding::{build_embedder_from_env, Embedder, EmbedderConfig};

pub struct EmbeddingClient {
    embedder: Arc<dyn Embedder>,
}

impl EmbeddingClient {
    pub fn from_env() -> Result<Self> {
        let config = EmbedderConfig::from_env();
        if !config.is_configured() {
            return Err(anyhow!("embedding provider is not configured"));
        }

        Ok(Self {
            embedder: config.build_sync().map_err(|error| anyhow!(error.to_string()))?,
        })
    }

    pub fn is_configured_from_env() -> bool {
        EmbedderConfig::from_env().is_configured()
    }

    pub async fn embed(&self, input: &str) -> Result<Vec<f32>> {
        self.embedder
            .encode(input)
            .await
            .map_err(|error| anyhow!(error.to_string()))
    }

    pub async fn health(&self) -> Result<bool> {
        Ok(!self.embed("health check").await?.is_empty())
    }

    pub async fn from_env_async() -> Result<Self> {
        let config = EmbedderConfig::from_env();
        if !config.is_configured() {
            return Err(anyhow!("embedding provider is not configured"));
        }

        Ok(Self {
            embedder: build_embedder_from_env()
                .await
                .map_err(|error| anyhow!(error.to_string()))?,
        })
    }
}
