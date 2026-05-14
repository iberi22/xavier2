use crate::agents::provider::ModelProviderClient;
use crate::agents::system1::RetrievedDocument;
use anyhow::Result;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub trait ResponseGenerator: Send + Sync {
    fn generate_response<'a>(
        &'a self,
        query: &'a str,
        context: &'a [RetrievedDocument],
    ) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>>;
}

impl ResponseGenerator for ModelProviderClient {
    fn generate_response<'a>(
        &'a self,
        query: &'a str,
        context: &'a [RetrievedDocument],
    ) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>> {
        Box::pin(async move { ModelProviderClient::generate_response(self, query, context).await })
    }
}

/// Cliente LLM para generar respuestas
pub struct LlmClient {
    provider: Arc<dyn ResponseGenerator>,
    model_label: Option<String>,
}

impl Default for LlmClient {
    fn default() -> Self {
        Self::new(None, None)
    }
}

impl LlmClient {
    pub fn new(model_override: Option<String>, provider_override: Option<String>) -> Self {
        let provider = if let Some(p) = provider_override {
            ModelProviderClient::for_provider(&p, model_override)
        } else {
            ModelProviderClient::from_model_override(model_override)
        };
        let status = provider.status();
        Self {
            provider: Arc::new(provider),
            model_label: Some(status.model),
        }
    }

    pub fn with_config(config: crate::agents::provider::ModelProviderConfig) -> Self {
        let provider = ModelProviderClient::new(config);
        let status = provider.status();
        Self {
            provider: Arc::new(provider),
            model_label: Some(status.model),
        }
    }

    pub async fn generate_response(
        &self,
        query: &str,
        context: &[RetrievedDocument],
    ) -> Result<String> {
        self.provider.generate_response(query, context).await
    }

    pub fn model_label(&self) -> Option<String> {
        self.model_label.clone()
    }

    #[cfg(test)]
    pub(crate) fn with_provider(provider: Arc<dyn ResponseGenerator>) -> Self {
        Self {
            provider,
            model_label: None,
        }
    }
}
