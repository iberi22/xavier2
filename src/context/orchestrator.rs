use serde::{Deserialize, Serialize};

use super::{
    classifier::{ContextClassifier, ContextLevel},
    hybrid::{ContextSearchHit, HybridContextSearch},
    ContextDocument,
};
use crate::memory::virtual_memory::{MemoryReference, VirtualMemoryEntry};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HookKind {
    SessionStart,
    Precompact,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub hook: HookKind,
    pub level: ContextLevel,
    pub query: String,
    pub max_documents: usize,
    pub max_tokens: usize,
    pub include_tool_calls: bool,
    pub include_metadata: bool,
    pub selected_document_ids: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Orchestrator {
    classifier: ContextClassifier,
    search: HybridContextSearch,
    budgets: ContextBudgetConfig,
}

impl Default for Orchestrator {
    fn default() -> Self {
        Self::new()
    }
}

impl Orchestrator {
    pub fn new() -> Self {
        Self {
            classifier: ContextClassifier::new(),
            search: HybridContextSearch::default(),
            budgets: ContextBudgetConfig::from_env(),
        }
    }

    pub fn with_budgets(budgets: ContextBudgetConfig) -> Self {
        Self {
            classifier: ContextClassifier::new(),
            search: HybridContextSearch::default(),
            budgets,
        }
    }

    pub fn session_start(
        &self,
        session_id: &str,
        prompt: &str,
        documents: &[ContextDocument],
    ) -> ExecutionPlan {
        self.plan(HookKind::SessionStart, session_id, prompt, documents)
    }

    pub fn precompact(
        &self,
        session_id: &str,
        prompt: &str,
        documents: &[ContextDocument],
    ) -> ExecutionPlan {
        self.plan(HookKind::Precompact, session_id, prompt, documents)
    }

    fn plan(
        &self,
        hook: HookKind,
        session_id: &str,
        prompt: &str,
        documents: &[ContextDocument],
    ) -> ExecutionPlan {
        let level = self.classifier.classify(prompt);
        let session_documents: Vec<_> = documents
            .iter()
            .filter(|document| document.session_id == session_id)
            .cloned()
            .collect();

        let config = self.budgets.plan(hook, level);
        let query = build_query(prompt, level, hook);
        let selected = self
            .search
            .search(&session_documents, &query, config.max_documents);

        ExecutionPlan {
            hook,
            level,
            query,
            max_documents: config.max_documents,
            max_tokens: config.max_tokens,
            include_tool_calls: config.include_tool_calls,
            include_metadata: config.include_metadata,
            selected_document_ids: selected.into_iter().map(|hit| hit.document.id).collect(),
        }
    }

    pub fn execute<'a>(
        &self,
        plan: &ExecutionPlan,
        documents: &'a [ContextDocument],
    ) -> Vec<ContextDocument> {
        let mut by_id = std::collections::HashMap::new();
        for document in documents {
            by_id.insert(document.id.as_str(), document);
        }

        let mut selected = Vec::new();
        let mut total_tokens = 0usize;
        for document_id in &plan.selected_document_ids {
            let Some(document) = by_id.get(document_id.as_str()).copied() else {
                continue;
            };

            if total_tokens + document.token_count > plan.max_tokens && !selected.is_empty() {
                break;
            }

            total_tokens += document.token_count;
            selected.push(document);
        }

        let mut final_docs = Vec::new();
        for document in selected {
            if plan.level == ContextLevel::Minimal {
                // L0/L1 Virtualization: Only send summary and keywords
                let path = document.metadata["path"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string();
                let virtual_entry = VirtualMemoryEntry::new(
                    path,
                    document.content.clone(),
                    document.metadata.clone(),
                );
                let reference = virtual_entry.to_reference();
                let virtual_content = format!(
                    "REF: {} | SUMMARY: {} | KEYWORDS: {}",
                    reference.path,
                    reference.summary,
                    reference.keywords.join(", ")
                );

                let mut virtual_doc = document.clone();
                virtual_doc.content = virtual_content;
                virtual_doc.token_count = virtual_doc.content.split_whitespace().count();
                final_docs.push(virtual_doc);
            } else {
                final_docs.push(document.clone());
            }
        }

        final_docs
    }

    pub fn ranked_hits(
        &self,
        session_id: &str,
        prompt: &str,
        documents: &[ContextDocument],
        limit: usize,
    ) -> Vec<ContextSearchHit> {
        let session_documents: Vec<_> = documents
            .iter()
            .filter(|document| document.session_id == session_id)
            .cloned()
            .collect();
        self.search.search(&session_documents, prompt, limit)
    }
}

/// Default budget constants for context regeneration.
pub const DEFAULT_SS_MIN_DOCS: usize = 3;
pub const DEFAULT_SS_MIN_TOKENS: usize = 600;
pub const DEFAULT_SS_MED_DOCS: usize = 5;
pub const DEFAULT_SS_MED_TOKENS: usize = 1_200;
pub const DEFAULT_SS_MAX_DOCS: usize = 8;
pub const DEFAULT_SS_MAX_TOKENS: usize = 2_400;
pub const DEFAULT_PC_MIN_DOCS: usize = 4;
pub const DEFAULT_PC_MIN_TOKENS: usize = 800;
pub const DEFAULT_PC_MED_DOCS: usize = 7;
pub const DEFAULT_PC_MED_TOKENS: usize = 1_600;
pub const DEFAULT_PC_MAX_DOCS: usize = 10;
pub const DEFAULT_PC_MAX_TOKENS: usize = 3_200;

/// In-memory configuration for context regeneration budgets.
///
/// # Environment variables
/// - `XAVIER_CTX_SS_MIN_DOCS` / `XAVIER_CTX_SS_MIN_TOKENS` (SessionStart Minimal)
/// - `XAVIER_CTX_SS_MED_DOCS` / `XAVIER_CTX_SS_MED_TOKENS` (SessionStart Medium)
/// - `XAVIER_CTX_SS_MAX_DOCS` / `XAVIER_CTX_SS_MAX_TOKENS` (SessionStart Maximum)
/// - `XAVIER_CTX_PC_MIN_DOCS` / `XAVIER_CTX_PC_MIN_TOKENS` (Precompact Minimal)
/// - `XAVIER_CTX_PC_MED_DOCS` / `XAVIER_CTX_PC_MED_TOKENS` (Precompact Medium)
/// - `XAVIER_CTX_PC_MAX_DOCS` / `XAVIER_CTX_PC_MAX_TOKENS` (Precompact Maximum)
#[derive(Debug, Clone, Copy)]
pub struct ContextBudgetConfig {
    pub session_start_min_docs: usize,
    pub session_start_min_tokens: usize,
    pub session_start_med_docs: usize,
    pub session_start_med_tokens: usize,
    pub session_start_max_docs: usize,
    pub session_start_max_tokens: usize,
    pub precompact_min_docs: usize,
    pub precompact_min_tokens: usize,
    pub precompact_med_docs: usize,
    pub precompact_med_tokens: usize,
    pub precompact_max_docs: usize,
    pub precompact_max_tokens: usize,
}

impl Default for ContextBudgetConfig {
    fn default() -> Self {
        Self {
            session_start_min_docs: DEFAULT_SS_MIN_DOCS,
            session_start_min_tokens: DEFAULT_SS_MIN_TOKENS,
            session_start_med_docs: DEFAULT_SS_MED_DOCS,
            session_start_med_tokens: DEFAULT_SS_MED_TOKENS,
            session_start_max_docs: DEFAULT_SS_MAX_DOCS,
            session_start_max_tokens: DEFAULT_SS_MAX_TOKENS,
            precompact_min_docs: DEFAULT_PC_MIN_DOCS,
            precompact_min_tokens: DEFAULT_PC_MIN_TOKENS,
            precompact_med_docs: DEFAULT_PC_MED_DOCS,
            precompact_med_tokens: DEFAULT_PC_MED_TOKENS,
            precompact_max_docs: DEFAULT_PC_MAX_DOCS,
            precompact_max_tokens: DEFAULT_PC_MAX_TOKENS,
        }
    }
}

impl ContextBudgetConfig {
    /// Load from environment variables with validated defaults.
    pub fn from_env() -> Self {
        let def = Self::default();
        Self {
            session_start_min_docs: env_or("XAVIER_CTX_SS_MIN_DOCS", def.session_start_min_docs),
            session_start_min_tokens: env_or(
                "XAVIER_CTX_SS_MIN_TOKENS",
                def.session_start_min_tokens,
            ),
            session_start_med_docs: env_or("XAVIER_CTX_SS_MED_DOCS", def.session_start_med_docs),
            session_start_med_tokens: env_or(
                "XAVIER_CTX_SS_MED_TOKENS",
                def.session_start_med_tokens,
            ),
            session_start_max_docs: env_or("XAVIER_CTX_SS_MAX_DOCS", def.session_start_max_docs),
            session_start_max_tokens: env_or(
                "XAVIER_CTX_SS_MAX_TOKENS",
                def.session_start_max_tokens,
            ),
            precompact_min_docs: env_or("XAVIER_CTX_PC_MIN_DOCS", def.precompact_min_docs),
            precompact_min_tokens: env_or("XAVIER_CTX_PC_MIN_TOKENS", def.precompact_min_tokens),
            precompact_med_docs: env_or("XAVIER_CTX_PC_MED_DOCS", def.precompact_med_docs),
            precompact_med_tokens: env_or("XAVIER_CTX_PC_MED_TOKENS", def.precompact_med_tokens),
            precompact_max_docs: env_or("XAVIER_CTX_PC_MAX_DOCS", def.precompact_max_docs),
            precompact_max_tokens: env_or("XAVIER_CTX_PC_MAX_TOKENS", def.precompact_max_tokens),
        }
    }

    fn plan(&self, hook: HookKind, level: ContextLevel) -> PlanConfig {
        let (max_documents, max_tokens) = match (hook, level) {
            (HookKind::SessionStart, ContextLevel::Minimal) => {
                (self.session_start_min_docs, self.session_start_min_tokens)
            }
            (HookKind::SessionStart, ContextLevel::Medium) => {
                (self.session_start_med_docs, self.session_start_med_tokens)
            }
            (HookKind::SessionStart, ContextLevel::Maximum) => {
                (self.session_start_max_docs, self.session_start_max_tokens)
            }
            (HookKind::Precompact, ContextLevel::Minimal) => {
                (self.precompact_min_docs, self.precompact_min_tokens)
            }
            (HookKind::Precompact, ContextLevel::Medium) => {
                (self.precompact_med_docs, self.precompact_med_tokens)
            }
            (HookKind::Precompact, ContextLevel::Maximum) => {
                (self.precompact_max_docs, self.precompact_max_tokens)
            }
        };
        // SessionStart Minimal excludes tool_calls/metadata by default; all others include them.
        let (include_tool_calls, include_metadata) = match (hook, level) {
            (HookKind::SessionStart, ContextLevel::Minimal) => (false, false),
            _ => (true, true),
        };
        PlanConfig {
            max_documents,
            max_tokens,
            include_tool_calls,
            include_metadata,
        }
    }
}

fn env_or<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

#[derive(Debug, Clone, Copy)]
struct PlanConfig {
    max_documents: usize,
    max_tokens: usize,
    include_tool_calls: bool,
    include_metadata: bool,
}

fn build_query(prompt: &str, level: ContextLevel, hook: HookKind) -> String {
    match (hook, level) {
        (HookKind::SessionStart, ContextLevel::Minimal) => prompt.to_string(),
        (HookKind::SessionStart, ContextLevel::Medium) => format!("{prompt} previous context"),
        (HookKind::SessionStart, ContextLevel::Maximum) => {
            format!("{prompt} previous context tool outputs codebase decisions")
        }
        (HookKind::Precompact, ContextLevel::Minimal) => format!("{prompt} summary"),
        (HookKind::Precompact, ContextLevel::Medium) => {
            format!("{prompt} summary tool outputs decisions")
        }
        (HookKind::Precompact, ContextLevel::Maximum) => {
            format!("{prompt} summary tool outputs decisions architecture regression")
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use serde_json::json;

    use super::*;

    fn doc(
        id: &str,
        session_id: &str,
        role: &str,
        content: &str,
        token_count: usize,
        seconds: i64,
    ) -> ContextDocument {
        ContextDocument::new(id, session_id, role, content)
            .with_token_count(token_count)
            .with_tool_calls(vec!["cargo_test".to_string()])
            .with_metadata(json!({"topic": "build", "kind": "summary"}))
            .with_created_at(
                Utc.timestamp_opt(seconds, 0)
                    .single()
                    .expect("test assertion"),
            )
    }

    #[test]
    fn session_start_plan_is_scoped_to_session() {
        let orchestrator = Orchestrator::new();
        let documents = vec![
            doc("1", "s-1", "assistant", "build regression in rust", 120, 1),
            doc("2", "s-1", "user", "cargo test still fails", 120, 2),
            doc(
                "3",
                "s-2",
                "assistant",
                "different session entirely",
                120,
                3,
            ),
        ];

        let plan = orchestrator.session_start("s-1", "debug the build regression", &documents);

        assert_eq!(plan.hook, HookKind::SessionStart);
        assert_eq!(plan.level, ContextLevel::Maximum);
        assert!(plan.selected_document_ids.iter().all(|id| id != "3"));
        assert!(!plan.selected_document_ids.is_empty());
    }

    #[test]
    fn precompact_plan_has_larger_budget_than_session_start() {
        let orchestrator = Orchestrator::new();
        let documents = vec![doc("1", "s-1", "assistant", "quick build summary", 100, 1)];

        let start = orchestrator.session_start("s-1", "continue from previous context", &documents);
        let compact = orchestrator.precompact("s-1", "continue from previous context", &documents);

        assert!(compact.max_documents > start.max_documents);
        assert!(compact.max_tokens > start.max_tokens);
        assert!(compact.include_metadata);
    }

    #[test]
    fn execute_respects_token_budget_after_first_document() {
        let orchestrator = Orchestrator::new();
        let documents = vec![
            doc("1", "s-1", "assistant", "build regression in rust", 700, 1),
            doc(
                "2",
                "s-1",
                "assistant",
                "cargo test output and fixes",
                700,
                2,
            ),
        ];

        let plan = ExecutionPlan {
            hook: HookKind::SessionStart,
            level: ContextLevel::Minimal,
            query: "build".to_string(),
            max_documents: 2,
            max_tokens: 800,
            include_tool_calls: false,
            include_metadata: false,
            selected_document_ids: vec!["1".to_string(), "2".to_string()],
        };

        let selected = orchestrator.execute(&plan, &documents);

        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].id, "1");
    }
}
