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
use tokio::time::{interval, sleep};
use tracing::{info, warn};

use crate::domain::memory::MemoryNamespace;
use crate::ports::outbound::{HealthCheckPort, StoragePort};

/// Interval in milliseconds between sync checks.
/// Default: 5 minutes (300_000 ms)
const DEFAULT_SYNC_INTERVAL_MS: u64 = 300_000;

/// Threshold for index lag alert (milliseconds)
/// Default: 30 seconds (30_000 ms)
const DEFAULT_LAG_THRESHOLD_MS: u64 = 30_000;

/// Threshold for save_ok_rate alert (percentage, expressed as 0.0-1.0)
/// Default: 0.95 (95%)
const DEFAULT_SAVE_OK_RATE_THRESHOLD: f64 = 0.95;

/// Max health check retries before marking the sync check degraded.
const DEFAULT_SYNC_MAX_RETRIES: u32 = 3;

/// Delay between health check retries in milliseconds.
const DEFAULT_SYNC_RETRY_DELAY_MS: u64 = 1_000;

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
    /// Storage port for querying memory records (optional, falls back if None)
    storage_port: Option<Arc<dyn StoragePort>>,
    /// Last successful check timestamp
    last_check: Arc<RwLock<Instant>>,
    /// Lag threshold in ms (configurable via XAVIER2_SYNC_LAG_THRESHOLD_MS)
    lag_threshold_ms: u64,
    /// Save ok rate threshold (configurable via XAVIER2_SYNC_SAVE_OK_RATE_THRESHOLD)
    save_ok_rate_threshold: f64,
    /// Max health check retries (configurable via XAVIER2_SYNC_MAX_RETRIES)
    max_retries: u32,
    /// Delay between health check retries (configurable via XAVIER2_SYNC_RETRY_DELAY_MS)
    retry_delay_ms: u64,
}

impl SessionSyncTask {
    /// Create a new SessionSyncTask with the given health check port.
    pub fn new(health_port: Arc<dyn HealthCheckPort>) -> Self {
        Self::with_storage(health_port, None)
    }

    /// Create a new SessionSyncTask with the given health and optional storage port.
    pub fn with_storage(
        health_port: Arc<dyn HealthCheckPort>,
        storage_port: Option<Arc<dyn StoragePort>>,
    ) -> Self {
        let interval_ms =
            read_env_or_legacy("XAVIER2_SYNC_INTERVAL_MS", "SEVIER2_SYNC_INTERVAL_MS")
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_SYNC_INTERVAL_MS);

        let lag_threshold_ms =
            read_env_or_legacy("XAVIER2_SYNC_LAG_THRESHOLD_MS", "SEVIER2_LAG_THRESHOLD_MS")
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_LAG_THRESHOLD_MS);

        let save_ok_rate_threshold = read_env_or_legacy(
            "XAVIER2_SYNC_SAVE_OK_RATE_THRESHOLD",
            "SEVIER2_SAVE_OK_RATE_THRESHOLD",
        )
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_SAVE_OK_RATE_THRESHOLD);

        let max_retries =
            read_env_or_legacy("XAVIER2_SYNC_MAX_RETRIES", "SEVIER2_SYNC_MAX_RETRIES")
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_SYNC_MAX_RETRIES);

        let retry_delay_ms =
            read_env_or_legacy("XAVIER2_SYNC_RETRY_DELAY_MS", "SEVIER2_SYNC_RETRY_DELAY_MS")
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_SYNC_RETRY_DELAY_MS);

        Self {
            interval_ms,
            health_port,
            storage_port,
            last_check: Arc::new(RwLock::new(Instant::now())),
            lag_threshold_ms,
            save_ok_rate_threshold,
            max_retries,
            retry_delay_ms,
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
        let mut health_status = None;
        let mut last_error = None;
        for attempt in 0..=self.max_retries {
            match self.health_port.check_health().await {
                Ok(hs) => {
                    health_status = Some(hs);
                    break;
                }
                Err(e) => {
                    tracing::debug!(
                        error = %e,
                        attempt = attempt + 1,
                        max_attempts = self.max_retries + 1,
                        "Health check failed"
                    );
                    last_error = Some(e.to_string());
                    if attempt < self.max_retries {
                        sleep(Duration::from_millis(self.retry_delay_ms)).await;
                    }
                }
            }
        }

        let health_status = match health_status {
            Some(hs) => hs,
            None => {
                if let Some(error) = last_error {
                    tracing::debug!(error = %error, "Health check retries exhausted");
                }
                alerts.push(format!(
                    "Xavier2 /xavier2/health endpoint unreachable after {} attempts",
                    self.max_retries + 1
                ));
                status = "degraded".to_string();
                let result = SyncCheckResult {
                    status,
                    lag_ms: 0,
                    save_ok_rate: 1.0,
                    match_score: 1.0,
                    active_agents: 0,
                    timestamp_ms: now_ms,
                    alerts,
                };
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

        // 2. Calculate index lag from storage (actual session record timestamps)
        let lag_ms = self.estimate_index_lag().await;

        if health_status.status != "ok" && health_status.status != "degraded" {
            alerts.push(format!("Xavier2 health status: {}", health_status.status));
            status = "degraded".to_string();
        }

        // 3. Check lag threshold
        if lag_ms > self.lag_threshold_ms {
            alerts.push(format!(
                "Index lag {}ms exceeds threshold {}ms",
                lag_ms, self.lag_threshold_ms
            ));
            status = "alert".to_string();
        }

        // 4. Get save_ok_rate (from stored metrics)
        let save_ok_rate = self.get_save_ok_rate().await;

        // 5. Get match_score (from stored metrics)
        let match_score = self.get_match_score().await;

        // Check save_ok_rate threshold
        if save_ok_rate < self.save_ok_rate_threshold {
            alerts.push(format!(
                "Save ok rate {:.1}% below threshold {:.1}%",
                save_ok_rate * 100.0,
                self.save_ok_rate_threshold * 100.0
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

    /// Estimate index lag by querying session memory records and comparing
    /// the most recent record's updated_at timestamp with the current time.
    /// Falls back to time-since-last-check if storage is unavailable.
    async fn estimate_index_lag(&self) -> u64 {
        use std::time::SystemTime;

        // Try to query session records from storage
        if let Some(ref storage) = self.storage_port {
            let records = match storage.list("session", 1000).await {
                Ok(recs) => recs,
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to list session records for lag estimation");
                    return self.fallback_lag().await;
                }
            };

            let now = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            if let Some(lag) = records
                .iter()
                .filter(|r| r.namespace == MemoryNamespace::Session)
                .map(|r| r.updated_at.timestamp_millis() as u64)
                .max()
                .map(|last_updated| now.saturating_sub(last_updated))
            {
                return lag.min(60_000); // cap at 60s
            }
        }

        self.fallback_lag().await
    }

    /// Fallback lag estimation using time since last successful check.
    async fn fallback_lag(&self) -> u64 {
        let last = *self.last_check.read().await;
        last.elapsed().as_millis() as u64
    }

    /// Get save_ok_rate from stored metrics
    async fn get_save_ok_rate(&self) -> f64 {
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
        Self {
            interval_ms: read_env_or_legacy("XAVIER2_SYNC_INTERVAL_MS", "SEVIER2_SYNC_INTERVAL_MS")
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_SYNC_INTERVAL_MS),
            lag_threshold_ms: read_env_or_legacy(
                "XAVIER2_SYNC_LAG_THRESHOLD_MS",
                "SEVIER2_LAG_THRESHOLD_MS",
            )
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_LAG_THRESHOLD_MS),
            save_ok_rate_threshold: read_env_or_legacy(
                "XAVIER2_SYNC_SAVE_OK_RATE_THRESHOLD",
                "SEVIER2_SAVE_OK_RATE_THRESHOLD",
            )
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_SAVE_OK_RATE_THRESHOLD),
            max_retries: read_env_or_legacy("XAVIER2_SYNC_MAX_RETRIES", "SEVIER2_SYNC_MAX_RETRIES")
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_SYNC_MAX_RETRIES),
            retry_delay_ms: read_env_or_legacy(
                "XAVIER2_SYNC_RETRY_DELAY_MS",
                "SEVIER2_SYNC_RETRY_DELAY_MS",
            )
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_SYNC_RETRY_DELAY_MS),
            health_port: Arc::new(
                crate::adapters::outbound::http_health_adapter::HttpHealthAdapter::new(
                    std::env::var("XAVIER2_URL")
                        .unwrap_or_else(|_| "http://localhost:8006".to_string()),
                ),
            ),
            storage_port: None,
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

fn read_env_or_legacy(primary: &str, legacy: &str) -> Option<String> {
    std::env::var(primary)
        .ok()
        .or_else(|| std::env::var(legacy).ok())
}
