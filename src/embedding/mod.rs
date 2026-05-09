use std::sync::Arc;

use async_trait::async_trait;
use thiserror::Error;

pub mod gllm;
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
    LocalGllm,
    Cloud,
    Auto,
    Disabled,
}

impl ProviderMode {
    fn from_env(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "local" => Some(Self::Local),
            "local-gllm" | "local_gllm" | "gllm" => Some(Self::LocalGllm),
            "cloud" => Some(Self::Cloud),
            "auto" => Some(Self::Auto),
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
pub(crate) struct GllmConfig {
    model: String,
    dimension: usize,
}

#[derive(Clone, Debug)]
pub(crate) enum EmbedderBackendConfig {
    Gllm(GllmConfig),
    OpenAICompatible(OpenAICompatibleConfig),
}

#[derive(Clone, Debug)]
pub(crate) enum EmbedderConfig {
    Fallback(Vec<EmbedderBackendConfig>),
    Noop,
}

impl EmbedderConfig {
    pub fn from_env() -> Self {
        let provider_mode = std::env::var("XAVIER_EMBEDDING_PROVIDER_MODE")
            .ok()
            .and_then(|value| ProviderMode::from_env(&value));
        let api_flavor = std::env::var("XAVIER_EMBEDDING_API_FLAVOR")
            .ok()
            .and_then(|value| ApiFlavor::from_env(&value))
            .unwrap_or(ApiFlavor::OpenAICompatible);

        let explicit_embedder = std::env::var("XAVIER_EMBEDDER")
            .ok()
            .map(|value| value.trim().to_ascii_lowercase());

        if provider_mode == Some(ProviderMode::Disabled)
            || explicit_embedder.as_deref() == Some("disabled")
        {
            return Self::Noop;
        }

        if explicit_embedder.as_deref() == Some("gllm") {
            return Self::gllm_only();
        }

        if api_flavor == ApiFlavor::AnthropicCompatible {
            return Self::Noop;
        }

        match provider_mode {
            Some(ProviderMode::Local) => Self::local_only(api_flavor),
            Some(ProviderMode::LocalGllm) => Self::gllm_only(),
            Some(ProviderMode::Cloud) => Self::cloud_only(api_flavor),
            Some(ProviderMode::Auto) => Self::auto_explicit(api_flavor),
            Some(ProviderMode::Disabled) => Self::Noop,
            None => Self::auto(api_flavor),
        }
    }

    pub fn is_configured(&self) -> bool {
        !matches!(self, Self::Noop)
    }

    pub fn build_sync(self) -> Result<Arc<dyn Embedder>, EmbeddingError> {
        match self {
            Self::Fallback(backends) => {
                let mut embedders: Vec<Arc<dyn Embedder>> = Vec::new();

                for backend in backends {
                    match build_backend(backend) {
                        Ok(embedder) => embedders.push(embedder),
                        Err(error) => {
                            tracing::warn!(%error, "embedding backend unavailable; trying fallback");
                        }
                    }
                }

                match embedders.len() {
                    0 => {
                        tracing::warn!(
                            "no embedding backend could be initialized; using no-op embedder"
                        );
                        Ok(Arc::new(NoopEmbedder))
                    }
                    1 => Ok(embedders.remove(0)),
                    _ => Ok(Arc::new(FallbackEmbedder { embedders })),
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
        let explicit_local_llm = std::env::var("XAVIER_MODEL_PROVIDER")
            .map(|value| value.eq_ignore_ascii_case("local"))
            .unwrap_or(false);

        match (local_signal || explicit_local_llm, cloud_signal) {
            (true, true) => Self::Fallback(vec![
                EmbedderBackendConfig::OpenAICompatible(local_config()),
                EmbedderBackendConfig::OpenAICompatible(cloud_config()),
            ]),
            (true, false) => Self::local_only(api_flavor),
            (false, true) => Self::cloud_only(api_flavor),
            (false, false) => Self::Noop,
        }
    }

    fn auto_explicit(api_flavor: ApiFlavor) -> Self {
        let mut backends = vec![EmbedderBackendConfig::Gllm(gllm_config())];

        if api_flavor == ApiFlavor::OpenAICompatible {
            backends.push(EmbedderBackendConfig::OpenAICompatible(local_config()));
        }

        if cloud_embedding_signal_present() {
            backends.push(EmbedderBackendConfig::OpenAICompatible(cloud_config()));
        }

        Self::Fallback(backends)
    }

    fn local_only(_api_flavor: ApiFlavor) -> Self {
        Self::Fallback(vec![
            EmbedderBackendConfig::Gllm(gllm_config()),
            EmbedderBackendConfig::OpenAICompatible(local_config()),
        ])
    }

    fn cloud_only(_api_flavor: ApiFlavor) -> Self {
        Self::Fallback(vec![
            EmbedderBackendConfig::OpenAICompatible(cloud_config()),
        ])
    }

    fn gllm_only() -> Self {
        Self::Fallback(vec![EmbedderBackendConfig::Gllm(gllm_config())])
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
    std::env::var("XAVIER_EMBEDDING_ENDPOINT").is_ok()
        || std::env::var("XAVIER_EMBEDDING_URL").is_ok()
        || std::env::var("XAVIER_EMBEDDING_MODEL").is_ok()
        || std::env::var("XAVIER_EMBEDDING_PROVIDER_MODE")
            .map(|value| value.eq_ignore_ascii_case("local"))
            .unwrap_or(false)
}

fn cloud_embedding_signal_present() -> bool {
    std::env::var("OPENAI_API_KEY").is_ok()
        || std::env::var("XAVIER_EMBEDDING_API_KEY").is_ok()
        || std::env::var("XAVIER_EMBEDDING_PROVIDER_MODE")
            .map(|value| value.eq_ignore_ascii_case("cloud"))
            .unwrap_or(false)
}

fn build_backend(config: EmbedderBackendConfig) -> Result<Arc<dyn Embedder>, EmbeddingError> {
    match config {
        EmbedderBackendConfig::Gllm(config) => Ok(Arc::new(gllm::GllmEmbedder::new(
            config.model,
            config.dimension,
        )?)),
        EmbedderBackendConfig::OpenAICompatible(config) => {
            Ok(Arc::new(openai::OpenAICompatibleEmbedder::new(
                config.api_key,
                config.model,
                config.endpoint,
                config.dimension,
            )?))
        }
    }
}

fn gllm_config() -> GllmConfig {
    let raw_model = std::env::var("XAVIER_GLLM_MODEL")
        .unwrap_or_else(|_| gllm::DEFAULT_GLLM_MODEL.to_string());
    let model = gllm::normalize_model_name(&raw_model);
    let dimension = std::env::var("XAVIER_GLLM_DIMENSION")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|dimension| *dimension > 0)
        .unwrap_or_else(|| gllm::dimension_for_model(&model));

    GllmConfig { model, dimension }
}

fn local_config() -> OpenAICompatibleConfig {
    let endpoint = std::env::var("XAVIER_EMBEDDING_ENDPOINT")
        .or_else(|_| std::env::var("XAVIER_EMBEDDING_URL"))
        .map(|value| normalize_openai_embeddings_endpoint(&value))
        .unwrap_or_else(|_| DEFAULT_LOCAL_EMBEDDING_ENDPOINT.to_string());

    let model = std::env::var("XAVIER_EMBEDDING_MODEL")
        .unwrap_or_else(|_| DEFAULT_LOCAL_EMBEDDING_MODEL.to_string());

    OpenAICompatibleConfig {
        api_key: std::env::var("XAVIER_EMBEDDING_API_KEY")
            .ok()
            .or_else(|| Some("ollama".to_string())),
        endpoint,
        dimension: embedding_dimension_for_model(&model),
        model,
    }
}

fn cloud_config() -> OpenAICompatibleConfig {
    let endpoint = std::env::var("XAVIER_EMBEDDING_ENDPOINT")
        .or_else(|_| std::env::var("XAVIER_EMBEDDING_URL"))
        .map(|value| normalize_openai_embeddings_endpoint(&value))
        .unwrap_or_else(|_| DEFAULT_CLOUD_EMBEDDING_ENDPOINT.to_string());

    let model = std::env::var("XAVIER_EMBEDDING_MODEL")
        .unwrap_or_else(|_| DEFAULT_CLOUD_EMBEDDING_MODEL.to_string());

    OpenAICompatibleConfig {
        api_key: std::env::var("XAVIER_EMBEDDING_API_KEY")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_mode_accepts_gllm_and_auto_aliases() {
        assert_eq!(
            ProviderMode::from_env("gllm"),
            Some(ProviderMode::LocalGllm)
        );
        assert_eq!(
            ProviderMode::from_env("local-gllm"),
            Some(ProviderMode::LocalGllm)
        );
        assert_eq!(ProviderMode::from_env("auto"), Some(ProviderMode::Auto));
    }

    #[test]
    fn gllm_minilm_aliases_normalize_to_supported_model() {
        assert_eq!(
            gllm::normalize_model_name("minilm-l6-v2-q4"),
            gllm::DEFAULT_GLLM_MODEL
        );
        assert_eq!(gllm::dimension_for_model("all-MiniLM-L6-v2"), 384);
        assert_eq!(gllm::dimension_for_model("qwen3-embedding-0.6b"), 1024);
    }
}
