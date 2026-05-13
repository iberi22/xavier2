use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::agents::system1::RetrievalResult;
use crate::agents::system2::ReasoningResult;
use crate::memory::semantic_cache::SemanticCache;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActorConfig {
    #[serde(skip)]
    pub semantic_cache: Option<Arc<SemanticCache>>,
    pub model_override: Option<String>,
    pub provider_override: Option<String>,
    pub max_tokens: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub query: String,
    pub response: String,
    pub actions_taken: Vec<String>,
    pub memory_updates: Vec<String>,
    pub tool_calls: Vec<String>,
    pub success: bool,
    pub semantic_cache_hit: bool,
    pub llm_used: bool,
    pub model: Option<String>,
}

pub struct System3Actor {
    config: ActorConfig,
}

impl System3Actor {
    pub fn new(config: ActorConfig) -> Self {
        Self { config }
    }

    pub fn simple_response(query: &str, _documents: &[crate::agents::system1::RetrievedDocument], _category: Option<&str>) -> String {
        format!("Simple response for: {}", query)
    }

    pub async fn run(&self, query: &str, _retrieval: &RetrievalResult, _reasoning: &ReasoningResult, _category: Option<&str>) -> Result<ActionResult> {
        Ok(ActionResult {
            query: query.to_string(),
            response: format!("Full response for: {} (max_tokens: {:?})", query, self.config.max_tokens),
            actions_taken: vec![],
            memory_updates: vec![],
            tool_calls: vec![],
            success: true,
            semantic_cache_hit: false,
            llm_used: true,
            model: self.config.model_override.clone(),
        })
    }
}
