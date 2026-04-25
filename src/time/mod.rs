//! Time Metrics storage for Xavier2.
//!
//! Stores TimeMetric records to SQLite at path: metrics/time/{YYYY-MM-DD}/{metric_type}/{agent_id}

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use chrono::Utc;
use parking_lot::Mutex;
use rusqlite::params;

use crate::adapters::inbound::http::dto::TimeMetricDto;

/// Table name for time metrics
const TABLE_TIME_METRICS: &str = "time_metrics";

/// Time metrics storage adapter
pub struct TimeMetricsStore {
    conn: Arc<Mutex<rusqlite::Connection>>,
    base_path: PathBuf,
}

impl TimeMetricsStore {
    /// Create a new TimeMetricsStore with the given SQLite connection
    pub fn new(conn: Arc<Mutex<rusqlite::Connection>>, base_path: PathBuf) -> Self {
        Self { conn, base_path }
    }

    /// Initialize the time_metrics table schema
    pub fn init_schema(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
        conn.execute_batch(&format!(
            r#"
            CREATE TABLE IF NOT EXISTS {} (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                path TEXT NOT NULL,
                metric_type TEXT NOT NULL,
                agent_id TEXT NOT NULL,
                task_id TEXT,
                started_at TEXT NOT NULL,
                completed_at TEXT NOT NULL,
                duration_ms INTEGER NOT NULL,
                status TEXT NOT NULL,
                error_message TEXT,
                provider TEXT,
                model TEXT,
                tokens_used INTEGER,
                task_category TEXT,
                metadata TEXT NOT NULL DEFAULT '{{}}',
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE INDEX IF NOT EXISTS idx_time_metrics_workspace ON {}(workspace_id);
            CREATE INDEX IF NOT EXISTS idx_time_metrics_agent ON {}(agent_id);
            CREATE INDEX IF NOT EXISTS idx_time_metrics_type ON {}(metric_type);
            CREATE INDEX IF NOT EXISTS idx_time_metrics_path ON {}(path);
            "#,
            TABLE_TIME_METRICS,
            TABLE_TIME_METRICS,
            TABLE_TIME_METRICS,
            TABLE_TIME_METRICS,
            TABLE_TIME_METRICS
        ))
    }

    /// Save a TimeMetric to the store
    pub async fn save_time_metric(
        &self,
        metric: &TimeMetricDto,
        workspace_id: &str,
    ) -> Result<(), String> {
        let id = ulid::Ulid::new().to_string();
        let now = Utc::now();
        let date = now.format("%Y-%m-%d").to_string();

        // Build path: metrics/time/{YYYY-MM-DD}/{metric_type}/{agent_id}
        let path = format!(
            "metrics/time/{}/{}/{}",
            date,
            metric.metric_type,
            metric.agent_id
        );

        let metadata_json =
            serde_json::to_string(&metric.metadata).map_err(|e| e.to_string())?;

        let conn = self.conn.lock();
        conn.execute(
            &format!(
                "INSERT INTO {} (id, workspace_id, path, metric_type, agent_id, task_id, \
                 started_at, completed_at, duration_ms, status, error_message, provider, \
                 model, tokens_used, task_category, metadata) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
                TABLE_TIME_METRICS
            ),
            params![
                id,
                workspace_id,
                path,
                metric.metric_type,
                metric.agent_id,
                metric.task_id,
                metric.started_at,
                metric.completed_at,
                metric.duration_ms,
                metric.status,
                metric.error_message,
                metric.provider,
                metric.model,
                metric.tokens_used,
                metric.task_category,
                metadata_json,
            ],
        )
        .map_err(|e| e.to_string())?;

        Ok(())
    }
}
