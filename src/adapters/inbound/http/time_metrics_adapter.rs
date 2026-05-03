use async_trait::async_trait;
use std::sync::Arc;

use crate::domain::memory::TimeMetric;
use crate::ports::inbound::TimeMetricsPort;
use crate::time::TimeMetricsStore;

/// Inbound adapter that wraps TimeMetricsStore and implements TimeMetricsPort
pub struct TimeMetricsAdapter {
    store: Arc<TimeMetricsStore>,
}

impl TimeMetricsAdapter {
    pub fn new(store: Arc<TimeMetricsStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl TimeMetricsPort for TimeMetricsAdapter {
    async fn save_time_metric(
        &self,
        metric: &TimeMetric,
        workspace_id: &str,
    ) -> Result<(), String> {
        let dto = metric.clone().into();
        self.store.save_time_metric(&dto, workspace_id).await
    }
}
