use async_trait::async_trait;
use crate::adapters::inbound::http::dto::TimeMetricDto;

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

/// No-op implementation for contexts that do not persist time metrics.
pub struct NoopTimeMetricsPort;

#[async_trait]
impl TimeMetricsPort for NoopTimeMetricsPort {
    async fn save_time_metric(
        &self,
        _metric: &TimeMetricDto,
        _workspace_id: &str,
    ) -> Result<(), String> {
        Ok(())
    }
}
