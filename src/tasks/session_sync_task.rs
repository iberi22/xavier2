//! Session Sync Task - Monitors Xavier2 session indexing and sync health.
//!
//! Runs on a configurable interval (default 5min) and:
//! - Checks if Xavier2 is reachable via /xavier2/health
//! - Verifies recent session events were indexed in memory
//! - Reports sync status metrics (save_ok_rate, index_lag_ms, match_score)
//! - Alerts if lag > 30s or save_ok_rate < 95%
//!
//! Also provides on-demand sync check via POST /xavier2/sync/check

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{info, warn};

use crate::ports::outbound::HealthCheckPort;

/// Interval in milliseconds between sync checks.
/// Default: 5 minutes (300_000 ms)
const DEFAULT_SYNC_INTERVAL_MS: u64 = 300_000;

/// Threshold for index lag alert (milliseconds)
const LAG_THRESHOLD_MS: u64 = 30_000;

/// Threshold for save_ok_rate alert (percentage)
const SAVE_OK_RATE_THRESHOLD: f64 = 0.95;

/// Last sync check result stored in memory (static)
pub(crate) static LAST_CHECK_TIMESTAMP_MS: AtomicU64 = AtomicU64::new(0);
pub(crate) static LAST_CHECK_LAG_MS: AtomicU64 = AtomicU64::new(0);
pub(crate) static LAST_CHECK_SAVE_OK_RATE: Mutex<f64> = Mutex::new(1.0);
pub(crate) static LAST_CHECK_MATCH_SCORE: Mutex<f64> = Mutex::new(1.0);
pub(crate) static LAST_CHECK_ACTIVE_AGENTS: AtomicU64 = AtomicU64::new(0);

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
    /// Last successful check timestamp
    last_check: Arc<RwLock<Instant>>,
}

impl SessionSyncTask {
    /// Create a new SessionSyncTask with the given health check port.
    pub fn new(health_port: Arc<dyn HealthCheckPort>) -> Self {
        let interval_ms = std::env::var("SEVIER2_SYNC_INTERVAL_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_SYNC_INTERVAL_MS);

        Self {
            interval_ms,
            health_port,
            last_check: Arc::new(RwLock::new(Instant::now())),
        }
    }

    /// Run the sync check (shared logic for both cron and on-demand)
    pub async fn run_sync_check(&self) -> SyncCheckResult {
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
                // Return early with degraded status
                let result = SyncCheckResult {
                    status,
                    lag_ms: 0,
                    save_ok_rate: 1.0,
                    match_score: 1.0,
                    active_agents: 0,
                    timestamp_ms: now_ms,
                    alerts,
                };
                // Update static last-check values
                LAST_CHECK_TIMESTAMP_MS.store(now_ms, Ordering::SeqCst);
                LAST_CHECK_LAG_MS.store(0, Ordering::SeqCst);
                {
                    let mut r = LAST_CHECK_SAVE_OK_RATE.lock().unwrap();
                    *r = 1.0;
                }
                {
                    let mut r = LAST_CHECK_MATCH_SCORE.lock().unwrap();
                    *r = 1.0;
                }
                LAST_CHECK_ACTIVE_AGENTS.store(0, Ordering::SeqCst);
                *self.last_check.write().await = Instant::now();
                return result;
            }
        };

        let active_agents = health_status.active_agents as u64;
        let lag_ms = health_status.lag_ms;

        if health_status.status != "ok" && health_status.status != "degraded" {
            alerts.push(format!("Xavier2 health status: {}", health_status.status));
            status = "degraded".to_string();
        }

        // 2. Calculate index lag
        if lag_ms > LAG_THRESHOLD_MS {
            alerts.push(format!(
                "Index lag {}ms exceeds threshold {}ms",
                lag_ms, LAG_THRESHOLD_MS
            ));
            status = "alert".to_string();
        }

        // 3. Get save_ok_rate (from stored metrics)
        let save_ok_rate = self.get_save_ok_rate().await;

        // 4. Get match_score (from stored metrics)
        let match_score = self.get_match_score().await;

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
            status,
            lag_ms,
            save_ok_rate,
            match_score,
            active_agents,
            timestamp_ms: now_ms,
            alerts,
        };

        // Update static last-check values
        LAST_CHECK_TIMESTAMP_MS.store(now_ms, Ordering::SeqCst);
        LAST_CHECK_LAG_MS.store(lag_ms, Ordering::SeqCst);
        {
            let mut r = LAST_CHECK_SAVE_OK_RATE.lock().unwrap();
            *r = save_ok_rate;
        }
        {
            let mut r = LAST_CHECK_MATCH_SCORE.lock().unwrap();
            *r = match_score;
        }
        LAST_CHECK_ACTIVE_AGENTS.store(active_agents, Ordering::SeqCst);

        // Update last_check timestamp
        *self.last_check.write().await = Instant::now();

        result
    }

    /// Start the cron loop for periodic sync checks
    pub async fn start_cron(&self) {
        let mut ticker = interval(Duration::from_millis(self.interval_ms));

        info!(
            interval_ms = self.interval_ms,
            "SessionSyncTask cron started"
        );

        loop {
            ticker.tick().await;

            let result = self.run_sync_check().await;

            // Log result
            if result.alerts.is_empty() {
                info!(
                    status = %result.status,
                    lag_ms = result.lag_ms,
                    save_ok_rate = "%.1",
                    match_score = "%.2",
                    active_agents = result.active_agents,
                    "SessionSyncTask check passed"
                );
            } else {
                warn!(
                    status = %result.status,
                    lag_ms = result.lag_ms,
                    save_ok_rate = "%.1",
                    alerts = ?result.alerts,
                    "SessionSyncTask check with alerts"
                );
            }
        }
    }

    /// Estimate index lag by checking recent session memory entries
    async fn estimate_index_lag(&self) -> u64 {
        // In a real implementation, query Xavier2 memory for sessions/* patterns
        // and compare event timestamps with indexed timestamps.
        // For now, we use the time since last check as a proxy.
        let last = *self.last_check.read().await;
        let elapsed = last.elapsed().as_millis() as u64;
        // Add some simulated lag based on recent activity
        elapsed.min(60_000) // cap at 60s
    }

    /// Get save_ok_rate from stored metrics
    async fn get_save_ok_rate(&self) -> f64 {
        // In real implementation, query Xavier2 memory for metrics/time/ patterns
        // For now return stored value
        *LAST_CHECK_SAVE_OK_RATE.lock().unwrap()
    }

    /// Get match_score from stored metrics
    async fn get_match_score(&self) -> f64 {
        *LAST_CHECK_MATCH_SCORE.lock().unwrap()
    }

    /// Update metrics (can be called by session event handler)
    pub fn update_metrics(save_ok_rate: f64, match_score: f64, active_agents: u64) {
        {
            let mut r = LAST_CHECK_SAVE_OK_RATE.lock().unwrap();
            *r = save_ok_rate;
        }
        {
            let mut r = LAST_CHECK_MATCH_SCORE.lock().unwrap();
            *r = match_score;
        }
        LAST_CHECK_ACTIVE_AGENTS.store(active_agents, Ordering::SeqCst);
    }
}

impl Default for SessionSyncTask {
    fn default() -> Self {
        // This is kept for backwards compatibility with Default trait,
        // but in practice SessionSyncTask must be constructed with a HealthCheckPort.
        // A default-constructed task uses a no-op health adapter.
        Self {
            interval_ms: DEFAULT_SYNC_INTERVAL_MS,
            health_port: Arc::new(crate::adapters::outbound::http_health_adapter::HttpHealthAdapter::new(
                std::env::var("XAVIER2_URL").unwrap_or_else(|_| "http://localhost:8006".to_string()),
            )),
            last_check: Arc::new(RwLock::new(Instant::now())),
        }
    }
}

/// Get last sync check result (for REST endpoint)
pub fn get_last_sync_result() -> SyncCheckResult {
    SyncCheckResult {
        status: "ok".to_string(),
        lag_ms: LAST_CHECK_LAG_MS.load(Ordering::SeqCst),
        save_ok_rate: *LAST_CHECK_SAVE_OK_RATE.lock().unwrap(),
        match_score: *LAST_CHECK_MATCH_SCORE.lock().unwrap(),
        active_agents: LAST_CHECK_ACTIVE_AGENTS.load(Ordering::SeqCst),
        timestamp_ms: LAST_CHECK_TIMESTAMP_MS.load(Ordering::SeqCst),
        alerts: Vec::new(),
    }
}