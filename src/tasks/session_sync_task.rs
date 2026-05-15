//! Session Sync Task - Monitors Xavier session indexing and sync health.
//!
//! Runs on a configurable interval (default 5min) and:
//! - Checks if Xavier is reachable via /xavier/health
//! - Verifies recent session events were indexed in memory
//! - Reports sync status metrics (save_ok_rate, index_lag_ms, match_score)
//! - Alerts if lag > 30s or save_ok_rate < 95%
//!
//! Also provides on-demand sync check via POST /xavier/sync/check

use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock as StdRwLock};
use std::time::{Duration, Instant};

use tokio::sync::{watch, RwLock as TokioRwLock};
use tokio::time::{interval, sleep, timeout};
use tracing::{info, warn};

use crate::memory::schema::{MemoryKind, MemoryQueryFilters};
use crate::memory::store::MemoryStore;
use crate::ports::outbound::HealthCheckPort;

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

/// Minimum interval between health check attempts in milliseconds.
const DEFAULT_SYNC_MIN_HEALTH_INTERVAL_MS: u64 = 1_000;

/// Timeout for each health check attempt in milliseconds.
const DEFAULT_SYNC_TIMEOUT_MS: u64 = 5_000;

/// Last sync check result stored in memory (static)
/// Unified last sync check result (single lock — no data race).
pub(crate) static LAST_CHECK_RESULT: Lazy<StdRwLock<SyncCheckResult>> =
    Lazy::new(|| StdRwLock::new(SyncCheckResult::default()));
static SYNC_CRON_STARTED: AtomicBool = AtomicBool::new(false);

/// Handle used by the HTTP server to request a graceful cron shutdown.
pub struct SessionSyncShutdown {
    shutdown_tx: watch::Sender<bool>,
    join_handle: Option<tokio::task::JoinHandle<()>>,
}

impl SessionSyncShutdown {
    /// Send the shutdown signal to the cron loop.
    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(true);
    }

    /// Wait for the spawned cron task to finish, with a timeout.
    /// If the task does not complete within the timeout, a warning is logged
    /// and this method returns (forced shutdown).
    pub async fn wait_for_shutdown(self, timeout_dur: Duration) {
        if let Some(handle) = self.join_handle {
            tokio::select! {
                _ = handle => {
                    tracing::info!("SessionSyncTask cron exited cleanly");
                }
                _ = tokio::time::sleep(timeout_dur) => {
                    tracing::warn!(
                        timeout_ms = timeout_dur.as_millis() as u64,
                        "SessionSyncTask shutdown timed out, forcing"
                    );
                }
            }
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
    /// Health check port for Xavier
    health_port: Arc<dyn HealthCheckPort>,
    /// Memory store for querying memory records (optional, falls back if None)
    memory_store: Option<Arc<dyn MemoryStore>>,
    /// Last successful check timestamp
    last_check: Arc<TokioRwLock<Instant>>,
    /// Lag threshold in ms (configurable via XAVIER_SYNC_LAG_THRESHOLD_MS or SEVIER_LAG_THRESHOLD_MS)
    lag_threshold_ms: u64,
    /// Save ok rate threshold (configurable via XAVIER_SYNC_SAVE_OK_RATE_THRESHOLD or SEVIER_SAVE_OK_RATE_THRESHOLD)
    save_ok_rate_threshold: f64,
    /// Max health check retries (configurable via XAVIER_SYNC_MAX_RETRIES)
    max_retries: u32,
    /// Minimum interval between health check attempts (configurable via XAVIER_SYNC_MIN_HEALTH_INTERVAL_MS)
    min_health_interval_ms: u64,
    /// Timeout per health check attempt (configurable via XAVIER_SYNC_TIMEOUT_MS)
    timeout_ms: u64,
    /// Shutdown signal shared with the running cron loop.
    shutdown_tx: watch::Sender<bool>,
}

impl SessionSyncTask {
    /// Create a new SessionSyncTask with the given health check port.
    pub fn new(health_port: Arc<dyn HealthCheckPort>) -> Self {
        Self::with_storage(health_port, None)
    }

    /// Create a new SessionSyncTask with the given health and optional memory store.
    pub fn with_storage(
        health_port: Arc<dyn HealthCheckPort>,
        memory_store: Option<Arc<dyn MemoryStore>>,
    ) -> Self {
        let interval_ms = read_env_or_legacy("XAVIER_SYNC_INTERVAL_MS", "SEVIER_SYNC_INTERVAL_MS")
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_SYNC_INTERVAL_MS);

        let lag_threshold_ms =
            read_env_or_legacy("XAVIER_SYNC_LAG_THRESHOLD_MS", "SEVIER_LAG_THRESHOLD_MS")
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_LAG_THRESHOLD_MS);

        let save_ok_rate_threshold = read_env_or_legacy(
            "XAVIER_SYNC_SAVE_OK_RATE_THRESHOLD",
            "SEVIER_SAVE_OK_RATE_THRESHOLD",
        )
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_SAVE_OK_RATE_THRESHOLD);

        let max_retries = read_env_or_legacy("XAVIER_SYNC_MAX_RETRIES", "SEVIER_SYNC_MAX_RETRIES")
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_SYNC_MAX_RETRIES);

        let min_health_interval_ms = read_env_or_legacy(
            "XAVIER_SYNC_MIN_HEALTH_INTERVAL_MS",
            "SEVIER_SYNC_MIN_HEALTH_INTERVAL_MS",
        )
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_SYNC_MIN_HEALTH_INTERVAL_MS);

        let timeout_ms = read_env_or_legacy("XAVIER_SYNC_TIMEOUT_MS", "SEVIER_SYNC_TIMEOUT_MS")
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_SYNC_TIMEOUT_MS);

        let (shutdown_tx, _) = watch::channel(false);

        Self {
            interval_ms,
            health_port,
            memory_store,
            last_check: Arc::new(TokioRwLock::new(Instant::now())),
            lag_threshold_ms,
            save_ok_rate_threshold,
            max_retries,
            min_health_interval_ms,
            timeout_ms,
            shutdown_tx,
        }
    }

    /// Spawn the cron loop at most once per process.
    /// Returns a shutdown handle when the task was spawned by this call.
    /// The caller can use [`SessionSyncShutdown::wait_for_shutdown`] to await
    /// a clean exit with a timeout.
    pub fn spawn_cron_once(self) -> Option<SessionSyncShutdown> {
        if SYNC_CRON_STARTED
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return None;
        }

        let shutdown_tx = self.shutdown_tx.clone();
        let handle = tokio::spawn(async move {
            self.start_cron().await;
            SYNC_CRON_STARTED.store(false, Ordering::SeqCst);
        });

        Some(SessionSyncShutdown {
            shutdown_tx,
            join_handle: Some(handle),
        })
    }

    /// Create a shutdown handle **without** a join handle (detached).
    /// Used when the caller only wants to signal shutdown but cannot await.
    pub fn shutdown_handle(&self) -> SessionSyncShutdown {
        SessionSyncShutdown {
            shutdown_tx: self.shutdown_tx.clone(),
            join_handle: None,
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

        // 1. Check if Xavier is reachable via /xavier/health
        let mut health_status = None;
        let mut last_error = None;
        for attempt in 0..=self.max_retries {
            match timeout(
                Duration::from_millis(self.timeout_ms),
                self.health_port.check_health(),
            )
            .await
            {
                Ok(Ok(hs)) => {
                    health_status = Some(hs);
                    break;
                }
                Ok(Err(e)) => {
                    tracing::debug!(
                        error = %e,
                        attempt = attempt + 1,
                        max_attempts = self.max_retries + 1,
                        "Health check failed"
                    );
                    last_error = Some(e.to_string());
                }
                Err(_) => {
                    tracing::debug!(
                        timeout_ms = self.timeout_ms,
                        attempt = attempt + 1,
                        max_attempts = self.max_retries + 1,
                        "Health check timed out"
                    );
                    last_error = Some(format!(
                        "health check timed out after {}ms",
                        self.timeout_ms
                    ));
                }
            }

            if attempt < self.max_retries {
                sleep(Duration::from_millis(self.min_health_interval_ms)).await;
            }
        }

        let health_status = match health_status {
            Some(hs) => hs,
            None => {
                if let Some(error) = last_error {
                    tracing::debug!(error = %error, "Health check retries exhausted");
                }
                alerts.push(format!(
                    "Xavier /xavier/health endpoint unreachable after {} attempts",
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
                {
                    if let Ok(mut r) = LAST_CHECK_RESULT.write() {
                        *r = result.clone();
                    }
                }
                *self.last_check.write().await = Instant::now();
                return result;
            }
        };

        let active_agents = health_status.active_agents as u64;

        // 2. Calculate index lag from storage (actual session record timestamps)
        let lag_ms = self.estimate_index_lag().await;

        if health_status.status != "ok" && health_status.status != "degraded" {
            alerts.push(format!("Xavier health status: {}", health_status.status));
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

        // Update static last-check values (unified lock — no data race)
        if let Ok(mut r) = LAST_CHECK_RESULT.write() {
            *r = result.clone();
        }

        // Update last_check timestamp
        *self.last_check.write().await = Instant::now();

        result
    }

    /// Start the cron loop for periodic sync checks
    pub async fn start_cron(&self) {
        let mut ticker = interval(Duration::from_millis(self.interval_ms));
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        info!(
            interval_ms = self.interval_ms,
            "SessionSyncTask cron started"
        );

        loop {
            tokio::select! {
                _ = ticker.tick() => {
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
                changed = shutdown_rx.changed() => {
                    if changed.is_err() || *shutdown_rx.borrow() {
                        info!("SessionSyncTask cron shutting down, flushing pending state...");
                        // Run one final sync check to flush any pending state
                        // into LAST_CHECK_RESULT before complete shutdown.
                        // Limited to 5s to avoid blocking graceful shutdown.
                        tokio::select! {
                            _ = self.run_sync_check() => {
                                info!("SessionSyncTask final check completed");
                            }
                            _ = tokio::time::sleep(Duration::from_secs(5)) => {
                                tracing::warn!("SessionSyncTask final check timed out during shutdown");
                            }
                        }
                        break;
                    }
                }
            }
        }
    }

    /// Estimate index lag by querying indexed session records and comparing
    /// the original event timestamp with the timestamp at which the record was indexed.
    async fn estimate_index_lag(&self) -> u64 {
        if let Some(ref storage) = self.memory_store {
            let workspace_id = std::env::var("XAVIER_DEFAULT_WORKSPACE_ID")
                .unwrap_or_else(|_| "default".to_string());
            let filters = MemoryQueryFilters {
                kinds: Some(vec![MemoryKind::Session]),
                ..MemoryQueryFilters::default()
            };

            let records = match storage.list_filtered(&workspace_id, &filters, 100).await {
                Ok(recs) => recs,
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to list session records for lag estimation");
                    return 0;
                }
            };

            if let Some((event_ts, indexed_ts)) = records
                .iter()
                .filter_map(|record| {
                    session_event_timestamp_ms(&record.content)
                        .map(|ts| (ts, record.updated_at.timestamp_millis()))
                })
                .max_by_key(|(event_ts, _)| *event_ts)
            {
                return indexed_ts.saturating_sub(event_ts).max(0) as u64;
            }
        }

        0
    }

    /// Get save_ok_rate from stored metrics
    async fn get_save_ok_rate(&self) -> f64 {
        LAST_CHECK_RESULT
            .read()
            .map(|r| r.save_ok_rate)
            .unwrap_or(1.0)
    }

    /// Get match_score from stored metrics
    async fn get_match_score(&self) -> f64 {
        LAST_CHECK_RESULT
            .read()
            .map(|r| r.match_score)
            .unwrap_or(1.0)
    }

    /// Update metrics (can be called by session event handler)
    pub fn update_metrics(save_ok_rate: f64, match_score: f64, active_agents: u64) {
        if let Ok(mut r) = LAST_CHECK_RESULT.write() {
            r.save_ok_rate = save_ok_rate;
            r.match_score = match_score;
            r.active_agents = active_agents;
        }
    }
}

impl Default for SessionSyncTask {
    fn default() -> Self {
        let (shutdown_tx, _) = watch::channel(false);

        Self {
            interval_ms: read_env_or_legacy("XAVIER_SYNC_INTERVAL_MS", "SEVIER_SYNC_INTERVAL_MS")
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_SYNC_INTERVAL_MS),
            lag_threshold_ms: read_env_or_legacy(
                "XAVIER_SYNC_LAG_THRESHOLD_MS",
                "SEVIER_LAG_THRESHOLD_MS",
            )
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_LAG_THRESHOLD_MS),
            save_ok_rate_threshold: read_env_or_legacy(
                "XAVIER_SYNC_SAVE_OK_RATE_THRESHOLD",
                "SEVIER_SAVE_OK_RATE_THRESHOLD",
            )
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_SAVE_OK_RATE_THRESHOLD),
            max_retries: read_env_or_legacy("XAVIER_SYNC_MAX_RETRIES", "SEVIER_SYNC_MAX_RETRIES")
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_SYNC_MAX_RETRIES),
            min_health_interval_ms: read_env_or_legacy(
                "XAVIER_SYNC_MIN_HEALTH_INTERVAL_MS",
                "SEVIER_SYNC_MIN_HEALTH_INTERVAL_MS",
            )
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_SYNC_MIN_HEALTH_INTERVAL_MS),
            timeout_ms: read_env_or_legacy("XAVIER_SYNC_TIMEOUT_MS", "SEVIER_SYNC_TIMEOUT_MS")
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_SYNC_TIMEOUT_MS),
            health_port: Arc::new({
                let url_str = std::env::var("XAVIER_URL")
                    .unwrap_or_else(|_| "http://localhost:8006".to_string());

                // Validate internal URL to prevent SSRF
                let final_url = match crate::security::url_validator::validate_internal_url(
                    &url_str,
                ) {
                    Ok(_) => url_str,
                    Err(e) => {
                        tracing::error!("XAVIER_URL validation failed in SessionSyncTask: {}. Falling back to localhost.", e);
                        "http://localhost:8006".to_string()
                    }
                };

                let client = reqwest::Client::builder()
                    .timeout(Duration::from_secs(30))
                    .user_agent(concat!("xavier-sync-task/", env!("CARGO_PKG_VERSION")))
                    .build()
                    .expect("failed to build sync task HTTP client");

                crate::adapters::outbound::http_health_adapter::HttpHealthAdapter::new(final_url, client)
            }),
            memory_store: None,
            last_check: Arc::new(TokioRwLock::new(Instant::now())),
            shutdown_tx,
        }
    }
}

/// Get last sync check result (for REST endpoint) — consistent snapshot via unified lock.
pub fn get_last_sync_result() -> SyncCheckResult {
    LAST_CHECK_RESULT
        .read()
        .map(|r| r.clone())
        .unwrap_or_default()
}

fn read_env_or_legacy(primary: &str, legacy: &str) -> Option<String> {
    std::env::var(primary)
        .ok()
        .or_else(|| std::env::var(legacy).ok())
}

fn session_event_timestamp_ms(content: &str) -> Option<i64> {
    let value: serde_json::Value = serde_json::from_str(content).ok()?;
    timestamp_ms_from_json(&value)
}

fn timestamp_ms_from_json(value: &serde_json::Value) -> Option<i64> {
    let object = value.as_object()?;

    for key in ["timestamp", "event_timestamp", "created_at"] {
        if let Some(timestamp) = object.get(key).and_then(parse_timestamp_ms) {
            return Some(timestamp);
        }
    }

    object.get("metadata").and_then(timestamp_ms_from_json)
}

fn parse_timestamp_ms(value: &serde_json::Value) -> Option<i64> {
    if let Some(milliseconds) = value.as_i64() {
        return Some(milliseconds);
    }

    let timestamp = value.as_str()?;
    chrono::DateTime::parse_from_rfc3339(timestamp)
        .ok()
        .map(|dt| dt.timestamp_millis())
}
