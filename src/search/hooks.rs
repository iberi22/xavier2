//! Hook System for search pipeline extensibility.
//!
//! Provides a way to plug in custom logic at different stages of the search process,
//! such as query expansion, result reranking, or logging.

use super::rrf::ScoredResult;
use crate::memory::schema::MemoryQueryFilters;
use async_trait::async_trait;
use std::sync::Arc;

/// A hook that can be executed during the search lifecycle.
#[async_trait]
pub trait SearchHook: Send + Sync {
    /// Name of the hook for identification and logging.
    fn name(&self) -> &str;

    /// Called before the search query is executed.
    /// Can modify the query string and filters.
    async fn pre_query(
        &self,
        query: &mut String,
        filters: &mut Option<MemoryQueryFilters>,
    ) -> anyhow::Result<()> {
        let _ = (query, filters);
        Ok(())
    }

    /// Called after search results are obtained.
    /// Can modify or rerank the results.
    async fn post_query(&self, query: &str, results: &mut Vec<ScoredResult>) -> anyhow::Result<()> {
        let _ = (query, results);
        Ok(())
    }
}

/// Registry for managing and executing search hooks.
#[derive(Default, Clone)]
pub struct HookRegistry {
    hooks: Vec<Arc<dyn SearchHook>>,
}

impl HookRegistry {
    /// Create a new empty HookRegistry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a hook to the registry.
    pub fn add_hook(&mut self, hook: Arc<dyn SearchHook>) {
        self.hooks.push(hook);
    }

    /// Execute all pre_query hooks in order.
    pub async fn execute_pre_query(
        &self,
        query: &mut String,
        filters: &mut Option<MemoryQueryFilters>,
    ) -> anyhow::Result<()> {
        for hook in &self.hooks {
            hook.pre_query(query, filters).await?;
        }
        Ok(())
    }

    /// Execute all post_query hooks in order.
    pub async fn execute_post_query(
        &self,
        query: &str,
        results: &mut Vec<ScoredResult>,
    ) -> anyhow::Result<()> {
        for hook in &self.hooks {
            hook.post_query(query, results).await?;
        }
        Ok(())
    }

    /// Get the number of registered hooks.
    pub fn len(&self) -> usize {
        self.hooks.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.hooks.is_empty()
    }
}

impl std::fmt::Debug for HookRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HookRegistry")
            .field("hook_count", &self.hooks.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockHook {
        name: String,
    }

    #[async_trait]
    impl SearchHook for MockHook {
        fn name(&self) -> &str {
            &self.name
        }

        async fn pre_query(
            &self,
            query: &mut String,
            _filters: &mut Option<MemoryQueryFilters>,
        ) -> anyhow::Result<()> {
            query.push_str(" expanded");
            Ok(())
        }

        async fn post_query(
            &self,
            _query: &str,
            results: &mut Vec<ScoredResult>,
        ) -> anyhow::Result<()> {
            if let Some(first) = results.first_mut() {
                first.score += 1.0;
            }
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_hook_execution() {
        let mut registry = HookRegistry::new();
        registry.add_hook(Arc::new(MockHook {
            name: "test".to_string(),
        }));

        let mut query = "original".to_string();
        let mut filters = None;
        registry
            .execute_pre_query(&mut query, &mut filters)
            .await
            .expect("test assertion");
        assert_eq!(query, "original expanded");

        let mut results = vec![ScoredResult {
            id: "1".to_string(),
            content: "content".to_string(),
            score: 0.5,
            source: "test".to_string(),
            path: "path".to_string(),
            updated_at: None,
        }];
        registry
            .execute_post_query(&query, &mut results)
            .await
            .expect("test assertion");
        assert_eq!(results[0].score, 1.5);
    }
}
