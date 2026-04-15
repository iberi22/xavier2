//! Inbound port for pattern discovery and management.
//!
//! This trait defines the contract for pattern storage adapters.

use async_trait::async_trait;

use crate::domain::pattern::{PatternCategory, VerifiedPattern};

/// Port for discovering, querying, and managing verified patterns.
#[async_trait]
pub trait PatternDiscoverPort: Send + Sync {
    /// Discover and store a new pattern. Returns the pattern ID.
    async fn discover(&self, pattern: VerifiedPattern) -> anyhow::Result<String>;

    /// Query patterns by project and optional filters.
    async fn query(
        &self,
        project: &str,
        category: Option<PatternCategory>,
        min_confidence: f32,
    ) -> anyhow::Result<Vec<VerifiedPattern>>;

    /// Verify or reject a pattern by ID.
    async fn verify(&self, id: &str, verified: bool) -> anyhow::Result<()>;

    /// Get a single pattern by ID.
    async fn get(&self, id: &str) -> anyhow::Result<Option<VerifiedPattern>>;

    /// Delete a pattern by ID. Returns the deleted pattern if found.
    async fn delete(&self, id: &str) -> anyhow::Result<Option<VerifiedPattern>>;

    /// Increment the usage count for a pattern.
    /// Auto-verifies if usage >= 5 and confidence >= 0.7.
    async fn increment_usage(&self, id: &str) -> anyhow::Result<()>;
}
