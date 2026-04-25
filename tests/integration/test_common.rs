//! Common test infrastructure for Xavier2 integration tests.
//!
//! Provides shared test helpers for SEVIER2 endpoint tests.

use std::sync::Arc;
use parking_lot::Mutex;
use rusqlite::Connection;

use xavier2::adapters::inbound::http::dto::TimeMetricDto;
use xavier2::time::TimeMetricsStore;

// ─── In-Memory SQLite for Tests ──────────────────────────────────────────────

/// Create an in-memory SQLite connection for testing.
pub fn create_test_db() -> Arc<Mutex<Connection>> {
    let conn = Connection::open_in_memory().expect("failed to create in-memory DB");
    Arc::new(Mutex::new(conn))
}

/// Initialise the time_metrics table schema in a test DB.
pub fn init_time_metrics_schema(conn: &Connection) -> rusqlite::Result<()> {
    TimeMetricsStore::init_schema(conn)
}

// ─── Sample Fixtures ────────────────────────────────────────────────────────

/// A sample `TimeMetricDto` for use in tests.
pub fn sample_time_metric() -> TimeMetricDto {
    TimeMetricDto {
        metric_type: "agent_execution".to_string(),
        agent_id: "test-agent-001".to_string(),
        task_id: Some("task-123".to_string()),
        started_at: "2026-04-24T10:00:00Z".to_string(),
        completed_at: "2026-04-24T10:00:05Z".to_string(),
        duration_ms: 5000,
        status: "success".to_string(),
        error_message: None,
        provider: Some("minimax".to_string()),
        model: Some("MiniMax-M2.7".to_string()),
        tokens_used: Some(1500),
        task_category: Some("coding".to_string()),
        metadata: serde_json::json!({}),
    }
}
