use super::lending::AuditLogger;
use chrono::Utc;
use parking_lot::Mutex;
use rusqlite::params;
use std::sync::Arc;

pub struct QmdAuditLogger {
    conn: Arc<Mutex<rusqlite::Connection>>,
}

impl QmdAuditLogger {
    pub fn new(conn: Arc<Mutex<rusqlite::Connection>>) -> Self {
        Self { conn }
    }

    pub fn init_schema(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS secret_audit_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                event_type TEXT NOT NULL,
                agent_id TEXT NOT NULL,
                session_token TEXT NOT NULL,
                secret_id TEXT,
                reason TEXT
            )",
            [],
        )?;
        Ok(())
    }
}

impl AuditLogger for QmdAuditLogger {
    fn log_lend(&self, agent_id: &str, secret_id: &str, session_token: &str, ttl_secs: u64) {
        let conn = self.conn.lock();
        let now = Utc::now().to_rfc3339();
        let _ = conn.execute(
            "INSERT INTO secret_audit_logs (timestamp, event_type, agent_id, session_token, secret_id, reason)
             VALUES (?, ?, ?, ?, ?, ?)",
            params![now, "LEND", agent_id, session_token, secret_id, format!("TTL: {}s", ttl_secs)],
        );
    }

    fn log_revoke(&self, agent_id: &str, session_token: &str, reason: &str) {
        let conn = self.conn.lock();
        let now = Utc::now().to_rfc3339();
        let _ = conn.execute(
            "INSERT INTO secret_audit_logs (timestamp, event_type, agent_id, session_token, reason)
             VALUES (?, ?, ?, ?, ?)",
            params![now, "REVOKE", agent_id, session_token, reason],
        );
    }
}
