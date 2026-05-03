use std::sync::Arc;

use crate::memory::embedder::EmbeddingClient;
use crate::ports::outbound::EmbeddingPort;
use async_trait::async_trait;

pub struct EmbeddingAdapter {
    client: Arc<EmbeddingClient>,
}

impl EmbeddingAdapter {
    pub fn new(client: EmbeddingClient) -> Self {
        Self {
            client: Arc::new(client),
        }
    }

    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self::new(EmbeddingClient::from_env()?))
    }
}

impl Default for EmbeddingAdapter {
    fn default() -> Self {
        Self::from_env().expect("EmbeddingAdapter::default requires a configured embedding backend")
    }
}

pub fn build_embedding_port_from_env() -> anyhow::Result<Arc<dyn EmbeddingPort>> {
    Ok(Arc::new(EmbeddingAdapter::from_env()?))
}

#[async_trait]
impl EmbeddingPort for EmbeddingAdapter {
    async fn embed(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        self.client.embed(text).await
    }
}
