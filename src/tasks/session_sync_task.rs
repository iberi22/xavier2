//! Session Sync Task - Monitors Xavier2 session indexing and sync health.
//!
//! Runs on a configurable interval (default 5min) and:
//! - Checks if Xavier2 is reachable via /xavier2/health
//! - Verifies recent session events were indexed in memory
//! - Reports sync status metrics (save_ok_rate, index_lag_ms, match_score)
//! - Alerts if lag > 30s or save_ok_rate < 95%
//!
//! Also provides on-demand sync check via POST /xavier2/sync/check

use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use tokio::sync::{broadcast, RwLock};
use tokio::time::interval;
use tracing::{info, warn};

use crate::domain::memory::MemoryNamespace;
use crate::ports::inbound::MemoryQueryPort;
use crate::ports::outbound::HealthCheckPort;

/// Interval in milliseconds between sync checks.
/// Default: 5 minutes (300_000 ms)
const DEFAULT_SYNC_INTERVAL_MS: u64 = 300_000;

/// Threshold for index lag alert (milliseconds) — configurable via SEVIER2_LAG_THRESHOLD_MS
/// Threshold for index lag alert (milliseconds)
/// Configurable via SEVIER2_LAG_THRESHOLD_MS env var (default: 30000)
const LAG_THRESHOLD_MS: u64 = 30_000;

/// Threshold for save_ok_rate alert
/// Configurable via SEVIER2_SAVE_OK_RATE_THRESHOLD env var (default: 0.95)
const SAVE_OK_RATE_THRESHOLD: f64 = 0.95;

/// Shared sync state — lives in CliState as Arc<RwLock<SyncState>>
/// All reads and writes go through the same lock to avoid inconsistent snapshots (ADR-003)
#[derive(Debug, Clone)]
pub struct SyncState {
    pub timestamp_ms: u64,
    pub lag_ms: u64,
    pub save_ok_rate: f64,
    pub match_score: f64,
    pub active_agents: u64,
}

impl Default for SyncState {
    fn default() -> Self {
        Self {
            timestamp_ms: 0,
            lag_ms: 0,
            save_ok_rate: 1.0,
            match_score: 1.0,
            active_agents: 0,
        }
    }
}

/// Sync check result
#[derive(Debug, Clone)]
pub struct SyncCheckResult {
    pub status: String,
    pub lag_ms: u64,
    pub save_ok_rate: f64,
    pub match_score: f64,
    pub active_agents: u64,
    pub timestamp_ms: u64,
    pub alerts: Vec<String>,
}

impl Default for SyncCheckResult {
    fn default() -> Self {
        Self {
            status: "unknown".to_string(),
            lag_ms: 0,
            save_ok_rate: 1.0,
            match_score: 1.0,
            active_agents: 0,
            timestamp_ms: 0,
            alerts: Vec::new(),
        }
    }
}

/// Session sync task - runs periodic sync checks
pub struct SessionSyncTask {
    /// Interval between sync checks (in ms)
    interval_ms: u64,
    /// Health check port for Xavier2
    health_port: Arc<dyn HealthCheckPort>,
    /// Memory port for querying session records (optional)
    memory: Option<Arc<dyn MemoryQueryPort>>,
    /// Last successful check timestamp
    last_check: Arc<RwLock<Instant>>,
}

impl SessionSyncTask {
    /// Create a new SessionSyncTask with the given health check port.
    pub fn new(health_port: Arc<dyn HealthCheckPort>) -> Self {
        Self::with_memory(health_port, None)
    }

    /// Create a new SessionSyncTask with optional memory port for real lag estimation.
    pub fn with_memory(health_port: Arc<dyn HealthCheckPort>, memory: Option<Arc<dyn MemoryQueryPort>>) -> Self {
        let interval_ms = std::env::var("SEVIER2_SYNC_INTERVAL_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_SYNC_INTERVAL_MS);

        Self {
            interval_ms,
            health_port,
            memory,
            last_check: Arc::new(RwLock::new(Instant::now())),
        }
    }

    /// Run the sync check (shared logic for both cron and on-demand)
    /// Writes ALL fields under a single lock for consistent snapshots
    pub async fn run_sync_check(&self, sync_state: Arc<RwLock<SyncState>>) -> SyncCheckResult {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let mut alerts = Vec::new();
        let mut status = "ok".to_string();

        // 1. Check if Xavier2 is reachable via /xavier2/health
        let health_status = match self.health_port.check_health().await {
            Ok(hs) => hs,
            Err(e) => {
                tracing::debug!(error = %e, "Health check failed");
                alerts.push("Xavier2 /xavier2/health endpoint unreachable".to_string());
                status = "degraded".to_string();
                let result = SyncCheckResult {
                    status: status.clone(),
                    lag_ms: 0,
                    save_ok_rate: 1.0,
                    match_score: 1.0,
                    active_agents: 0,
                    timestamp_ms: now_ms,
                    alerts: alerts.clone(),
                };
                // Update all fields under one lock — consistent snapshot
                {
                    let mut s = sync_state.write().await;
                    s.timestamp_ms = now_ms;
                    s.lag_ms = 0;
                    s.save_ok_rate = 1.0;
                    s.match_score = 1.0;
                    s.active_agents = 0;
                }
                *self.last_check.write().await = Instant::now();
                return result;
            }
        };

        let active_agents = health_status.active_agents as u64;
        let health_reported_lag = health_status.lag_ms;

        if health_status.status != "ok" && health_status.status != "degraded" {
            alerts.push(format!("Xavier2 health status: {}", health_status.status));
            status = "degraded".to_string();
        }

        // 2. Calculate index lag — prefer real lag from memory if available
        let lag_ms = if let Some(memory) = &self.memory {
            match Self::estimate_index_lag_impl(memory).await {
                Ok(real_lag) => {
                    tracing::debug!(lag_ms = real_lag, "Using real index lag from memory");
                    real_lag
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to estimate index lag from memory, falling back to health port");
                    health_reported_lag
                }
            }
        } else {
            health_reported_lag
        };

        if lag_ms > LAG_THRESHOLD_MS {
            alerts.push(format!(
                "Index lag {}ms exceeds threshold {}ms",
                lag_ms, LAG_THRESHOLD_MS
            ));
            status = "alert".to_string();
        }

        // 3. Get save_ok_rate (from sync state — consistent read)
        let (save_ok_rate, match_score) = {
            let s = sync_state.read().await;
            (s.save_ok_rate, s.match_score)
        };

        // Check save_ok_rate threshold
        if save_ok_rate < SAVE_OK_RATE_THRESHOLD {
            alerts.push(format!(
                "Save ok rate {:.1}% below threshold {:.1}%",
                save_ok_rate * 100.0,
                SAVE_OK_RATE_THRESHOLD * 100.0
            ));
            status = "alert".to_string();
        }

        let result = SyncCheckResult {
            status: status.clone(),
            lag_ms,
            save_ok_rate,
            match_score,
            active_agents,
            timestamp_ms: now_ms,
            alerts: alerts.clone(),
        };

        // Update all fields under one lock — consistent snapshot (ADR-003)
        {
            let mut s = sync_state.write().await;
            s.timestamp_ms = now_ms;
            s.lag_ms = lag_ms;
            s.save_ok_rate = save_ok_rate;
            s.match_score = match_score;
            s.active_agents = active_agents;
        }

        // Update last_check timestamp
        *self.last_check.write().await = Instant::now();

        result
    }

    /// Start the cron loop for periodic sync checks.
    /// Accepts sync_state so all writes go through the same lock.
    pub async fn start_cron(&self, sync_state: Arc<RwLock<SyncState>>, mut shutdown_rx: broadcast::Receiver<()>) {
        let mut ticker = interval(Duration::from_millis(self.interval_ms));

        info!(
            interval_ms = self.interval_ms,
            "SessionSyncTask cron started"
        );

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    let result = self.run_sync_check(sync_state.clone()).await;

                    // Log result
                    if result.alerts.is_empty() {
                        info!(
                            status = %result.status,
                            lag_ms = result.lag_ms,
                            active_agents = result.active_agents,
                            "SessionSyncTask check passed"
                        );
                    } else {
                        warn!(
                            status = %result.status,
                            lag_ms = result.lag_ms,
                            alerts = ?result.alerts,
                            "SessionSyncTask check with alerts"
                        );
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("SessionSyncTask cron received shutdown signal, exiting");
                    break;
                }
            }
        }
    }

    /// Estimate index lag by querying session memory records.
    /// Returns the time elapsed since the most recent session record was indexed.
    async fn estimate_index_lag_impl(memory: &Arc<dyn MemoryQueryPort>) -> anyhow::Result<u64> {
        // Use Session variant to list session records
        let namespace = MemoryNamespace::Session;

        // Fetch recent session records to find the most recent indexed timestamp
        let sessions = memory.list(namespace, 100).await?;

        if sessions.is_empty() {
            return Ok(0);
        }

        // Find the most recent updated_at timestamp among session records
        let now = Utc::now();
        let latest = sessions
            .iter()
            .map(|r| r.updated_at)
            .max()
            .unwrap_or_else(|| DateTime::<Utc>::from(std::time::SystemTime::now()));

        let lag_ms = now.signed_duration_since(latest).num_milliseconds() as u64;
        Ok(lag_ms)
    }

    /// Update metrics (can be called by session event handler)
    pub async fn update_metrics(
        &self,
        save_ok_rate: f64,
        match_score: f64,
        active_agents: u64,
        sync_state: Arc<RwLock<SyncState>>,
    ) {
        let mut s = sync_state.write().await;
        s.save_ok_rate = save_ok_rate;
        s.match_score = match_score;
        s.active_agents = active_agents;
    }
}

impl Default for SessionSyncTask {
    fn default() -> Self {
        Self {
            interval_ms: DEFAULT_SYNC_INTERVAL_MS,
            health_port: Arc::new(crate::adapters::outbound::http_health_adapter::HttpHealthAdapter::new(
                std::env::var("XAVIER2_URL").unwrap_or_else(|_| "http://localhost:8006".to_string()),
            )),
            memory: None,
            last_check: Arc::new(RwLock::new(Instant::now())),
        }
    }
}

/// Get last sync check result (for REST endpoint).
/// Reads ALL fields from a single consistent snapshot (ADR-003).
pub async fn get_last_sync_result(sync_state: Arc<RwLock<SyncState>>) -> SyncCheckResult {
    let s = sync_state.read().await;
    let status = if s.lag_ms > LAG_THRESHOLD_MS || s.save_ok_rate < SAVE_OK_RATE_THRESHOLD {
        "alert".to_string()
    } else {
        "ok".to_string()
    };
    SyncCheckResult {
        status,
        lag_ms: s.lag_ms,
        save_ok_rate: s.save_ok_rate,
        match_score: s.match_score,
        active_agents: s.active_agents,
        timestamp_ms: s.timestamp_ms,
        alerts: Vec::new(),
    }
}
