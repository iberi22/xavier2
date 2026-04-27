use crate::hooks::{SessionSnapshot, FileEdit, GitOp, TaskState, Decision};
use rusqlite::{params, Connection};
use std::sync::{Arc, Mutex};
use anyhow::{Result, Error};
use chrono::{DateTime, Utc};

pub struct SessionStore {
    db: Arc<Mutex<Connection>>,
}

impl SessionStore {
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS session_snapshots (
                session_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                data TEXT NOT NULL
            )",
            [],
        )?;
        Ok(Self {
            db: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn new_with_conn(conn: Connection) -> Result<Self> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS session_snapshots (
                session_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                data TEXT NOT NULL
            )",
            [],
        )?;
        Ok(Self {
            db: Arc::new(Mutex::new(conn)),
        })
    }

    pub async fn save_snapshot(&self, snapshot: SessionSnapshot) -> Result<(), Error> {
        let db = self.db.lock().map_err(|e| anyhow::anyhow!("mutex error: {}", e))?;
        let data = serde_json::to_string(&snapshot)?;
        db.execute(
            "INSERT INTO session_snapshots (session_id, timestamp, data) VALUES (?1, ?2, ?3)",
            params![snapshot.session_id, snapshot.timestamp.to_rfc3339(), data],
        )?;
        Ok(())
    }

    pub async fn get_snapshot(&self, session_id: &str) -> Result<Option<SessionSnapshot>, Error> {
        let db = self.db.lock().map_err(|e| anyhow::anyhow!("mutex error: {}", e))?;
        let mut stmt = db.prepare(
            "SELECT data FROM session_snapshots WHERE session_id = ?1 ORDER BY timestamp DESC LIMIT 1"
        )?;
        let mut rows = stmt.query(params![session_id])?;

        if let Some(row) = rows.next()? {
            let data: String = row.get(0)?;
            let snapshot: SessionSnapshot = serde_json::from_str(&data)?;
            Ok(Some(snapshot))
        } else {
            Ok(None)
        }
    }

    pub async fn search_snapshots(&self, _query: &str) -> Result<Vec<SessionSnapshot>, Error> {
        // Implementación básica: devolver todos los snapshots de la sesión actual o similar.
        // Para búsqueda BM25 en snapshots, integraríamos el BM25Index aquí.
        let db = self.db.lock().map_err(|e| anyhow::anyhow!("mutex error: {}", e))?;
        let mut stmt = db.prepare("SELECT data FROM session_snapshots ORDER BY timestamp DESC")?;
        let rows = stmt.query_map([], |row| {
            let data: String = row.get(0)?;
            Ok(data)
        })?;

        let mut results = Vec::new();
        for data_res in rows {
            if let Ok(data) = data_res {
                if let Ok(snapshot) = serde_json::from_str(&data) {
                    results.push(snapshot);
                }
            }
        }
        Ok(results)
    }
}
