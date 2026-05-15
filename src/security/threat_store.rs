use anyhow::Result;
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use std::sync::Arc;
use crate::ports::outbound::schema_init::SchemaInitializer;
use crate::security::detections::Threat;
use sha2::{Digest, Sha256};
use chrono::Utc;

pub struct SecurityThreatStore {
    conn: Arc<Mutex<Connection>>,
}

impl SecurityThreatStore {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    pub fn save_threat(&self, threat: &Threat, component: &str) -> Result<()> {
        let conn = self.conn.lock();

        // 1. Get previous hash from the chain
        let prev_hash: Option<String> = conn.query_row(
            "SELECT threat_hash FROM security_threat_chain ORDER BY created_at DESC LIMIT 1",
            [],
            |row| row.get(0),
        ).ok();

        // 2. Compute current threat hash for the chain
        let mut hasher = Sha256::new();
        if let Some(ref h) = prev_hash {
            hasher.update(h.as_bytes());
        }
        hasher.update(threat.severity.as_str().as_bytes());
        hasher.update(threat.category.as_str().as_bytes());
        hasher.update(threat.message.as_bytes());
        hasher.update(threat.evidence.as_bytes());
        hasher.update(component.as_bytes());
        let threat_hash = hex::encode(hasher.finalize());

        let id = ulid::Ulid::new().to_string();
        let now = Utc::now().to_rfc3339();

        // 3. Save threat and chain entry in a transaction
        conn.execute(
            "INSERT INTO security_threats (id, severity, layer, category, message, evidence, context, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                id,
                threat.severity.as_str(),
                threat.layer,
                threat.category.as_str(),
                threat.message,
                threat.evidence,
                component,
                now
            ],
        )?;

        conn.execute(
            "INSERT INTO security_threat_chain (id, prev_hash, threat_hash, created_at)
             VALUES (?, ?, ?, ?)",
            params![id, prev_hash, threat_hash, now],
        )?;

        Ok(())
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

            CREATE TABLE IF NOT EXISTS security_threat_chain (
                id TEXT PRIMARY KEY,
                prev_hash TEXT,
                threat_hash TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_threats_severity ON security_threats(severity);
            CREATE INDEX IF NOT EXISTS idx_threats_created ON security_threats(created_at);
            CREATE INDEX IF NOT EXISTS idx_threat_chain_created ON security_threat_chain(created_at);
            "#,
        )?;
        Ok(())
    }
}
