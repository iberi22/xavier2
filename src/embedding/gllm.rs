use std::fmt;
#[cfg(feature = "local-gllm")]
use std::sync::Arc;

use crate::embedding::{Embedder, EmbeddingError};

pub const DEFAULT_GLLM_MODEL: &str = "all-MiniLM-L6-v2";
pub const DEFAULT_GLLM_DIMENSION: usize = 384;

#[cfg(feature = "local-gllm")]
type InnerEmbedder = ::gllm::FallbackEmbedder;

pub struct GllmEmbedder {
    #[cfg(feature = "local-gllm")]
    inner: Arc<InnerEmbedder>,
    model: String,
    dimension: usize,
}

impl fmt::Debug for GllmEmbedder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GllmEmbedder")
            .field("model", &self.model)
            .field("dimension", &self.dimension)
            .finish_non_exhaustive()
    }
}

impl GllmEmbedder {
    #[cfg(feature = "local-gllm")]
    pub fn new(model: String, dimension: usize) -> Result<Self, EmbeddingError> {
        let inner = ::gllm::FallbackEmbedder::new(&model)
            .map_err(|error| EmbeddingError::Config(error.to_string()))?;

        Ok(Self {
            inner: Arc::new(inner),
            model,
            dimension,
        })
    }

    #[cfg(not(feature = "local-gllm"))]
    pub fn new(model: String, _dimension: usize) -> Result<Self, EmbeddingError> {
        Err(EmbeddingError::Config(format!(
            "gllm embedder requested for model {model}, but Xavier was built without the local-gllm feature"
        )))
    }
}

#[async_trait::async_trait]
impl Embedder for GllmEmbedder {
    async fn encode(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        #[cfg(feature = "local-gllm")]
        {
            let inner = Arc::clone(&self.inner);
            let text = text.to_string();
            return tokio::task::spawn_blocking(move || inner.embed(&text))
                .await
                .map_err(|error| EmbeddingError::Network(error.to_string()))?
                .map_err(|error| EmbeddingError::Network(error.to_string()));
        }

        #[cfg(not(feature = "local-gllm"))]
        {
            let _ = text;
            Err(EmbeddingError::Config(
                "gllm embedder requested, but Xavier was built without the local-gllm feature"
                    .to_string(),
            ))
        }
    }

    fn dimension(&self) -> usize {
        self.dimension
    }
}

pub fn normalize_model_name(raw: &str) -> String {
    match raw.trim() {
        "" => DEFAULT_GLLM_MODEL.to_string(),
        "minilm-l6-v2" | "minilm-l6-v2-q4" | "all-minilm-l6-v2" => DEFAULT_GLLM_MODEL.to_string(),
        value => value.to_string(),
    }
}

pub fn dimension_for_model(model: &str) -> usize {
    match model.trim().to_ascii_lowercase().as_str() {
        "all-minilm-l6-v2"
        | "minilm-l6-v2"
        | "minilm-l6-v2-q4"
        | "all-minilm-l12-v2"
        | "bge-small-en"
        | "e5-small"
        | "jina-embeddings-v2-small-en"
        | "multilingual-minilm-l12-v2" => 384,
        "bge-small-zh" => 512,
        "bge-base-en" | "e5-base" | "all-mpnet-base-v2" | "m3e-base" => 768,
        "bge-large-en" | "e5-large" | "qwen3-embedding-0.6b" | "codexembed-400m" => 1024,
        "codexembed-2b" => 1536,
        "jina-embeddings-v4" => 2048,
        "qwen3-embedding-4b" => 2560,
        "qwen3-embedding-8b" | "llama-embed-nemotron-8b" | "codexembed-7b" => 4096,
        _ => DEFAULT_GLLM_DIMENSION,
    }
}
