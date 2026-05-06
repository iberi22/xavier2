use std::sync::Arc;

use async_trait::async_trait;
use thiserror::Error;

pub mod openai;

const DEFAULT_LOCAL_EMBEDDING_ENDPOINT: &str = "http://localhost:11434/v1/embeddings";
const DEFAULT_LOCAL_EMBEDDING_MODEL: &str = "embeddinggemma";
const DEFAULT_CLOUD_EMBEDDING_ENDPOINT: &str = "https://api.openai.com/v1/embeddings";
const DEFAULT_CLOUD_EMBEDDING_MODEL: &str = "text-embedding-3-small";

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderMode {
    Local,
    Cloud,
    Disabled,
}

impl ProviderMode {
    fn from_env(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "local" => Some(Self::Local),
            "cloud" => Some(Self::Cloud),
            "disabled" => Some(Self::Disabled),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiFlavor {
    OpenAICompatible,
    AnthropicCompatible,
}

impl ApiFlavor {
    fn from_env(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "openai-compatible" | "openai" => Some(Self::OpenAICompatible),
            "anthropic-compatible" | "anthropic" => Some(Self::AnthropicCompatible),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct OpenAICompatibleConfig {
    endpoint: String,
    api_key: Option<String>,
    model: String,
    dimension: usize,
}

#[derive(Clone, Debug)]
pub(crate) enum EmbedderConfig {
    OpenAICompatible {
        primary: OpenAICompatibleConfig,
        fallback: Option<OpenAICompatibleConfig>,
    },
    Noop,
}

impl EmbedderConfig {
    pub fn from_env() -> Self {
        let provider_mode = std::env::var("XAVIER2_EMBEDDING_PROVIDER_MODE")
            .ok()
            .and_then(|value| ProviderMode::from_env(&value));
        let api_flavor = std::env::var("XAVIER2_EMBEDDING_API_FLAVOR")
            .ok()
            .and_then(|value| ApiFlavor::from_env(&value))
            .unwrap_or(ApiFlavor::OpenAICompatible);

        if provider_mode == Some(ProviderMode::Disabled)
            || std::env::var("XAVIER2_EMBEDDER")
                .map(|value| value.eq_ignore_ascii_case("disabled"))
                .unwrap_or(false)
        {
            return Self::Noop;
        }

        if api_flavor == ApiFlavor::AnthropicCompatible {
            return Self::Noop;
        }

        match provider_mode {
            Some(ProviderMode::Local) => Self::local_only(api_flavor),
            Some(ProviderMode::Cloud) => Self::cloud_only(api_flavor),
            Some(ProviderMode::Disabled) => Self::Noop,
            None => Self::auto(api_flavor),
        }
    }

    pub fn is_configured(&self) -> bool {
        !matches!(self, Self::Noop)
    }

    pub fn build_sync(self) -> Result<Arc<dyn Embedder>, EmbeddingError> {
        match self {
            Self::OpenAICompatible {
                primary, fallback, ..
            } => {
                let mut embedders: Vec<Arc<dyn Embedder>> =
                    vec![Arc::new(openai::OpenAICompatibleEmbedder::new(
                        primary.api_key,
                        primary.model,
                        primary.endpoint,
                        primary.dimension,
                    )?)];

                if let Some(fallback) = fallback {
                    embedders.push(Arc::new(openai::OpenAICompatibleEmbedder::new(
                        fallback.api_key,
                        fallback.model,
                        fallback.endpoint,
                        fallback.dimension,
                    )?));
                }

                if embedders.len() == 1 {
                    Ok(embedders.remove(0))
                } else {
                    Ok(Arc::new(FallbackEmbedder { embedders }))
                }
            }
            Self::Noop => Ok(Arc::new(NoopEmbedder)),
        }
    }

    pub async fn build(self) -> Result<Arc<dyn Embedder>, EmbeddingError> {
        self.build_sync()
    }

    fn auto(api_flavor: ApiFlavor) -> Self {
        let local_signal = local_embedding_signal_present();
        let cloud_signal = cloud_embedding_signal_present();
        let explicit_local_llm = std::env::var("XAVIER2_MODEL_PROVIDER")
            .map(|value| value.eq_ignore_ascii_case("local"))
            .unwrap_or(false);

        match (local_signal || explicit_local_llm, cloud_signal) {
            (true, true) => Self::OpenAICompatible {
                primary: local_config(),
                fallback: Some(cloud_config()),
            },
            (true, false) => Self::local_only(api_flavor),
            (false, true) => Self::cloud_only(api_flavor),
            (false, false) => Self::Noop,
        }
    }

    fn local_only(_api_flavor: ApiFlavor) -> Self {
        Self::OpenAICompatible {
            primary: local_config(),
            fallback: None,
        }
    }

    fn cloud_only(_api_flavor: ApiFlavor) -> Self {
        Self::OpenAICompatible {
            primary: cloud_config(),
            fallback: None,
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

struct FallbackEmbedder {
    embedders: Vec<Arc<dyn Embedder>>,
}

#[async_trait]
impl Embedder for FallbackEmbedder {
    async fn encode(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        let mut last_error = None;
        for embedder in &self.embedders {
            match embedder.encode(text).await {
                Ok(vector) if !vector.is_empty() => return Ok(vector),
                Ok(_) => {
                    last_error = Some(EmbeddingError::Parse(
                        "embedding backend returned an empty vector".to_string(),
                    ))
                }
                Err(error) => last_error = Some(error),
            }
        }

        Err(last_error.unwrap_or_else(|| {
            EmbeddingError::Config("no embedding backend produced a usable vector".to_string())
        }))
    }

    fn dimension(&self) -> usize {
        self.embedders
            .iter()
            .map(|embedder| embedder.dimension())
            .find(|dimension| *dimension > 0)
            .unwrap_or(0)
    }
}

fn local_embedding_signal_present() -> bool {
    std::env::var("XAVIER2_EMBEDDING_ENDPOINT").is_ok()
        || std::env::var("XAVIER2_EMBEDDING_URL").is_ok()
        || std::env::var("XAVIER2_EMBEDDING_MODEL").is_ok()
        || std::env::var("XAVIER2_EMBEDDING_PROVIDER_MODE")
            .map(|value| value.eq_ignore_ascii_case("local"))
            .unwrap_or(false)
}

fn cloud_embedding_signal_present() -> bool {
    std::env::var("OPENAI_API_KEY").is_ok()
        || std::env::var("XAVIER2_EMBEDDING_API_KEY").is_ok()
        || std::env::var("XAVIER2_EMBEDDING_PROVIDER_MODE")
            .map(|value| value.eq_ignore_ascii_case("cloud"))
            .unwrap_or(false)
}

fn local_config() -> OpenAICompatibleConfig {
    let endpoint = std::env::var("XAVIER2_EMBEDDING_ENDPOINT")
        .or_else(|_| std::env::var("XAVIER2_EMBEDDING_URL"))
        .map(|value| normalize_openai_embeddings_endpoint(&value))
        .unwrap_or_else(|_| DEFAULT_LOCAL_EMBEDDING_ENDPOINT.to_string());

    let model = std::env::var("XAVIER2_EMBEDDING_MODEL")
        .unwrap_or_else(|_| DEFAULT_LOCAL_EMBEDDING_MODEL.to_string());

    OpenAICompatibleConfig {
        api_key: std::env::var("XAVIER2_EMBEDDING_API_KEY")
            .ok()
            .or_else(|| Some("ollama".to_string())),
        endpoint,
        dimension: embedding_dimension_for_model(&model),
        model,
    }
}

fn cloud_config() -> OpenAICompatibleConfig {
    let endpoint = std::env::var("XAVIER2_EMBEDDING_ENDPOINT")
        .or_else(|_| std::env::var("XAVIER2_EMBEDDING_URL"))
        .map(|value| normalize_openai_embeddings_endpoint(&value))
        .unwrap_or_else(|_| DEFAULT_CLOUD_EMBEDDING_ENDPOINT.to_string());

    let model = std::env::var("XAVIER2_EMBEDDING_MODEL")
        .unwrap_or_else(|_| DEFAULT_CLOUD_EMBEDDING_MODEL.to_string());

    OpenAICompatibleConfig {
        api_key: std::env::var("XAVIER2_EMBEDDING_API_KEY")
            .ok()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok()),
        endpoint,
        dimension: embedding_dimension_for_model(&model),
        model,
    }
}

fn normalize_openai_embeddings_endpoint(raw: &str) -> String {
    let trimmed = raw.trim_end_matches('/');
    if trimmed.ends_with("/v1/embeddings") || trimmed.ends_with("/api/embed") {
        trimmed.to_string()
    } else if trimmed.ends_with("/v1") {
        format!("{trimmed}/embeddings")
    } else if trimmed.ends_with("/embeddings") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/v1/embeddings")
    }
}

fn embedding_dimension_for_model(model: &str) -> usize {
    match model.trim().to_ascii_lowercase().as_str() {
        "embeddinggemma" => 768,
        "nomic-embed-text" | "nomic-embed-text-v1.5" => 768,
        "all-minilm" => 384,
        "qwen3-embedding" => 1024,
        "text-embedding-3-large" => 3072,
        "text-embedding-3-small" | "text-embedding-ada-002" => 1536,
        _ => 768,
    }
}
