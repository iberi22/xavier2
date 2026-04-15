//! Pattern adapter using in-memory HashMap store.
//!
//! This adapter implements PatternDiscoverPort using a thread-safe
//! in-memory HashMap for pattern storage.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use chrono::Utc;

use crate::domain::pattern::{PatternCategory, PatternVerification, VerifiedPattern};
use crate::ports::inbound::pattern_port::PatternDiscoverPort;
use ulid::Ulid;

/// In-memory pattern store backed by RwLock<HashMap>
pub struct PatternAdapter {
    patterns: Arc<RwLock<HashMap<String, VerifiedPattern>>>,
}

impl PatternAdapter {
    /// Create a new empty PatternAdapter
    pub fn new() -> Self {
        Self {
            patterns: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn row_to_pattern(&self, id: &str) -> Option<VerifiedPattern> {
        let patterns = self.patterns.read().ok()?;
        patterns.get(id).cloned()
    }

    fn auto_verify_if_ready(&self, _id: &str, pattern: &mut VerifiedPattern) {
        if pattern.usage_count >= 5
            && pattern.confidence >= 0.7
            && pattern.verification == PatternVerification::Pending
        {
            pattern.verification = PatternVerification::Verified;
            pattern.updated_at = Utc::now();
        }
    }
}

impl Default for PatternAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PatternDiscoverPort for PatternAdapter {
    async fn discover(&self, pattern: VerifiedPattern) -> anyhow::Result<String> {
        let id = if pattern.id.is_empty() {
            Ulid::new().to_string()
        } else {
            pattern.id.clone()
        };

        let mut pattern = pattern;
        pattern.id = id.clone();
        pattern.created_at = Utc::now();
        pattern.updated_at = Utc::now();

        let mut patterns = self
            .patterns
            .write()
            .map_err(|_| anyhow::anyhow!("failed to acquire write lock on pattern store"))?;

        patterns.insert(id.clone(), pattern);

        Ok(id)
    }

    async fn query(
        &self,
        project: &str,
        category: Option<PatternCategory>,
        min_confidence: f32,
    ) -> anyhow::Result<Vec<VerifiedPattern>> {
        let patterns = self
            .patterns
            .read()
            .map_err(|_| anyhow::anyhow!("failed to acquire read lock on pattern store"))?;

        let filtered: Vec<VerifiedPattern> = patterns
            .values()
            .filter(|p| {
                p.project == project
                    && p.confidence >= min_confidence
                    && category.is_none_or(|cat| p.category == cat)
            })
            .cloned()
            .collect();

        Ok(filtered)
    }

    async fn verify(&self, id: &str, verified: bool) -> anyhow::Result<()> {
        let mut patterns = self
            .patterns
            .write()
            .map_err(|_| anyhow::anyhow!("failed to acquire write lock on pattern store"))?;

        if let Some(pattern) = patterns.get_mut(id) {
            pattern.verification = if verified {
                PatternVerification::Verified
            } else {
                PatternVerification::Rejected
            };
            pattern.updated_at = Utc::now();
        }

        Ok(())
    }

    async fn get(&self, id: &str) -> anyhow::Result<Option<VerifiedPattern>> {
        Ok(self.row_to_pattern(id))
    }

    async fn delete(&self, id: &str) -> anyhow::Result<Option<VerifiedPattern>> {
        let mut patterns = self
            .patterns
            .write()
            .map_err(|_| anyhow::anyhow!("failed to acquire write lock on pattern store"))?;

        Ok(patterns.remove(id))
    }

    async fn increment_usage(&self, id: &str) -> anyhow::Result<()> {
        let mut patterns = self
            .patterns
            .write()
            .map_err(|_| anyhow::anyhow!("failed to acquire write lock on pattern store"))?;

        if let Some(pattern) = patterns.get_mut(id) {
            pattern.usage_count += 1;
            pattern.updated_at = Utc::now();
            self.auto_verify_if_ready(id, pattern);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_pattern() -> VerifiedPattern {
        VerifiedPattern {
            id: String::new(),
            category: PatternCategory::Naming,
            pattern: "snake_case".to_string(),
            project: "xavier2".to_string(),
            discovered_by: "agent-1".to_string(),
            confidence: 0.8,
            source_file: "src/utils.rs".to_string(),
            source_occurrences: 10,
            source_snippet: "fn my_function_name()".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            usage_count: 0,
            verification: PatternVerification::Pending,
        }
    }

    #[tokio::test]
    async fn discover_inserts_and_returns_id() {
        let adapter = PatternAdapter::new();
        let pattern = test_pattern();

        let id = adapter.discover(pattern.clone()).await.unwrap();
        assert!(!id.is_empty());

        let stored = adapter.get(&id).await.unwrap();
        assert!(stored.is_some());
        assert_eq!(stored.unwrap().pattern, "snake_case");
    }

    #[tokio::test]
    async fn query_filters_by_project_and_confidence() {
        let adapter = PatternAdapter::new();

        let mut p1 = test_pattern();
        p1.project = "xavier2".to_string();
        p1.confidence = 0.9;
        adapter.discover(p1).await.unwrap();

        let mut p2 = test_pattern();
        p2.project = "other".to_string();
        p2.confidence = 0.5;
        adapter.discover(p2).await.unwrap();

        let results = adapter.query("xavier2", None, 0.5).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].project, "xavier2");
    }

    #[tokio::test]
    async fn query_filters_by_category() {
        let adapter = PatternAdapter::new();

        let mut naming = test_pattern();
        naming.category = PatternCategory::Naming;
        adapter.discover(naming).await.unwrap();

        let mut structure = test_pattern();
        structure.category = PatternCategory::Structure;
        adapter.discover(structure).await.unwrap();

        let results = adapter
            .query("xavier2", Some(PatternCategory::Naming), 0.0)
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].category, PatternCategory::Naming);
    }

    #[tokio::test]
    async fn verify_sets_verification_status() {
        let adapter = PatternAdapter::new();
        let id = adapter.discover(test_pattern()).await.unwrap();

        adapter.verify(&id, true).await.unwrap();
        let pattern = adapter.get(&id).await.unwrap().unwrap();
        assert_eq!(pattern.verification, PatternVerification::Verified);

        adapter.verify(&id, false).await.unwrap();
        let pattern = adapter.get(&id).await.unwrap().unwrap();
        assert_eq!(pattern.verification, PatternVerification::Rejected);
    }

    #[tokio::test]
    async fn delete_removes_and_returns_pattern() {
        let adapter = PatternAdapter::new();
        let id = adapter.discover(test_pattern()).await.unwrap();

        let deleted = adapter.delete(&id).await.unwrap();
        assert!(deleted.is_some());

        let gone = adapter.get(&id).await.unwrap();
        assert!(gone.is_none());
    }

    #[tokio::test]
    async fn auto_verify_after_5_usages() {
        let adapter = PatternAdapter::new();
        let id = adapter.discover(test_pattern()).await.unwrap();

        for _ in 0..4 {
            adapter.increment_usage(&id).await.unwrap();
        }

        let pattern = adapter.get(&id).await.unwrap().unwrap();
        assert_eq!(pattern.verification, PatternVerification::Pending);

        adapter.increment_usage(&id).await.unwrap();
        let pattern = adapter.get(&id).await.unwrap().unwrap();
        assert_eq!(pattern.verification, PatternVerification::Verified);
    }
}
