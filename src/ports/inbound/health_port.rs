use async_trait::async_trait;
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct HealthStatus {
    pub status: String,
    pub lag_ms: u64,
    pub save_ok_rate: f64,
    pub match_score: f64,
    pub active_agents: u64,
    pub timestamp_ms: u64,
    pub alerts: Vec<String>,
}

#[async_trait]
pub trait HealthPort: Send + Sync {
    async fn get_health_status(&self) -> HealthStatus;
}
