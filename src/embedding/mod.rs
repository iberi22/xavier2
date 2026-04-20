use std::sync::Arc;

use async_trait::async_trait;
use thiserror::Error;

pub mod openai;

#[derive(Debug, Error)]
pub enum EmbeddingError {
    #[error("embedding provider configuration error: {0}")]
    Config(String),
    #[error("embedding network error: {0}")]
    Network(String),
    #[error("embedding parse error: {0}")]
    Parse(String),
}

#[async_trait]
pub trait Embedder: Send + Sync {
    async fn encode(&self, text: &str) -> Result<Vec<f32>, EmbeddingError>;
    fn dimension(&self) -> usize;
}

#[derive(Clone, Debug)]
pub enum EmbedderConfig {
    OpenAI {
        api_key: String,
        model: String,
        endpoint: String,
    },
    Noop,
}

impl EmbedderConfig {
    pub fn from_env() -> Self {
        match std::env::var("XAVIER2_EMBEDDER")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str()
        {
            "openai" => Self::OpenAI {
                api_key: std::env::var("OPENAI_API_KEY").unwrap_or_default(),
                model: std::env::var("XAVIER2_EMBEDDING_MODEL")
                    .unwrap_or_else(|_| "text-embedding-3-small".to_string()),
                endpoint: std::env::var("XAVIER2_EMBEDDING_ENDPOINT")
                    .unwrap_or_else(|_| "https://api.openai.com/v1/embeddings".to_string()),
            },
            _ => {
                if std::env::var("OPENAI_API_KEY").is_ok() {
                    Self::OpenAI {
                        api_key: std::env::var("OPENAI_API_KEY").unwrap_or_default(),
                        model: std::env::var("XAVIER2_EMBEDDING_MODEL")
                            .unwrap_or_else(|_| "text-embedding-3-small".to_string()),
                        endpoint: std::env::var("XAVIER2_EMBEDDING_ENDPOINT")
                            .unwrap_or_else(|_| "https://api.openai.com/v1/embeddings".to_string()),
                    }
                } else {
                    Self::Noop
                }
            }
        }
    }

    pub async fn build(self) -> Result<Arc<dyn Embedder>, EmbeddingError> {
        match self {
            Self::OpenAI {
                api_key,
                model,
                endpoint,
            } => {
                if api_key.trim().is_empty() {
                    return Err(EmbeddingError::Config(
                        "OPENAI_API_KEY is required for XAVIER2_EMBEDDER=openai".to_string(),
                    ));
                }

                Ok(Arc::new(openai::OpenAIEmbedder::new(
                    api_key, model, endpoint,
                )?))
            }
            Self::Noop => Ok(Arc::new(NoopEmbedder)),
        }
    }
}

pub async fn build_embedder_from_env() -> Result<Arc<dyn Embedder>, EmbeddingError> {
    EmbedderConfig::from_env().build().await
}

#[derive(Debug, Default)]
pub struct NoopEmbedder;

#[async_trait]
impl Embedder for NoopEmbedder {
    async fn encode(&self, _text: &str) -> Result<Vec<f32>, EmbeddingError> {
        Ok(Vec::new())
    }

    fn dimension(&self) -> usize {
        0
    }
}
