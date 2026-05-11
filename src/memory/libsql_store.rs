//! LibSQL backend for Xavier memory store.
//!
//! Provides a persistent, Turso-compatible storage layer using LibSQL.

use std::{any::Any, path::PathBuf};

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use libsql::{params, Connection};
use tokio::fs;

use crate::checkpoint::Checkpoint;
use crate::memory::belief_graph::BeliefRelation;
use crate::memory::schema::MemoryQueryFilters;
use crate::memory::store::{
    filter_records, revisioned_record, stable_key, DurableWorkspaceState, MemoryBackend,
    MemoryRecord, MemoryStore, SessionTokenRecord,
};
use crate::settings::XavierSettings;

const DB_FILENAME: &str = "xavier_memory.libsql";
pub(crate) const TABLE_MEMORIES: &str = "memory_records";
pub(crate) const TABLE_BELIEFS: &str = "belief_states";
pub(crate) const TABLE_SESSION_TOKENS: &str = "session_tokens";
pub(crate) const TABLE_CHECKPOINTS: &str = "checkpoint_records";

struct SessionTokenRow {
    token: String,
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}

impl From<SessionTokenRow> for SessionTokenRecord {
    fn from(value: SessionTokenRow) -> Self {
        Self {
            token: value.token,
            created_at: value.created_at,
            expires_at: value.expires_at,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LibsqlStoreConfig {
    pub path: PathBuf,
}

impl LibsqlStoreConfig {
    pub fn from_env() -> Self {
        let settings = XavierSettings::current();
        Self {
            path: std::env::var("XAVIER_MEMORY_LIBSQL_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|_| {
                    if settings.memory.libsql_path.trim().is_empty() {
                        PathBuf::from(&settings.memory.data_dir).join(DB_FILENAME)
                    } else {
                        PathBuf::from(&settings.memory.libsql_path)
                    }
                }),
        }
    }

    fn detail(&self) -> String {
        self.path.display().to_string()
    }
}

pub struct LibsqlMemoryStore {
    conn: Connection,
    config: LibsqlStoreConfig,
}

impl LibsqlMemoryStore {
    pub async fn from_env() -> Result<Self> {
        Self::new(LibsqlStoreConfig::from_env()).await
    }

    pub async fn new(config: LibsqlStoreConfig) -> Result<Self> {
        if let Some(parent) = config.path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let db = libsql::Builder::new_local(&config.path)
            .build()
            .await
            .with_context(|| format!("failed to open LibSQL database at {}", config.path.display()))?;
        let conn = db.connect().context("failed to connect to LibSQL database")?;

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
                cluster_id TEXT,
                level TEXT NOT NULL DEFAULT 'raw',
                relation TEXT,
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

            CREATE INDEX IF NOT EXISTS idx_memories_workspace_libsql ON {}(workspace_id);
            CREATE INDEX IF NOT EXISTS idx_memories_path_libsql ON {}(workspace_id, path);
            CREATE INDEX IF NOT EXISTS idx_session_tokens_workspace_libsql ON {}(workspace_id);
            CREATE INDEX IF NOT EXISTS idx_checkpoints_workspace_libsql ON {}(workspace_id);
            CREATE INDEX IF NOT EXISTS idx_checkpoints_task_libsql ON {}(workspace_id, task_id);
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
        )).await?;

        Ok(Self {
            conn,
            config,
        })
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

    fn deserialize_record(row: &libsql::Row) -> Result<MemoryRecord> {
        let id: String = row.get(0)?;
        let workspace_id: String = row.get(1)?;
        let path: String = row.get(2)?;
        let content: String = row.get(3)?;
        let metadata_str: String = row.get(4)?;
        let embedding_blob: Vec<u8> = row.get(5)?;
        let created_at_str: String = row.get(6)?;
        let updated_at_str: String = row.get(7)?;
        let revision: i64 = row.get(8)?;
        let primary_flag: i32 = row.get(9)?;
        let parent_id: Option<String> = row.get(10)?;
        let cluster_id: Option<String> = row.get(11)?;
        let level_str: String = row.get(12)?;
        let relation_str: Option<String> = row.get(13)?;
        let revisions_str: String = row.get(14)?;

        Ok(MemoryRecord {
            id,
            workspace_id,
            path,
            content,
            metadata: serde_json::from_str(&metadata_str).unwrap_or_default(),
            embedding: Self::deserialize_embedding(&embedding_blob),
            created_at: DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            updated_at: DateTime::parse_from_rfc3339(&updated_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            revision: revision as u64,
            primary: primary_flag != 0,
            parent_id,
            cluster_id,
            level: crate::memory::schema::MemoryLevel::parse(&level_str).unwrap_or(crate::memory::schema::MemoryLevel::Raw),
            relation: relation_str.and_then(|s| crate::memory::schema::RelationKind::parse(&s)),
            revisions: serde_json::from_str(&revisions_str).unwrap_or_default(),
        })
    }
}

#[async_trait]
impl MemoryStore for LibsqlMemoryStore {
    fn backend(&self) -> MemoryBackend {
        MemoryBackend::Libsql
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    async fn health(&self) -> Result<String> {
        self.conn.query("SELECT 1", ()).await?;
        Ok(format!("libsql {}", self.config.detail()))
    }

    async fn put(&self, record: MemoryRecord) -> Result<()> {
        self.conn.execute(
            &format!(
                "INSERT OR REPLACE INTO {} (id, workspace_id, path, content, metadata, embedding, created_at, updated_at, revision, primary_flag, parent_id, cluster_id, level, relation, revisions) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
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
                record.cluster_id,
                record.level.as_str(),
                record.relation.map(|r| r.as_str()),
                serde_json::to_string(&record.revisions).unwrap_or_default(),
            ],
        ).await?;
        Ok(())
    }

    async fn get(&self, workspace_id: &str, id_or_path: &str) -> Result<Option<MemoryRecord>> {
        // Try by id first
        let key = Self::row_key(workspace_id, id_or_path);
        let mut rows = self.conn.query(
            &format!(
                "SELECT id, workspace_id, path, content, metadata, embedding, created_at, updated_at, revision, primary_flag, parent_id, cluster_id, level, relation, revisions FROM {} WHERE id = ?1",
                TABLE_MEMORIES
            ),
            params![key],
        ).await?;

        if let Some(row) = rows.next().await? {
            return Ok(Some(Self::deserialize_record(&row)?));
        }

        // Fallback: try by path
        let mut rows = self.conn.query(
            &format!(
                "SELECT id, workspace_id, path, content, metadata, embedding, created_at, updated_at, revision, primary_flag, parent_id, cluster_id, level, relation, revisions FROM {} WHERE workspace_id = ?1 AND path = ?2",
                TABLE_MEMORIES
            ),
            params![workspace_id, id_or_path],
        ).await?;

        if let Some(row) = rows.next().await? {
            Ok(Some(Self::deserialize_record(&row)?))
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
            self.conn.execute(
                &format!("DELETE FROM {} WHERE id = ?1", TABLE_MEMORIES),
                params![key],
            ).await?;

            // Also delete children
            self.conn.execute(
                &format!(
                    "DELETE FROM {} WHERE workspace_id = ?1 AND parent_id = ?2",
                    TABLE_MEMORIES
                ),
                params![workspace_id.to_string(), record.id.clone()],
            ).await?;
        }
        Ok(removed)
    }

    async fn list(&self, workspace_id: &str) -> Result<Vec<MemoryRecord>> {
        let mut rows = self.conn.query(
            &format!(
                "SELECT id, workspace_id, path, content, metadata, embedding, created_at, updated_at, revision, primary_flag, parent_id, cluster_id, level, relation, revisions FROM {} WHERE workspace_id = ?1",
                TABLE_MEMORIES
            ),
            params![workspace_id],
        ).await?;

        let mut records = Vec::new();
        while let Some(row) = rows.next().await? {
            if let Ok(record) = Self::deserialize_record(&row) {
                records.push(record);
            }
        }
        Ok(records)
    }

    async fn export(&self, path: &std::path::Path) -> Result<()> {
        // VACUUM INTO might not be directly supported or behaves differently in LibSQL via API.
        // For simplicity, we could bail or attempt to run the command.
        // LibSQL local might support it.
        self.conn.execute(&format!("VACUUM INTO '{}'", path.to_string_lossy()), ()).await?;
        Ok(())
    }

    async fn export_tree(&self, workspace_id: &str, path: &std::path::Path) -> Result<()> {
        let records = self.list(workspace_id).await?;
        let tree = crate::memory::store::build_context_tree(records);
        let json = serde_json::to_string_pretty(&tree)?;
        tokio::fs::write(path, json).await?;
        Ok(())
    }

    async fn import(&self, _path: &std::path::Path) -> Result<()> {
        anyhow::bail!("Import into an active LibSQL store is not yet supported. Use the CLI to load a context file.")
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
        // Load memories
        let mut memories = Vec::new();
        {
            let mut rows = self.conn.query(
                &format!(
                    "SELECT id, workspace_id, path, content, metadata, embedding, created_at, updated_at, revision, primary_flag, parent_id, cluster_id, level, relation, revisions FROM {} WHERE workspace_id = ?1",
                    TABLE_MEMORIES
                ),
                params![workspace_id],
            ).await?;

            while let Some(row) = rows.next().await? {
                if let Ok(record) = Self::deserialize_record(&row) {
                    memories.push(record);
                }
            }
        }

        // Load beliefs
        let beliefs = {
            let belief_key = stable_key("belief_row", &[workspace_id]);
            let mut rows = self.conn.query(
                &format!(
                    "SELECT beliefs FROM {} WHERE id = ?1",
                    TABLE_BELIEFS
                ),
                params![belief_key],
            ).await?;

            if let Some(row) = rows.next().await? {
                let beliefs_str: String = row.get(0)?;
                serde_json::from_str(&beliefs_str).unwrap_or_default()
            } else {
                Vec::new()
            }
        };

        // Load session tokens (filter expired)
        let now = Utc::now();
        let session_tokens = {
            let mut rows = self.conn.query(
                &format!(
                    "SELECT token, created_at, expires_at FROM {} WHERE workspace_id = ?1",
                    TABLE_SESSION_TOKENS
                ),
                params![workspace_id],
            ).await?;

            let mut tokens = Vec::new();
            while let Some(row) = rows.next().await? {
                let token: String = row.get(0)?;
                let created_at_str: String = row.get(1)?;
                let expires_at_str: String = row.get(2)?;

                let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                let expires_at = DateTime::parse_from_rfc3339(&expires_at_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                if expires_at > now {
                    tokens.push(SessionTokenRecord {
                        token,
                        created_at,
                        expires_at,
                    });
                }
            }
            tokens
        };

        // Load checkpoints
        let checkpoints = {
            let mut rows = self.conn.query(
                &format!(
                    "SELECT task_id, name, data FROM {} WHERE workspace_id = ?1",
                    TABLE_CHECKPOINTS
                ),
                params![workspace_id],
            ).await?;

            let mut checkpoints = Vec::new();
            while let Some(row) = rows.next().await? {
                let task_id: String = row.get(0)?;
                let name: String = row.get(1)?;
                let data_str: String = row.get(2)?;
                checkpoints.push(Checkpoint {
                    task_id,
                    name,
                    data: serde_json::from_str(&data_str).unwrap_or_default(),
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
        let beliefs_json = serde_json::to_string(&beliefs)?;

        self.conn.execute(
            &format!(
                "INSERT OR REPLACE INTO {} (id, workspace_id, beliefs, updated_at) VALUES (?1, ?2, ?3, ?4)",
                TABLE_BELIEFS
            ),
            params![belief_key, workspace_id, beliefs_json, Utc::now().to_rfc3339()],
        ).await?;
        Ok(())
    }

    async fn save_session_token(
        &self,
        workspace_id: &str,
        token: SessionTokenRecord,
    ) -> Result<()> {
        let token_key = stable_key("session_token_row", &[workspace_id, &token.token]);

        // Delete expired tokens first
        self.conn.execute(
            &format!(
                "DELETE FROM {} WHERE workspace_id = ?1 AND expires_at <= ?2",
                TABLE_SESSION_TOKENS
            ),
            params![workspace_id, Utc::now().to_rfc3339()],
        ).await?;

        // Insert new token
        self.conn.execute(
            &format!(
                "INSERT OR REPLACE INTO {} (id, workspace_id, token, created_at, expires_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                TABLE_SESSION_TOKENS
            ),
            params![
                token_key,
                workspace_id,
                token.token,
                token.created_at.to_rfc3339(),
                token.expires_at.to_rfc3339(),
            ],
        ).await?;
        Ok(())
    }

    async fn is_session_token_valid(&self, workspace_id: &str, token: &str) -> Result<bool> {
        let token_key = stable_key("session_token_row", &[workspace_id, token]);
        let now = Utc::now().to_rfc3339();

        let mut rows = self.conn.query(
            &format!(
                "SELECT COUNT(*) FROM {} WHERE id = ?1 AND expires_at > ?2",
                TABLE_SESSION_TOKENS
            ),
            params![token_key, now],
        ).await?;

        if let Some(row) = rows.next().await? {
            let count: i64 = row.get(0)?;
            Ok(count > 0)
        } else {
            Ok(false)
        }
    }

    async fn save_checkpoint(&self, workspace_id: &str, checkpoint: Checkpoint) -> Result<()> {
        let checkpoint_key = stable_key(
            "checkpoint_row",
            &[workspace_id, &checkpoint.task_id, &checkpoint.name],
        );
        let data_json = serde_json::to_string(&checkpoint.data)?;

        self.conn.execute(
            &format!(
                "INSERT OR REPLACE INTO {} (id, workspace_id, task_id, name, data) VALUES (?1, ?2, ?3, ?4, ?5)",
                TABLE_CHECKPOINTS
            ),
            params![checkpoint_key, workspace_id, checkpoint.task_id, checkpoint.name, data_json],
        ).await?;
        Ok(())
    }

    async fn load_checkpoint(
        &self,
        workspace_id: &str,
        task_id: &str,
        name: &str,
    ) -> Result<Option<Checkpoint>> {
        let mut rows = self.conn.query(
            &format!(
                "SELECT data FROM {} WHERE workspace_id = ?1 AND task_id = ?2 AND name = ?3",
                TABLE_CHECKPOINTS
            ),
            params![workspace_id, task_id, name],
        ).await?;

        if let Some(row) = rows.next().await? {
            let data_str: String = row.get(0)?;
            Ok(Some(Checkpoint {
                task_id: task_id.to_string(),
                name: name.to_string(),
                data: serde_json::from_str(&data_str).unwrap_or_default(),
            }))
        } else {
            Ok(None)
        }
    }

    async fn list_checkpoints(&self, workspace_id: &str, task_id: &str) -> Result<Vec<Checkpoint>> {
        let mut rows = self.conn.query(
            &format!(
                "SELECT task_id, name, data FROM {} WHERE workspace_id = ?1 AND task_id = ?2",
                TABLE_CHECKPOINTS
            ),
            params![workspace_id, task_id],
        ).await?;

        let mut checkpoints = Vec::new();
        while let Some(row) = rows.next().await? {
            let task_id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let data_str: String = row.get(2)?;
            checkpoints.push(Checkpoint {
                task_id,
                name,
                data: serde_json::from_str(&data_str).unwrap_or_default(),
            });
        }
        Ok(checkpoints)
    }

    async fn delete_checkpoint(&self, workspace_id: &str, task_id: &str, name: &str) -> Result<()> {
        let checkpoint_key = stable_key("checkpoint_row", &[workspace_id, task_id, name]);
        self.conn.execute(
            &format!("DELETE FROM {} WHERE id = ?1", TABLE_CHECKPOINTS),
            params![checkpoint_key],
        ).await?;
        Ok(())
    }
}
