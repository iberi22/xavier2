use crate::memory::semantic_cache::SemanticCache;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Response del System 3
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub query: String,
    pub response: String,
    pub actions_taken: Vec<Action>,
    pub memory_updates: Vec<MemoryUpdate>,
    pub tool_calls: Vec<ToolCall>,
    pub success: bool,
    pub semantic_cache_hit: bool,
    pub llm_used: bool,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub action_type: ActionType,
    pub description: String,
    pub target: Option<String>,
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    Response,
    MemoryStore,
    ToolExecution,
    BeliefUpdate,
    NoOp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUpdate {
    pub path: String,
    pub content: String,
    pub operation: MemoryOperation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryOperation {
    Create,
    Update,
    Delete,
    Compress,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub tool_name: String,
    pub arguments: serde_json::Value,
    pub result: Option<String>,
}

/// Config del Actor
#[derive(Clone)]
pub struct ActorConfig {
    pub use_llm: bool,
    pub max_actions: usize,
    pub semantic_cache: Option<Arc<SemanticCache>>,
    pub model_override: Option<String>,
    pub provider_override: Option<String>,
}

impl Default for ActorConfig {
    fn default() -> Self {
        Self {
            use_llm: true,
            max_actions: 5,
            semantic_cache: None,
            model_override: None,
            provider_override: None,
        }
    }
}
