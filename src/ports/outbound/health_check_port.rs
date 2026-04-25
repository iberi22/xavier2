use async_trait::async_trait;

/// Health status returned by health check
#[derive(Debug, Clone)]
pub struct HealthStatus {
    /// Overall status: "ok", "degraded", "alert"
    pub status: String,
    /// Index lag in milliseconds
    pub lag_ms: u64,
    /// Number of currently active agents
    pub active_agents: usize,
}

/// Outbound port for checking Xavier2 health.
/// Abstracts the HTTP call to /xavier2/health so SessionSyncTask
/// doesn't depend on reqwest directly.
#[async_trait]
pub trait HealthCheckPort: Send + Sync {
    /// Check Xavier2 health endpoint and return status.
    async fn check_health(&self) -> anyhow::Result<HealthStatus>;
}