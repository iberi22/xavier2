use serde::{Deserialize, Serialize};

use super::bm25::tokenize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextLevel {
    Minimal,
    Medium,
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

        if contains_any(&lowered, &MAXIMUM_KEYWORDS) || token_count >= 80 {
            return ContextLevel::Maximum;
        }

        if contains_any(&lowered, &MEDIUM_KEYWORDS) || token_count >= 20 {
            return ContextLevel::Medium;
        }

        ContextLevel::Minimal
    }
}

const MEDIUM_KEYWORDS: [&str; 8] = [
    "continue",
    "resume",
    "follow up",
    "context",
    "history",
    "previous",
    "prior",
    "tool",
];

const MAXIMUM_KEYWORDS: [&str; 10] = [
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
            classifier.classify("continue from previous context and tool outputs"),
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
}
