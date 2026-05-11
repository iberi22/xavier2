use crate::domain::security::ImpactReport;
use async_trait::async_trait;

#[async_trait]
pub trait ChangeControlPort: Send + Sync {
    /// Calculate the impact of changes to a set of file patterns
    async fn calculate_impact(&self, patterns: &[String]) -> anyhow::Result<ImpactReport>;
}
