//! SQLite backend for Xavier2 memory store.
//!
//! Provides a persistent, ACID-compliant storage layer using SQLite.
//! Used as a fallback when SurrealDB is unavailable.

use std::{path::PathBuf, sync::Arc};

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use r2d2::Pool;
use r2d2_rusqlite::RusqliteConnectionManager;
use rusqlite::{params, Connection};
use tokio::fs;

use crate::checkpoint::Checkpoint;
use crate::memory::belief_graph::BeliefRelation;
use crate::memory::schema::MemoryQueryFilters;
use crate::memory::surreal_store::{
    filter_records, revisioned_record, stable_key, DurableWorkspaceState, MemoryBackend,
    MemoryRecord, MemoryStore, SessionTokenRecord, SessionTokenRow, TABLE_BELIEFS,
    TABLE_CHECKPOINTS, TABLE_MEMORIES, TABLE_SESSION_TOKENS,
};

const DB_FILENAME: &str = "xavier2_memory.db";

#[derive(Debug, Clone)]
pub struct SqliteStoreConfig {
    pub path: PathBuf,
}

impl SqliteStoreConfig {
    pub fn from_env() -> Self {
        let data_dir = std::env::var("XAVIER2_DATA_DIR").unwrap_or_else(|_| "/data".to_string());
        Self {
            path: PathBuf::from(data_dir).join(DB_FILENAME),
        }
    }

    fn detail(&self) -> String {
        self.path.display().to_string()
    }
}

#[derive(Clone)]
pub struct SqliteMemoryStore {
    pool: Pool<RusqliteConnectionManager>,
    config: SqliteStoreConfig,
}

impl SqliteMemoryStore {
    pub async fn from_env() -> Result<Self> {
        Self::new(SqliteStoreConfig::from_env()).await
    }

    pub async fn new(config: SqliteStoreConfig) -> Result<Self> {
        if let Some(parent) = config.path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let manager = RusqliteConnectionManager::file(&config.path);
        let pool = Pool::builder()
            .max_size(10)
            .build(manager)
            .context("failed to create sqlite connection pool")?;

        let conn = pool.get()?;

        // Enable WAL mode for better concurrency
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;

        // Initialize schema
        conn.execute_batch(&format!(
            r#"
            CREATE TABLE IF NOT EXISTS {} (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                path TEXT NOT NULL,
                content TEXT NOT NULL,
                metadata TEXT NOT NULL DEFAULT '{{}}',
                embedding BLOB,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                revision INTEGER NOT NULL DEFAULT 1,
                primary_flag INTEGER NOT NULL DEFAULT 1,
                parent_id TEXT,
                revisions TEXT NOT NULL DEFAULT '[]'
            );

            CREATE TABLE IF NOT EXISTS {} (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                beliefs TEXT NOT NULL DEFAULT '[]',
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS {} (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                token TEXT NOT NULL,
                created_at TEXT NOT NULL,
                expires_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS {} (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                task_id TEXT NOT NULL,
                name TEXT NOT NULL,
                data TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_memories_workspace ON {}(workspace_id);
            CREATE INDEX IF NOT EXISTS idx_memories_path ON {}(workspace_id, path);
            CREATE INDEX IF NOT EXISTS idx_session_tokens_workspace ON {}(workspace_id);
            CREATE INDEX IF NOT EXISTS idx_checkpoints_workspace ON {}(workspace_id);
            CREATE INDEX IF NOT EXISTS idx_checkpoints_task ON {}(workspace_id, task_id);
            "#,
            TABLE_MEMORIES,
            TABLE_BELIEFS,
            TABLE_SESSION_TOKENS,
            TABLE_CHECKPOINTS,
            TABLE_MEMORIES,
            TABLE_MEMORIES,
            TABLE_SESSION_TOKENS,
            TABLE_CHECKPOINTS,
            TABLE_CHECKPOINTS
        ))?;

        Ok(Self { pool, config })
    }

    fn serialize_embedding(embedding: &[f32]) -> Vec<u8> {
        embedding.iter().flat_map(|v| v.to_le_bytes()).collect()
    }

    fn deserialize_embedding(data: &[u8]) -> Vec<f32> {
        data.chunks_exact(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect()
    }

    fn row_key(workspace_id: &str, memory_id: &str) -> String {
        stable_key("sqlite_mem", &[workspace_id, memory_id])
    }

    fn deserialize_record(row: &rusqlite::Row) -> rusqlite::Result<MemoryRecord> {
        let metadata_str: String = row.get(3)?;
        let revisions_str: String = row.get(12)?;
        let embedding_blob: Vec<u8> = row.get(5)?;

        Ok(MemoryRecord {
            id: row.get(0)?,
            workspace_id: row.get(1)?,
            path: row.get(2)?,
            content: row.get(3)?,
            metadata: serde_json::from_str(&metadata_str).unwrap_or_default(),
            embedding: Self::deserialize_embedding(&embedding_blob),
            created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            revision: row.get(8)?,
            primary: row.get::<_, i32>(9)? != 0,
            parent_id: row.get(10)?,
            revisions: serde_json::from_str(&revisions_str).unwrap_or_default(),
        })
    }
}

#[async_trait]
impl MemoryStore for SqliteMemoryStore {
    fn backend(&self) -> MemoryBackend {
        MemoryBackend::Sqlite
    }

    async fn health(&self) -> Result<String> {
        let conn = self.pool.get()?;
        conn.execute("SELECT 1", [])?;
        Ok(format!("sqlite {}", self.config.detail()))
    }

    async fn put(&self, record: MemoryRecord) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            &format!(
                "INSERT OR REPLACE INTO {} (id, workspace_id, path, content, metadata, embedding, created_at, updated_at, revision, primary_flag, parent_id, revisions) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                TABLE_MEMORIES
            ),
            params![
                record.id,
                record.workspace_id,
                record.path,
                record.content,
                serde_json::to_string(&record.metadata).unwrap_or_default(),
                Self::serialize_embedding(&record.embedding),
                record.created_at.to_rfc3339(),
                record.updated_at.to_rfc3339(),
                record.revision,
                record.primary as i32,
                record.parent_id,
                serde_json::to_string(&record.revisions).unwrap_or_default(),
            ],
        )?;
        Ok(())
    }

    async fn get(&self, workspace_id: &str, id_or_path: &str) -> Result<Option<MemoryRecord>> {
        let conn = self.pool.get()?;

        // Try by id first (O(1) lookup)
        let key = Self::row_key(workspace_id, id_or_path);
        let mut stmt = conn.prepare(&format!(
            "SELECT id, workspace_id, path, content, metadata, embedding, created_at, updated_at, revision, primary_flag, parent_id, revisions FROM {} WHERE id = ?",
            TABLE_MEMORIES
        ))?;

        let mut rows = stmt.query([&key])?;
        if let Some(row) = rows.next()? {
            return Ok(Some(Self::deserialize_record(row)?));
        }
        drop(rows);
        drop(stmt);

        // Fallback: try by path
        let mut stmt = conn.prepare(&format!(
            "SELECT id, workspace_id, path, content, metadata, embedding, created_at, updated_at, revision, primary_flag, parent_id, revisions FROM {} WHERE workspace_id = ? AND path = ?",
            TABLE_MEMORIES
        ))?;

        let mut rows = stmt.query(params![workspace_id, id_or_path])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Self::deserialize_record(row)?))
        } else {
            Ok(None)
        }
    }

    async fn update(&self, record: MemoryRecord) -> Result<()> {
        let record = if let Some(existing) = self.get(&record.workspace_id, &record.id).await? {
            revisioned_record(existing, record)
        } else if let Some(existing) = self.get(&record.workspace_id, &record.path).await? {
            revisioned_record(existing, record)
        } else {
            record
        };
        self.put(record).await
    }

    async fn delete(&self, workspace_id: &str, id_or_path: &str) -> Result<Option<MemoryRecord>> {
        let removed = self.get(workspace_id, id_or_path).await?;
        if let Some(record) = &removed {
            let key = Self::row_key(workspace_id, &record.id);
            let conn = self.pool.get()?;
            conn.execute(
                &format!("DELETE FROM {} WHERE id = ?", TABLE_MEMORIES),
                [&key],
            )?;

            // Also delete children
            conn.execute(
                &format!(
                    "DELETE FROM {} WHERE workspace_id = ? AND parent_id = ?",
                    TABLE_MEMORIES
                ),
                params![workspace_id, &record.id],
            )?;
        }
        Ok(removed)
    }

    async fn list(&self, workspace_id: &str) -> Result<Vec<MemoryRecord>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(&format!(
            "SELECT id, workspace_id, path, content, metadata, embedding, created_at, updated_at, revision, primary_flag, parent_id, revisions FROM {} WHERE workspace_id = ?",
            TABLE_MEMORIES
        ))?;

        let mut rows = stmt.query([workspace_id])?;
        let mut records = Vec::new();
        while let Some(row) = rows.next()? {
            if let Ok(record) = Self::deserialize_record(row) {
                records.push(record);
            }
        }
        Ok(records)
    }

    async fn search(
        &self,
        workspace_id: &str,
        query: &str,
        filters: Option<&MemoryQueryFilters>,
    ) -> Result<Vec<MemoryRecord>> {
        filter_records(self.list(workspace_id).await?, workspace_id, query, filters)
    }

    async fn load_workspace_state(&self, workspace_id: &str) -> Result<DurableWorkspaceState> {
        let conn = self.pool.get()?;

        // Load memories
        let mut memories = Vec::new();
        {
            let mut stmt = conn.prepare(&format!(
                "SELECT id, workspace_id, path, content, metadata, embedding, created_at, updated_at, revision, primary_flag, parent_id, revisions FROM {} WHERE workspace_id = ?",
                TABLE_MEMORIES
            ))?;
            let mut rows = stmt.query([workspace_id])?;
            while let Some(row) = rows.next()? {
                if let Ok(record) = Self::deserialize_record(row) {
                    memories.push(record);
                }
            }
        }

        // Load beliefs
        let beliefs = {
            let belief_key = stable_key("belief_row", &[workspace_id]);
            let mut stmt = conn.prepare(&format!(
                "SELECT beliefs FROM {} WHERE id = ?",
                TABLE_BELIEFS
            ))?;
            match stmt.query_row([&belief_key], |row| {
                let beliefs_str: String = row.get(0)?;
                Ok(beliefs_str)
            }) {
                Ok(beliefs_str) => serde_json::from_str(&beliefs_str).unwrap_or_default(),
                Err(_) => Vec::new(),
            }
        };

        // Load session tokens (filter expired)
        let now = Utc::now();
        let session_tokens = {
            let mut stmt = conn.prepare(&format!(
                "SELECT id, workspace_id, token, created_at, expires_at FROM {} WHERE workspace_id = ?",
                TABLE_SESSION_TOKENS
            ))?;
            let mut rows = stmt.query([workspace_id])?;
            let mut tokens = Vec::new();
            while let Some(row) = rows.next()? {
                let token_row = SessionTokenRow {
                    storage_id: row.get(0)?,
                    workspace_id: row.get(1)?,
                    token: row.get(2)?,
                    created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    expires_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                };
                if token_row.expires_at > now {
                    tokens.push(SessionTokenRecord::from(token_row));
                }
            }
            tokens
        };

        // Load checkpoints
        let checkpoints = {
            let mut stmt = conn.prepare(&format!(
                "SELECT task_id, name, data FROM {} WHERE workspace_id = ?",
                TABLE_CHECKPOINTS
            ))?;
            let mut rows = stmt.query([workspace_id])?;
            let mut checkpoints = Vec::new();
            while let Some(row) = rows.next()? {
                checkpoints.push(Checkpoint {
                    task_id: row.get(0)?,
                    name: row.get(1)?,
                    data: serde_json::from_str(&row.get::<_, String>(2)?).unwrap_or_default(),
                });
            }
            checkpoints
        };

        Ok(DurableWorkspaceState {
            memories,
            beliefs,
            session_tokens,
            checkpoints,
        })
    }

    async fn save_beliefs(&self, workspace_id: &str, beliefs: Vec<BeliefRelation>) -> Result<()> {
        let belief_key = stable_key("belief_row", &[workspace_id]);
        let conn = self.pool.get()?;
        let beliefs_json = serde_json::to_string(&beliefs)?;

        conn.execute(
            &format!(
                "INSERT OR REPLACE INTO {} (id, workspace_id, beliefs, updated_at) VALUES (?, ?, ?, ?)",
                TABLE_BELIEFS
            ),
            params![belief_key, workspace_id, beliefs_json, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    async fn save_session_token(
        &self,
        workspace_id: &str,
        token: SessionTokenRecord,
    ) -> Result<()> {
        let token_key = stable_key("session_token_row", &[workspace_id, &token.token]);
        let conn = self.pool.get()?;

        // Delete expired tokens first
        conn.execute(
            &format!(
                "DELETE FROM {} WHERE workspace_id = ? AND expires_at <= ?",
                TABLE_SESSION_TOKENS
            ),
            params![workspace_id, Utc::now().to_rfc3339()],
        )?;

        // Insert new token
        conn.execute(
            &format!(
                "INSERT OR REPLACE INTO {} (id, workspace_id, token, created_at, expires_at) VALUES (?, ?, ?, ?, ?)",
                TABLE_SESSION_TOKENS
            ),
            params![
                token_key,
                workspace_id,
                token.token,
                token.created_at.to_rfc3339(),
                token.expires_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    async fn is_session_token_valid(&self, workspace_id: &str, token: &str) -> Result<bool> {
        let token_key = stable_key("session_token_row", &[workspace_id, token]);
        let conn = self.pool.get()?;
        let now = Utc::now().to_rfc3339();

        let count: i32 = conn.query_row(
            &format!(
                "SELECT COUNT(*) FROM {} WHERE id = ? AND expires_at > ?",
                TABLE_SESSION_TOKENS
            ),
            params![token_key, now],
            |row| row.get(0),
        )?;

        Ok(count > 0)
    }

    async fn save_checkpoint(&self, workspace_id: &str, checkpoint: Checkpoint) -> Result<()> {
        let checkpoint_key = stable_key(
            "checkpoint_row",
            &[workspace_id, &checkpoint.task_id, &checkpoint.name],
        );
        let conn = self.pool.get()?;
        let data_json = serde_json::to_string(&checkpoint.data)?;

        conn.execute(
            &format!(
                "INSERT OR REPLACE INTO {} (id, workspace_id, task_id, name, data) VALUES (?, ?, ?, ?, ?)",
                TABLE_CHECKPOINTS
            ),
            params![checkpoint_key, workspace_id, checkpoint.task_id, checkpoint.name, data_json],
        )?;
        Ok(())
    }

    async fn load_checkpoint(
        &self,
        workspace_id: &str,
        task_id: &str,
        name: &str,
    ) -> Result<Option<Checkpoint>> {
        let conn = self.pool.get()?;

        let mut stmt = conn.prepare(&format!(
            "SELECT data FROM {} WHERE workspace_id = ? AND task_id = ? AND name = ?",
            TABLE_CHECKPOINTS
        ))?;

        match stmt.query_row(params![workspace_id, task_id, name], |row| {
            let data_str: String = row.get(0)?;
            Ok(serde_json::from_str(&data_str).unwrap_or_default())
        }) {
            Ok(data) => Ok(Some(Checkpoint {
                task_id: task_id.to_string(),
                name: name.to_string(),
                data,
            })),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(anyhow::anyhow!("SQLite query failed: {}", e)),
        }
    }

    async fn list_checkpoints(&self, workspace_id: &str, task_id: &str) -> Result<Vec<Checkpoint>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(&format!(
            "SELECT task_id, name, data FROM {} WHERE workspace_id = ? AND task_id = ?",
            TABLE_CHECKPOINTS
        ))?;

        let mut rows = stmt.query(params![workspace_id, task_id])?;
        let mut checkpoints = Vec::new();
        while let Some(row) = rows.next()? {
            checkpoints.push(Checkpoint {
                task_id: row.get(0)?,
                name: row.get(1)?,
                data: serde_json::from_str(&row.get::<_, String>(2)?).unwrap_or_default(),
            });
        }
        Ok(checkpoints)
    }

    async fn delete_checkpoint(&self, workspace_id: &str, task_id: &str, name: &str) -> Result<()> {
        let checkpoint_key = stable_key("checkpoint_row", &[workspace_id, task_id, name]);
        let conn = self.pool.get()?;
        conn.execute(
            &format!("DELETE FROM {} WHERE id = ?", TABLE_CHECKPOINTS),
            [&checkpoint_key],
        )?;
        Ok(())
    }
}
