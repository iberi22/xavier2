use anyhow::Result;
use parking_lot::Mutex;
use rusqlite::Connection;
use std::sync::Arc;
use crate::ports::outbound::schema_init::SchemaInitializer;

pub struct SecurityThreatStore {
    conn: Arc<Mutex<Connection>>,
}

impl SecurityThreatStore {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }
}

impl SchemaInitializer for SecurityThreatStore {
    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS security_threats (
                id TEXT PRIMARY KEY,
                severity TEXT NOT NULL,
                layer TEXT NOT NULL,
                category TEXT NOT NULL,
                message TEXT NOT NULL,
                evidence TEXT NOT NULL,
                context TEXT DEFAULT '',
                created_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_threats_severity ON security_threats(severity);
            CREATE INDEX IF NOT EXISTS idx_threats_created ON security_threats(created_at);
            "#,
        )?;
        Ok(())
    }
}
