use crate::adapters::inbound::http::dto::TimeMetricDto;
use async_trait::async_trait;

/// Port for time metrics operations (inbound)
#[async_trait]
pub trait TimeMetricsPort: Send + Sync {
    /// Save a time metric record
    async fn save_time_metric(
        &self,
        metric: &TimeMetricDto,
        workspace_id: &str,
    ) -> Result<(), String>;
}
