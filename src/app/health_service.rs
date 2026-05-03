use crate::ports::inbound::health_port::{HealthPort, HealthStatus};
use crate::tasks::session_sync_task::get_last_sync_result;
use async_trait::async_trait;

pub struct HealthService;

impl HealthService {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HealthService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl HealthPort for HealthService {
    async fn get_health_status(&self) -> HealthStatus {
        let result = get_last_sync_result();

        HealthStatus {
            status: result.status,
            lag_ms: result.lag_ms,
            save_ok_rate: result.save_ok_rate,
            match_score: result.match_score,
            active_agents: result.active_agents,
            timestamp_ms: result.timestamp_ms,
            alerts: result.alerts,
        }
    }
}
