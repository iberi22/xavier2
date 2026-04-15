use crate::ports::outbound::EmbeddingPort;
use async_trait::async_trait;

pub struct EmbeddingAdapter {
    // EmbeddingClient placeholder
}

impl EmbeddingAdapter {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for EmbeddingAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EmbeddingPort for EmbeddingAdapter {
    async fn embed(&self, _text: &str) -> anyhow::Result<Vec<f32>> {
        todo!()
    }
}
