use serde::{Deserialize, Serialize};

use super::bm25::tokenize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextLevel {
    /// minimo -> only system prompt
    Minimal,
    /// medio -> system + recent context
    Medium,
    /// maximo -> full retrieval + skills
    Maximum,
}

#[derive(Debug, Clone, Default)]
pub struct ContextClassifier;

impl ContextClassifier {
    pub fn new() -> Self {
        Self
    }

    pub fn classify(&self, prompt: &str) -> ContextLevel {
        let token_count = tokenize(prompt).len();
        let lowered = prompt.to_lowercase();

        // maximo: full retrieval + skills (architecture, debug, deep dive, etc)
        if contains_any(&lowered, &MAXIMUM_KEYWORDS) || token_count >= 100 {
            return ContextLevel::Maximum;
        }

        // medio: system + recent context (follow up, continue, etc)
        if contains_any(&lowered, &MEDIUM_KEYWORDS) || token_count >= 20 {
            return ContextLevel::Medium;
        }

        // minimo: only system prompt
        ContextLevel::Minimal
    }
}

const MEDIUM_KEYWORDS: [&str; 10] = [
    "continue",
    "resume",
    "follow up",
    "context",
    "history",
    "previous",
    "prior",
    "tool",
    "yesterday",
    "last message",
];

const MAXIMUM_KEYWORDS: [&str; 15] = [
    "debug",
    "incident",
    "root cause",
    "architecture",
    "refactor",
    "migration",
    "regression",
    "codebase",
    "full context",
    "deep dive",
    "implement",
    "search",
    "find",
    "analyze",
    "skill",
];

fn contains_any(input: &str, keywords: &[&str]) -> bool {
    keywords.iter().any(|keyword| input.contains(keyword))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_short_prompt_as_minimal() {
        let classifier = ContextClassifier::new();
        assert_eq!(classifier.classify("status?"), ContextLevel::Minimal);
    }

    #[test]
    fn classifies_contextual_prompt_as_medium() {
        let classifier = ContextClassifier::new();
        assert_eq!(
            classifier.classify("continue from previous context"),
            ContextLevel::Medium
        );
    }

    #[test]
    fn classifies_complex_prompt_as_maximum() {
        let classifier = ContextClassifier::new();
        assert_eq!(
            classifier.classify("need a deep dive into the codebase to debug a regression"),
            ContextLevel::Maximum
        );
    }

    #[test]
    fn classifies_skill_request_as_maximum() {
        let classifier = ContextClassifier::new();
        assert_eq!(
            classifier.classify("how do I use the memory skill?"),
            ContextLevel::Maximum
        );
    }
}
