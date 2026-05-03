use crate::domain::memory::TimeMetric;
use async_trait::async_trait;

/// Port for time metrics operations (inbound)
#[async_trait]
pub trait TimeMetricsPort: Send + Sync {
    /// Save a time metric record
    async fn save_time_metric(
        &self,
        metric: &TimeMetric,
        workspace_id: &str,
    ) -> Result<(), String>;
}
