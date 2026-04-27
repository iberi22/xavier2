//! Context regeneration system.
//!
//! Sprint 1 covers:
//! - Phase 0: prompt classification (`Minimal`, `Medium`, `Maximum`)
//! - Phase 1: lexical retrieval with whitespace-tokenized BM25
//! - Phase 2: hybrid fusion with Reciprocal Rank Fusion (`k = 60`)
//! - Hook-based orchestration for `session_start` and `precompact`

pub mod bm25;
pub mod classifier;
pub mod hybrid;
pub mod orchestrator;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub use classifier::{ContextClassifier, ContextLevel};
pub use orchestrator::{ExecutionPlan, HookKind, Orchestrator};

/// Canonical unit used by context regeneration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextDocument {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    #[serde(default)]
    pub tool_calls: Vec<String>,
    #[serde(default)]
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub token_count: usize,
}

impl ContextDocument {
    pub fn new(
        id: impl Into<String>,
        session_id: impl Into<String>,
        role: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        let content = content.into();
        Self {
            id: id.into(),
            session_id: session_id.into(),
            role: role.into(),
            token_count: content.split_whitespace().count(),
            content,
            tool_calls: Vec::new(),
            metadata: serde_json::Value::Null,
            created_at: Utc::now(),
        }
    }

    pub fn with_tool_calls(mut self, tool_calls: Vec<String>) -> Self {
        self.tool_calls = tool_calls;
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn with_created_at(mut self, created_at: DateTime<Utc>) -> Self {
        self.created_at = created_at;
        self
    }

    pub fn with_token_count(mut self, token_count: usize) -> Self {
        self.token_count = token_count;
        self
    }
}
