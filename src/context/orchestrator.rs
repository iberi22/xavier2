use serde::{Deserialize, Serialize};

use super::{
    classifier::{ContextClassifier, ContextLevel},
    hybrid::{ContextSearchHit, HybridContextSearch},
    ContextDocument,
};

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

        let config = config_for(hook, level);
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
    ) -> Vec<&'a ContextDocument> {
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

        selected
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

#[derive(Debug, Clone, Copy)]
struct PlanConfig {
    max_documents: usize,
    max_tokens: usize,
    include_tool_calls: bool,
    include_metadata: bool,
}

fn config_for(hook: HookKind, level: ContextLevel) -> PlanConfig {
    match (hook, level) {
        (HookKind::SessionStart, ContextLevel::Minimal) => PlanConfig {
            max_documents: 3,
            max_tokens: 600,
            include_tool_calls: false,
            include_metadata: false,
        },
        (HookKind::SessionStart, ContextLevel::Medium) => PlanConfig {
            max_documents: 5,
            max_tokens: 1_200,
            include_tool_calls: true,
            include_metadata: false,
        },
        (HookKind::SessionStart, ContextLevel::Maximum) => PlanConfig {
            max_documents: 8,
            max_tokens: 2_400,
            include_tool_calls: true,
            include_metadata: true,
        },
        (HookKind::Precompact, ContextLevel::Minimal) => PlanConfig {
            max_documents: 4,
            max_tokens: 800,
            include_tool_calls: true,
            include_metadata: false,
        },
        (HookKind::Precompact, ContextLevel::Medium) => PlanConfig {
            max_documents: 7,
            max_tokens: 1_600,
            include_tool_calls: true,
            include_metadata: true,
        },
        (HookKind::Precompact, ContextLevel::Maximum) => PlanConfig {
            max_documents: 10,
            max_tokens: 3_200,
            include_tool_calls: true,
            include_metadata: true,
        },
    }
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
            .with_created_at(Utc.timestamp_opt(seconds, 0).unwrap())
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
