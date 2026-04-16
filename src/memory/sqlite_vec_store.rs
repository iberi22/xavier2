//! SQLite backend with sqlite-vec vector search for Xavier2 memory store.
//!
//! Uses HNSW-like approximate nearest neighbor search via sqlite-vec
//! for semantic similarity matching on memory embeddings.

use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::OnceLock,
};

use anyhow::{Context, Result};
use async_trait::async_trait;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use regex::Regex;
use rusqlite::ffi::sqlite3_auto_extension;
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
use tokio::fs;

use crate::checkpoint::Checkpoint;
use crate::memory::belief_graph::BeliefRelation;
use crate::memory::embedder::EmbeddingClient;
use crate::memory::schema::MemoryQueryFilters;
use crate::memory::surreal_store::{
    stable_key, DurableWorkspaceState, GraphHopPath, GraphHopResult, HybridSearchMode,
    HybridSearchResult, MemoryBackend, MemoryRecord, MemoryStore, SessionTokenRecord,
    SessionTokenRow, TABLE_BELIEFS, TABLE_CHECKPOINTS, TABLE_MEMORIES, TABLE_SESSION_TOKENS,
};

const DB_FILENAME: &str = "xavier2_memory_vec.db";
const DEFAULT_EMBEDDING_DIMENSIONS: usize = 768;
const DEFAULT_RRF_K: usize = 60;
const DEFAULT_VECTOR_WEIGHT: f32 = 0.40;
const DEFAULT_FTS_WEIGHT: f32 = 0.35;
const DEFAULT_KG_WEIGHT: f32 = 0.25;
const DEFAULT_QJL_THRESHOLD: usize = 30_000;
const QJL_MAGIC: &[u8; 4] = b"QJL2";
static SQLITE_VEC_EXTENSION_INIT: OnceLock<Result<(), String>> = OnceLock::new();

#[derive(Debug, Clone, Copy)]
enum FusionSource {
    Vector,
    Fts,
    Kg,
}

#[derive(Debug, Clone)]
struct ExtractedEntity {
    value: String,
    entity_type: &'static str,
    relation_type: &'static str,
}

#[derive(Debug, Clone)]
struct TimelineEventRecord {
    id: String,
    agent_id: String,
    timestamp: String,
    operation: String,
    prev_hash: Option<String>,
    curr_hash: String,
}

impl FusionSource {
    fn default_weight(self) -> f32 {
        match self {
            Self::Vector => DEFAULT_VECTOR_WEIGHT,
            Self::Fts => DEFAULT_FTS_WEIGHT,
            Self::Kg => DEFAULT_KG_WEIGHT,
        }
    }
}

/// Configuration for the vector-enabled SQLite store
#[derive(Debug, Clone)]
pub struct VecSqliteStoreConfig {
    pub path: PathBuf,
    pub embedding_dimensions: usize,
}

impl VecSqliteStoreConfig {
    pub fn from_env() -> Self {
        let data_dir = std::env::var("XAVIER2_DATA_DIR").unwrap_or_else(|_| "/data".to_string());
        let embedding_dimensions = std::env::var("XAVIER2_EMBEDDING_DIMENSIONS")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(DEFAULT_EMBEDDING_DIMENSIONS);

        Self {
            path: PathBuf::from(data_dir).join(DB_FILENAME),
            embedding_dimensions,
        }
    }

    fn detail(&self) -> String {
        format!(
            "{} ({}d embeddings)",
            self.path.display(),
            self.embedding_dimensions
        )
    }
}

/// Vector-enabled SQLite memory store using sqlite-vec for HNSW-like similarity search.
#[derive(Clone)]
pub struct VecSqliteMemoryStore {
    pool: Pool<SqliteConnectionManager>,
    config: VecSqliteStoreConfig,
}

impl VecSqliteMemoryStore {
    pub async fn from_env() -> Result<Self> {
        Self::new(VecSqliteStoreConfig::from_env()).await
    }

    pub async fn new(config: VecSqliteStoreConfig) -> Result<Self> {
        if let Some(parent) = config.path.parent() {
            fs::create_dir_all(parent).await?;
        }

        Self::register_sqlite_vec_extension()?;

        let manager = SqliteConnectionManager::file(&config.path)
            .with_init(|c| {
                // Enable WAL mode with full optimizations for better concurrency and performance
                c.execute_batch(
                    "PRAGMA journal_mode=WAL; \
                     PRAGMA synchronous=NORMAL; \
                     PRAGMA wal_autocheckpoint=1000; \
                     PRAGMA cache_size=-32768; \
                     PRAGMA mmap_size=268435456; \
                     PRAGMA temp_store=MEMORY; \
                     PRAGMA foreign_keys=ON;",
                ).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

                c.query_row("SELECT vec_version()", [], |row| row.get::<_, String>(0))
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

                Ok(())
            });

        let pool = Pool::builder()
            .max_size(16)
            .build(manager)
            .with_context(|| {
                format!(
                    "failed to create SQLite connection pool for {}",
                    config.path.display()
                )
            })?;

        let conn = pool.get()?;

        // Initialize schema
        Self::init_schema(&conn, config.embedding_dimensions)?;

        Ok(Self {
            pool,
            config,
        })
    }

    fn register_sqlite_vec_extension() -> Result<()> {
        SQLITE_VEC_EXTENSION_INIT
            .get_or_init(|| unsafe {
                type SqliteExtFn = unsafe extern "C" fn(
                    *mut rusqlite::ffi::sqlite3,
                    *mut *mut i8,
                    *const rusqlite::ffi::sqlite3_api_routines,
                ) -> i32;
                let entry: SqliteExtFn =
                    std::mem::transmute(sqlite_vec::sqlite3_vec_init as *const ());
                let rc = sqlite3_auto_extension(Some(entry));
                if rc != 0 {
                    Err(format!(
                        "failed to register sqlite-vec auto extension: {}",
                        rc
                    ))
                } else {
                    Ok(())
                }
            })
            .clone()
            .map_err(anyhow::Error::msg)
    }

    fn init_schema(conn: &Connection, embedding_dimensions: usize) -> Result<()> {
        // Create main memory table (same as SqliteMemoryStore)
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

        Self::ensure_vector_index(conn, embedding_dimensions)?;
        Self::ensure_fts_index(conn)?;

        // Knowledge graph for entity/relationship memory
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS entities (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                entity_type TEXT,
                properties TEXT DEFAULT '{}',
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS relations (
                id TEXT PRIMARY KEY,
                source_id TEXT NOT NULL,
                target_id TEXT NOT NULL,
                relation_type TEXT NOT NULL,
                properties TEXT DEFAULT '{}',
                FOREIGN KEY (source_id) REFERENCES entities(id),
                FOREIGN KEY (target_id) REFERENCES entities(id)
            );

            CREATE INDEX IF NOT EXISTS idx_entities_type ON entities(entity_type);
            CREATE INDEX IF NOT EXISTS idx_relations_source ON relations(source_id);
            CREATE INDEX IF NOT EXISTS idx_relations_target ON relations(target_id);

            CREATE TABLE IF NOT EXISTS memory_entities (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                memory_id TEXT NOT NULL,
                entity_id TEXT NOT NULL,
                relation_type TEXT NOT NULL DEFAULT 'mentions',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (memory_id) REFERENCES memory_records(id),
                FOREIGN KEY (entity_id) REFERENCES entities(id)
            );

            CREATE INDEX IF NOT EXISTS idx_memory_entities_memory ON memory_entities(workspace_id, memory_id);
            CREATE INDEX IF NOT EXISTS idx_memory_entities_entity ON memory_entities(workspace_id, entity_id);
        "#,
        )?;

        // Tamper-evident hash chain (content chaining for integrity verification)
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS memory_chain (
                id TEXT PRIMARY KEY,
                prev_hash TEXT,
                content_hash TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS timeline_events (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                memory_id TEXT NOT NULL,
                agent_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                operation TEXT NOT NULL,
                prev_hash TEXT,
                curr_hash TEXT NOT NULL,
                payload TEXT NOT NULL DEFAULT '{}'
            );

            CREATE INDEX IF NOT EXISTS idx_timeline_events_workspace ON timeline_events(workspace_id, timestamp);
            CREATE INDEX IF NOT EXISTS idx_timeline_events_memory ON timeline_events(workspace_id, memory_id);
        "#,
        )?;

        // Pattern Protocol - verified patterns discovered by agents
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS patterns (
                id TEXT PRIMARY KEY,
                category TEXT NOT NULL,
                pattern TEXT NOT NULL,
                project TEXT NOT NULL,
                discovered_by TEXT NOT NULL,
                confidence REAL NOT NULL DEFAULT 0.5,
                source_file TEXT DEFAULT '',
                source_occurrences INTEGER DEFAULT 0,
                source_snippet TEXT DEFAULT '',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                usage_count INTEGER DEFAULT 0,
                verification TEXT DEFAULT 'pending'
            );

            CREATE INDEX IF NOT EXISTS idx_patterns_project ON patterns(project);
            CREATE INDEX IF NOT EXISTS idx_patterns_category ON patterns(category);
            CREATE INDEX IF NOT EXISTS idx_patterns_confidence ON patterns(confidence);
        "#,
        )?;

        // Security threats log
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

    fn virtual_table_columns(conn: &Connection, table: &str) -> Result<Vec<String>> {
        let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
        let mut columns = Vec::new();
        for column in rows {
            columns.push(column?);
        }
        Ok(columns)
    }

    fn ensure_vector_index(conn: &Connection, embedding_dimensions: usize) -> Result<()> {
        let expected_columns = ["embedding", "id", "workspace_id"];
        let existing_columns = Self::virtual_table_columns(conn, "memory_embeddings")?;
        let needs_rebuild = existing_columns.as_slice() != expected_columns;

        if needs_rebuild {
            conn.execute_batch("DROP TABLE IF EXISTS memory_embeddings;")
                .context("failed to drop stale memory_embeddings virtual table")?;
        }

        let create_vec_table = format!(
            "CREATE VIRTUAL TABLE IF NOT EXISTS memory_embeddings USING vec0( \
             embedding float[{}], \
             id TEXT, \
             workspace_id TEXT \
             )",
            embedding_dimensions
        );
        conn.execute(&create_vec_table, [])
            .context("failed to create memory_embeddings virtual table")?;

        let expected_vectors = conn
            .query_row(
                &format!(
                    "SELECT COUNT(*) FROM {} WHERE length(embedding) > 0",
                    TABLE_MEMORIES
                ),
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or_default()
            .max(0);
        let indexed_vectors = conn
            .query_row("SELECT COUNT(*) FROM memory_embeddings", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap_or_default()
            .max(0);

        if needs_rebuild || indexed_vectors != expected_vectors {
            conn.execute("DELETE FROM memory_embeddings", [])
                .context("failed to clear stale memory_embeddings rows")?;
            Self::rebuild_vector_index(conn)?;
        }

        Ok(())
    }

    fn ensure_fts_index(conn: &Connection) -> Result<()> {
        let expected_columns = ["id", "path", "content", "code_tokens"];
        let existing_columns = Self::virtual_table_columns(conn, "memory_fts")?;
        let needs_rebuild = existing_columns.as_slice() != expected_columns;

        if needs_rebuild {
            conn.execute_batch("DROP TABLE IF EXISTS memory_fts;")
                .context("failed to drop stale memory_fts virtual table")?;
        }

        conn.execute_batch(
            r#"
            CREATE VIRTUAL TABLE IF NOT EXISTS memory_fts USING fts5(
                id UNINDEXED,
                path UNINDEXED,
                content,
                code_tokens,
                tokenize='porter unicode61',
                prefix='2 3 4 5 6 8 10 12'
            );
        "#,
        )?;

        let expected_rows = conn
            .query_row(
                &format!("SELECT COUNT(*) FROM {}", TABLE_MEMORIES),
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or_default()
            .max(0);
        let indexed_rows = conn
            .query_row("SELECT COUNT(*) FROM memory_fts", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap_or_default()
            .max(0);

        if needs_rebuild || indexed_rows != expected_rows {
            conn.execute("DELETE FROM memory_fts", [])
                .context("failed to clear stale memory_fts rows")?;
            Self::rebuild_fts_index(conn)?;
        }

        Ok(())
    }

    fn rebuild_vector_index(conn: &Connection) -> Result<()> {
        let mut stmt = conn.prepare(&format!(
            "SELECT rowid, id, workspace_id, embedding FROM {} WHERE length(embedding) > 0",
            TABLE_MEMORIES
        ))?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Vec<u8>>(3)?,
            ))
        })?;

        for row in rows {
            let (rowid, id, workspace_id, embedding_blob) = row?;
            let embedding = Self::deserialize_embedding(&embedding_blob);
            if embedding.is_empty() {
                continue;
            }

            let embedding_json =
                serde_json::to_string(&embedding).context("failed to serialize embedding")?;
            conn.execute(
                "INSERT INTO memory_embeddings(rowid, embedding, id, workspace_id) VALUES (?, vec_f32(?), ?, ?)",
                params![
                    rowid,
                    format!(
                        "[{}]",
                        embedding_json.trim_start_matches('[').trim_end_matches(']')
                    ),
                    id,
                    workspace_id
                ],
            )
            .context("failed to rebuild memory_embeddings row")?;
        }

        Ok(())
    }

    fn rebuild_fts_index(conn: &Connection) -> Result<()> {
        let mut stmt =
            conn.prepare(&format!("SELECT id, path, content FROM {}", TABLE_MEMORIES))?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;

        for row in rows {
            let (id, path, content) = row?;
            let code_tokens = Self::code_tokens(&format!("{path} {content}")).join(" ");
            conn.execute(
                "INSERT INTO memory_fts(id, path, content, code_tokens) VALUES (?, ?, ?, ?)",
                params![id, path, content, code_tokens],
            )
            .context("failed to rebuild memory_fts row")?;
        }

        Ok(())
    }

    pub fn configured_qjl_threshold() -> usize {
        std::env::var("XAVIER2_QJL_THRESHOLD")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(DEFAULT_QJL_THRESHOLD)
    }

    fn serialize_embedding(embedding: &[f32]) -> Vec<u8> {
        embedding.iter().flat_map(|v| v.to_le_bytes()).collect()
    }

    fn serialize_embedding_qjl(embedding: &[f32]) -> Vec<u8> {
        let dims = embedding.len() as u32;
        let max_abs = embedding
            .iter()
            .fold(0.0_f32, |acc, value| acc.max(value.abs()));
        let scale_1 = if max_abs > 0.0 { max_abs / 127.0 } else { 1.0 };
        let coarse: Vec<i8> = embedding
            .iter()
            .map(|value| ((value / scale_1).round().clamp(-127.0, 127.0)) as i8)
            .collect();
        let residuals: Vec<f32> = embedding
            .iter()
            .zip(coarse.iter())
            .map(|(value, quantized)| value - (*quantized as f32 * scale_1))
            .collect();
        let residual_max = residuals
            .iter()
            .fold(0.0_f32, |acc, value| acc.max(value.abs()));
        let scale_2 = if residual_max > 0.0 {
            residual_max / 127.0
        } else {
            1.0
        };
        let residual_quantized: Vec<i8> = residuals
            .iter()
            .map(|value| ((value / scale_2).round().clamp(-127.0, 127.0)) as i8)
            .collect();

        let mut bytes = Vec::with_capacity(16 + (embedding.len() * 2));
        bytes.extend_from_slice(QJL_MAGIC);
        bytes.extend_from_slice(&dims.to_le_bytes());
        bytes.extend_from_slice(&scale_1.to_le_bytes());
        bytes.extend_from_slice(&scale_2.to_le_bytes());
        bytes.extend(coarse.into_iter().map(|value| value as u8));
        bytes.extend(residual_quantized.into_iter().map(|value| value as u8));
        bytes
    }

    fn deserialize_embedding(data: &[u8]) -> Vec<f32> {
        if data.len() >= 16 && &data[..4] == QJL_MAGIC {
            let dims = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
            let scale_1 = f32::from_le_bytes([data[8], data[9], data[10], data[11]]);
            let scale_2 = f32::from_le_bytes([data[12], data[13], data[14], data[15]]);
            let expected_len = 16 + (dims * 2);
            if data.len() >= expected_len {
                let coarse = &data[16..16 + dims];
                let residual = &data[16 + dims..expected_len];
                return coarse
                    .iter()
                    .zip(residual.iter())
                    .map(|(coarse, residual)| {
                        let coarse = *coarse as i8 as f32;
                        let residual = *residual as i8 as f32;
                        (coarse * scale_1) + (residual * scale_2)
                    })
                    .collect();
            }
        }

        data.chunks_exact(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect()
    }

    fn row_key(workspace_id: &str, memory_id: &str) -> String {
        stable_key("sqlite_mem", &[workspace_id, memory_id])
    }

    fn deserialize_record(row: &rusqlite::Row) -> rusqlite::Result<MemoryRecord> {
        let metadata_str: String = row.get(4)?;
        let revisions_str: String = row.get(11)?;
        let embedding_blob: Vec<u8> = row.get(5)?;

        Ok(MemoryRecord {
            id: row.get(0)?,
            workspace_id: row.get(1)?,
            path: row.get(2)?,
            content: row.get(3)?,
            metadata: serde_json::from_str(&metadata_str).unwrap_or_default(),
            embedding: Self::deserialize_embedding(&embedding_blob),
            created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            revision: row.get::<_, i64>(8)? as u64,
            primary: row.get::<_, i32>(9)? != 0,
            parent_id: row.get(10)?,
            revisions: serde_json::from_str(&revisions_str).unwrap_or_default(),
        })
    }

    fn candidate_limit(limit: usize) -> usize {
        limit.max(1).saturating_mul(5)
    }

    pub fn configured_rrf_k() -> usize {
        std::env::var("XAVIER2_RRF_K")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(DEFAULT_RRF_K)
    }

    fn source_weight(source: FusionSource) -> f32 {
        source.default_weight()
    }

    fn dynamic_rrf_k(dataset_size: usize) -> usize {
        let base = Self::configured_rrf_k();
        if dataset_size <= 1_000 {
            return base;
        }

        base.saturating_add(dataset_size / 1_000)
    }

    pub fn entity_extraction_enabled() -> bool {
        std::env::var("XAVIER2_ENTITY_EXTRACTION_ENABLED")
            .ok()
            .map(|value| {
                !matches!(
                    value.trim().to_ascii_lowercase().as_str(),
                    "0" | "false" | "off"
                )
            })
            .unwrap_or(true)
    }

    pub fn audit_chain_enabled() -> bool {
        std::env::var("XAVIER2_AUDIT_CHAIN_ENABLED")
            .ok()
            .map(|value| {
                !matches!(
                    value.trim().to_ascii_lowercase().as_str(),
                    "0" | "false" | "off"
                )
            })
            .unwrap_or(true)
    }

    fn qjl_enabled_for_workspace(conn: &Connection, workspace_id: &str) -> bool {
        let threshold = Self::configured_qjl_threshold();
        let current_vectors = conn
            .query_row(
                "SELECT COUNT(*) FROM memory_embeddings WHERE workspace_id = ?",
                params![workspace_id],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or_default()
            .max(0) as usize;
        current_vectors >= threshold
    }

    fn search_tokens(query: &str) -> Vec<String> {
        static TOKEN_RE: OnceLock<Regex> = OnceLock::new();
        let re = TOKEN_RE.get_or_init(|| {
            Regex::new(r"[A-Za-z0-9][A-Za-z0-9._:/#-]{1,}").expect("valid search token regex")
        });

        let mut seen = HashSet::new();
        re.find_iter(query)
            .filter_map(|m| {
                let token = m.as_str().trim_matches('"').trim().to_string();
                if token.len() < 2 {
                    return None;
                }
                let lowered = token.to_ascii_lowercase();
                if seen.insert(lowered) {
                    Some(token)
                } else {
                    None
                }
            })
            .collect()
    }

    fn build_fts_query(query: &str) -> Option<String> {
        let mut tokens = Self::search_tokens(query);
        tokens.extend(Self::code_tokens(query));
        if tokens.is_empty() {
            return None;
        }

        Some(
            tokens
                .into_iter()
                .filter_map(|token| {
                    let escaped = token
                        .chars()
                        .filter(|ch| {
                            ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/')
                        })
                        .collect::<String>();
                    if escaped.is_empty() {
                        None
                    } else {
                        Some(format!("{escaped}*"))
                    }
                })
                .collect::<Vec<_>>()
                .join(" OR "),
        )
    }

    fn detect_query_entities(query: &str) -> Vec<String> {
        Self::search_tokens(query)
            .into_iter()
            .filter(|token| {
                token.chars().any(|c| c.is_ascii_digit())
                    || token.contains('-')
                    || token.contains('_')
                    || token.chars().any(|c| c.is_ascii_uppercase())
            })
            .collect()
    }

    fn split_camel_case(token: &str) -> Vec<String> {
        let mut words = Vec::new();
        let mut current = String::new();
        let mut previous_lower = false;
        for ch in token.chars() {
            if !ch.is_ascii_alphanumeric() {
                if !current.is_empty() {
                    words.push(current.clone());
                    current.clear();
                }
                previous_lower = false;
                continue;
            }

            let is_upper = ch.is_ascii_uppercase();
            if is_upper && previous_lower && !current.is_empty() {
                words.push(current.clone());
                current.clear();
            }
            previous_lower = ch.is_ascii_lowercase();
            current.push(ch.to_ascii_lowercase());
        }
        if !current.is_empty() {
            words.push(current);
        }
        words
    }

    fn code_tokens(text: &str) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut expanded = Vec::new();
        for token in Self::search_tokens(text) {
            for segment in token
                .split(|ch: char| ['_', '-', '/', '.', ':'].contains(&ch))
                .filter(|segment| !segment.is_empty())
            {
                for part in Self::split_camel_case(segment) {
                    if part.len() > 1 && seen.insert(part.clone()) {
                        expanded.push(part);
                    }
                }
            }
        }
        expanded
    }

    fn extract_entities(content: &str) -> Vec<ExtractedEntity> {
        static MENTION_RE: OnceLock<Regex> = OnceLock::new();
        static TOPIC_RE: OnceLock<Regex> = OnceLock::new();
        static URL_RE: OnceLock<Regex> = OnceLock::new();
        static DATE_RE: OnceLock<Regex> = OnceLock::new();

        let mention_re =
            MENTION_RE.get_or_init(|| Regex::new(r"@[\w.-]{2,}").expect("valid mention regex"));
        let topic_re =
            TOPIC_RE.get_or_init(|| Regex::new(r"#[\w-]{2,}").expect("valid topic regex"));
        let url_re = URL_RE
            .get_or_init(|| Regex::new(r#"https?://[^\s)>"]+"#).expect("valid url entity regex"));
        let date_re = DATE_RE.get_or_init(|| {
            Regex::new(
                r"\b(\d{4}-\d{2}-\d{2}|\d{4}/\d{2}/\d{2}|(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)[a-z]*\s+\d{1,2},\s+\d{4})\b",
            )
            .expect("valid date entity regex")
        });

        let mut entities = Vec::new();
        let mut seen = HashSet::new();
        for (regex, entity_type, relation_type) in [
            (mention_re, "mention", "mentions"),
            (topic_re, "topic", "tags"),
            (url_re, "url", "references"),
            (date_re, "date", "dated_on"),
        ] {
            for matched in regex.find_iter(content) {
                let value = matched.as_str().trim().to_string();
                let key = format!("{entity_type}:{}", value.to_ascii_lowercase());
                if seen.insert(key) {
                    entities.push(ExtractedEntity {
                        value,
                        entity_type,
                        relation_type,
                    });
                }
            }
        }

        entities
    }

    fn row_matches_filters(
        workspace_id: &str,
        record: &MemoryRecord,
        filters: Option<&MemoryQueryFilters>,
    ) -> bool {
        filters.is_none_or(|filters| {
            crate::memory::schema::resolve_metadata(
                &record.path,
                &record.metadata,
                workspace_id,
                None,
            )
            .map(|resolved| {
                filters
                    .workspace_id
                    .as_deref()
                    .is_none_or(|value| resolved.namespace.workspace_id.as_deref() == Some(value))
                    && filters
                        .project
                        .as_deref()
                        .is_none_or(|value| resolved.namespace.project.as_deref() == Some(value))
                    && filters
                        .scope
                        .as_deref()
                        .is_none_or(|value| resolved.namespace.scope.as_deref() == Some(value))
                    && filters
                        .session_id
                        .as_deref()
                        .is_none_or(|value| resolved.namespace.session_id.as_deref() == Some(value))
            })
            .unwrap_or(false)
        })
    }

    fn merge_rrf_result(
        scored: &mut HashMap<String, HybridSearchResult>,
        source: FusionSource,
        rrf_k: usize,
        rank: usize,
        raw_score: Option<f32>,
        record: MemoryRecord,
    ) {
        let contribution = (1.0 / (rrf_k as f32 + rank as f32)) * Self::source_weight(source);
        scored
            .entry(record.id.clone())
            .and_modify(|existing| {
                existing.score += contribution;
                match source {
                    FusionSource::Vector => existing.vector_score += contribution,
                    FusionSource::Fts => {
                        existing.lexical_score += contribution;
                        if raw_score.is_some() {
                            existing.bm25 = raw_score;
                        }
                    }
                    FusionSource::Kg => existing.kg_score += contribution,
                }
            })
            .or_insert(HybridSearchResult {
                record,
                score: contribution,
                vector_score: if matches!(source, FusionSource::Vector) {
                    contribution
                } else {
                    0.0
                },
                lexical_score: if matches!(source, FusionSource::Fts) {
                    contribution
                } else {
                    0.0
                },
                kg_score: if matches!(source, FusionSource::Kg) {
                    contribution
                } else {
                    0.0
                },
                bm25: raw_score.filter(|_| matches!(source, FusionSource::Fts)),
            });
    }

    fn load_record_by_id(
        conn: &Connection,
        workspace_id: &str,
        memory_id: &str,
    ) -> Result<Option<MemoryRecord>> {
        let mut stmt = conn.prepare(&format!(
            "SELECT id, workspace_id, path, content, metadata, embedding, created_at, updated_at, revision, primary_flag, parent_id, revisions FROM {} WHERE id = ? AND workspace_id = ?",
            TABLE_MEMORIES
        ))?;

        match stmt.query_row(params![memory_id, workspace_id], Self::deserialize_record) {
            Ok(record) => Ok(Some(record)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    fn memory_node_id(workspace_id: &str, memory_id: &str) -> String {
        stable_key("memory_entity", &[workspace_id, memory_id])
    }

    fn entity_node_id(workspace_id: &str, entity_type: &str, value: &str) -> String {
        stable_key(
            "entity",
            &[
                workspace_id,
                entity_type,
                &value.trim().to_ascii_lowercase(),
            ],
        )
    }

    fn sync_memory_entities(
        conn: &Connection,
        workspace_id: &str,
        record: &MemoryRecord,
    ) -> Result<()> {
        if !Self::entity_extraction_enabled() {
            return Ok(());
        }

        let memory_node_id = Self::memory_node_id(workspace_id, &record.id);
        conn.execute(
            "INSERT OR REPLACE INTO entities (id, name, entity_type, properties) VALUES (?, ?, ?, ?)",
            params![
                &memory_node_id,
                &record.path,
                "memory",
                serde_json::json!({
                    "memory_id": record.id,
                    "path": record.path,
                    "workspace_id": workspace_id,
                })
                .to_string()
            ],
        )?;

        conn.execute(
            "DELETE FROM memory_entities WHERE workspace_id = ? AND memory_id = ?",
            params![workspace_id, &record.id],
        )?;
        conn.execute(
            "DELETE FROM relations WHERE source_id = ?",
            params![&memory_node_id],
        )?;

        for entity in Self::extract_entities(&record.content) {
            let entity_id = Self::entity_node_id(workspace_id, entity.entity_type, &entity.value);
            conn.execute(
                "INSERT OR REPLACE INTO entities (id, name, entity_type, properties) VALUES (?, ?, ?, ?)",
                params![
                    &entity_id,
                    &entity.value,
                    entity.entity_type,
                    serde_json::json!({
                        "workspace_id": workspace_id,
                        "normalized": entity.value.to_ascii_lowercase(),
                    })
                    .to_string()
                ],
            )?;
            conn.execute(
                "INSERT OR REPLACE INTO memory_entities (id, workspace_id, memory_id, entity_id, relation_type) VALUES (?, ?, ?, ?, ?)",
                params![
                    stable_key("memory_entity_link", &[workspace_id, &record.id, &entity_id]),
                    workspace_id,
                    &record.id,
                    &entity_id,
                    entity.relation_type,
                ],
            )?;
            conn.execute(
                "INSERT OR REPLACE INTO relations (id, source_id, target_id, relation_type, properties) VALUES (?, ?, ?, ?, ?)",
                params![
                    stable_key("memory_relation", &[workspace_id, &memory_node_id, &entity_id, entity.relation_type]),
                    &memory_node_id,
                    &entity_id,
                    entity.relation_type,
                    serde_json::json!({
                        "memory_id": record.id,
                        "path": record.path,
                        "entity_type": entity.entity_type,
                    })
                    .to_string()
                ],
            )?;
        }

        Ok(())
    }

    fn append_timeline_event(
        conn: &Connection,
        workspace_id: &str,
        record: &MemoryRecord,
    ) -> Result<()> {
        if !Self::audit_chain_enabled() {
            return Ok(());
        }

        let (previous_event_id, previous_hash): (Option<String>, Option<String>) = conn
            .query_row(
                "SELECT id, curr_hash FROM timeline_events WHERE workspace_id = ? ORDER BY timestamp DESC LIMIT 1",
                params![workspace_id],
                |row| Ok((row.get(0).ok(), row.get(1).ok())),
            )
            .unwrap_or((None, None));
        let event_id = ulid::Ulid::new().to_string();
        let timestamp = chrono::Utc::now().to_rfc3339();
        let agent_id = record
            .metadata
            .get("_audit")
            .and_then(|value| value.get("agent_id"))
            .and_then(|value| value.as_str())
            .or_else(|| {
                record
                    .metadata
                    .get("agent_id")
                    .and_then(|value| value.as_str())
            })
            .unwrap_or("system")
            .to_string();
        let operation = record
            .metadata
            .get("_audit")
            .and_then(|value| value.get("operation"))
            .and_then(|value| value.as_str())
            .unwrap_or("memory.add")
            .to_string();
        let content_hash = format!("{:x}", Sha256::digest(record.content.as_bytes()));
        let curr_hash = format!(
            "{:x}",
            Sha256::digest(
                format!(
                    "{}|{}|{}|{}|{}|{}|{}",
                    previous_hash.clone().unwrap_or_default(),
                    event_id,
                    record.id,
                    agent_id,
                    timestamp,
                    operation,
                    content_hash
                )
                .as_bytes()
            )
        );

        let event = TimelineEventRecord {
            id: event_id,
            agent_id,
            timestamp,
            operation,
            prev_hash: previous_hash,
            curr_hash,
        };

        conn.execute(
            "INSERT INTO timeline_events (id, workspace_id, memory_id, agent_id, timestamp, operation, prev_hash, curr_hash, payload)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                &event.id,
                workspace_id,
                &record.id,
                &event.agent_id,
                &event.timestamp,
                &event.operation,
                &event.prev_hash,
                &event.curr_hash,
                serde_json::json!({
                    "path": record.path,
                    "revision": record.revision,
                })
                .to_string()
            ],
        )?;

        let memory_node_id = Self::memory_node_id(workspace_id, &record.id);
        let event_node_id = stable_key("timeline_event_node", &[workspace_id, &event.id]);
        conn.execute(
            "INSERT OR REPLACE INTO entities (id, name, entity_type, properties) VALUES (?, ?, ?, ?)",
            params![
                &event_node_id,
                format!("{} {}", event.operation, record.path),
                "timeline_event",
                serde_json::json!({
                    "event_id": event.id,
                    "memory_id": record.id,
                    "agent_id": event.agent_id,
                    "timestamp": event.timestamp,
                    "operation": event.operation,
                    "prev_hash": event.prev_hash,
                    "curr_hash": event.curr_hash,
                })
                .to_string()
            ],
        )?;
        conn.execute(
            "INSERT OR REPLACE INTO relations (id, source_id, target_id, relation_type, properties) VALUES (?, ?, ?, ?, ?)",
            params![
                stable_key("timeline_memory_relation", &[workspace_id, &event_node_id, &memory_node_id]),
                &event_node_id,
                &memory_node_id,
                "operation_on",
                serde_json::json!({"operation": event.operation}).to_string()
            ],
        )?;

        if let Some(previous_event_id) = previous_event_id {
            let previous_event_node_id =
                stable_key("timeline_event_node", &[workspace_id, &previous_event_id]);
            conn.execute(
                "INSERT OR REPLACE INTO relations (id, source_id, target_id, relation_type, properties) VALUES (?, ?, ?, ?, ?)",
                params![
                    stable_key("timeline_prev_relation", &[workspace_id, &previous_event_node_id, &event_node_id]),
                    &previous_event_node_id,
                    &event_node_id,
                    "precedes",
                    serde_json::json!({"chain": "timeline"}).to_string()
                ],
            )?;
        }

        Ok(())
    }

    fn resolve_graph_seed_entities(
        conn: &Connection,
        workspace_id: &str,
        source: &MemoryRecord,
        query: &str,
    ) -> Result<Vec<String>> {
        let mut seed_ids = Vec::new();
        let mut seen = HashSet::new();

        let mut link_stmt = conn.prepare(
            "SELECT entity_id FROM memory_entities WHERE workspace_id = ? AND memory_id = ?",
        )?;
        let mut link_rows = link_stmt.query(params![workspace_id, &source.id])?;
        while let Some(row) = link_rows.next()? {
            let entity_id: String = row.get(0)?;
            if seen.insert(entity_id.clone()) {
                seed_ids.push(entity_id);
            }
        }

        let mut terms = Self::detect_query_entities(query);
        if terms.is_empty() {
            terms = Self::detect_query_entities(&source.content);
        }

        if !terms.is_empty() {
            let mut entity_stmt = conn.prepare(
                "SELECT id FROM entities WHERE lower(name) LIKE lower(?) ORDER BY created_at DESC LIMIT 3",
            )?;
            for term in terms {
                let mut entity_rows = entity_stmt.query(params![format!("%{term}%")])?;
                while let Some(row) = entity_rows.next()? {
                    let entity_id: String = row.get(0)?;
                    if seen.insert(entity_id.clone()) {
                        seed_ids.push(entity_id);
                    }
                }
            }
        }

        let memory_node_id = Self::memory_node_id(workspace_id, &source.id);
        if seen.insert(memory_node_id.clone()) {
            seed_ids.push(memory_node_id);
        }

        Ok(seed_ids)
    }

    /// Insert or update a vector in the sqlite-vec virtual table
    fn upsert_vector(&self, memory_id: &str, workspace_id: &str, embedding: &[f32]) -> Result<()> {
        let conn = self.pool.get()?;
        let embedding_json =
            serde_json::to_string(embedding).context("failed to serialize embedding")?;

        // Delete existing vector first
        conn.execute(
            "DELETE FROM memory_embeddings WHERE id = ? AND workspace_id = ?",
            params![memory_id, workspace_id],
        )?;

        // Insert new vector using vec0's API
        // The rowid will be used to join back to memory_records
        conn.execute(
            "INSERT INTO memory_embeddings(rowid, embedding, id, workspace_id) \
             VALUES ( \
               (SELECT rowid FROM memory_records WHERE id = ? AND workspace_id = ?), \
               vec_f32(?), \
               ?, \
               ? \
             )",
            params![
                memory_id,
                workspace_id,
                format!(
                    "[{}]",
                    embedding_json.trim_start_matches('[').trim_end_matches(']')
                ),
                memory_id,
                workspace_id
            ],
        )?;

        Ok(())
    }

    /// Search using RRF (Reciprocal Rank Fusion) combining vector, FTS5, and KG signals.
    pub async fn hybrid_search_with_embedding(
        &self,
        workspace_id: &str,
        query: &str,
        mode: HybridSearchMode,
        query_embedding: Option<&[f32]>,
        filters: Option<&MemoryQueryFilters>,
        limit: usize,
    ) -> Result<Vec<HybridSearchResult>> {
        let trimmed_query = query.trim();
        let candidate_limit = Self::candidate_limit(limit).min(100);
        let include_vector = matches!(mode, HybridSearchMode::Vector | HybridSearchMode::Both);
        let include_text = matches!(mode, HybridSearchMode::Text | HybridSearchMode::Both);
        let mut scored: HashMap<String, HybridSearchResult> = HashMap::new();

        {
            let conn = self.pool.get()?;
            let dataset_size = conn
                .query_row(
                    &format!(
                        "SELECT COUNT(*) FROM {} WHERE workspace_id = ?",
                        TABLE_MEMORIES
                    ),
                    params![workspace_id],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap_or_default()
                .max(0) as usize;
            let rrf_k = Self::dynamic_rrf_k(dataset_size);

            if include_vector {
                if let Some(embedding) = query_embedding {
                    if !embedding.is_empty() && embedding.len() == self.config.embedding_dimensions
                    {
                        let embedding_str = format!(
                            "[{}]",
                            embedding
                                .iter()
                                .map(|v| v.to_string())
                                .collect::<Vec<_>>()
                                .join(",")
                        );

                        let vec_sql = r#"
                            SELECT m.id, m.workspace_id, m.path, m.content, m.metadata, m.embedding,
                                   m.created_at, m.updated_at, m.revision, m.primary_flag,
                                   m.parent_id, m.revisions, ve.distance
                            FROM memory_embeddings ve
                            JOIN memory_records m ON m.id = ve.id AND m.workspace_id = ?
                            WHERE ve.embedding MATCH vec_f32(?)
                              AND k = ?
                            ORDER BY distance
                            LIMIT ?
                        "#;

                        let mut stmt = conn
                            .prepare(vec_sql)
                            .context("vector search prepare failed")?;
                        let mut rows = stmt.query(params![
                            workspace_id,
                            &embedding_str,
                            candidate_limit as i64,
                            candidate_limit as i64
                        ])?;

                        let mut rank = 0usize;
                        while let Some(row) = rows.next()? {
                            if let Ok(record) = Self::deserialize_record(row) {
                                if Self::row_matches_filters(workspace_id, &record, filters) {
                                    rank += 1;
                                    Self::merge_rrf_result(
                                        &mut scored,
                                        FusionSource::Vector,
                                        rrf_k,
                                        rank,
                                        None,
                                        record,
                                    );
                                }
                            }
                        }
                    }
                }
            }

            if include_text && !trimmed_query.is_empty() {
                if let Some(fts_query) = Self::build_fts_query(trimmed_query) {
                    let fts_sql = r#"
                        SELECT m.id, m.workspace_id, m.path, m.content, m.metadata, m.embedding,
                               m.created_at, m.updated_at, m.revision, m.primary_flag,
                               m.parent_id, m.revisions, bm25(f, 1.0, 0.8) AS rank
                        FROM memory_fts f
                        JOIN memory_records m ON m.id = f.id AND m.workspace_id = ?
                        WHERE f.memory_fts MATCH ?
                        ORDER BY rank
                        LIMIT ?
                    "#;

                    if let Ok(mut stmt) = conn.prepare(fts_sql) {
                        if let Ok(mut rows) =
                            stmt.query(params![workspace_id, fts_query, candidate_limit as i64])
                        {
                            let mut rank = 0usize;
                            while let Some(row) = rows.next()? {
                                let bm25_score = row.get::<_, f32>(12).ok();
                                if let Ok(record) = Self::deserialize_record(row) {
                                    if Self::row_matches_filters(workspace_id, &record, filters) {
                                        rank += 1;
                                        Self::merge_rrf_result(
                                            &mut scored,
                                            FusionSource::Fts,
                                            rrf_k,
                                            rank,
                                            bm25_score,
                                            record,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }

                let entity_terms = Self::detect_query_entities(trimmed_query);
                if !entity_terms.is_empty() {
                    let belief_key = stable_key("belief_row", &[workspace_id]);
                    let beliefs_json = conn
                        .query_row(
                            &format!("SELECT beliefs FROM {} WHERE id = ?", TABLE_BELIEFS),
                            params![belief_key],
                            |row| row.get::<_, String>(0),
                        )
                        .ok();

                    if let Some(payload) = beliefs_json {
                        let beliefs: Vec<BeliefRelation> =
                            serde_json::from_str(&payload).unwrap_or_default();
                        let mut kg_rank = 0usize;
                        let mut seen_ids = HashSet::new();

                        for belief in beliefs.into_iter().filter(|belief| {
                            let haystack = format!(
                                "{} {} {}",
                                belief.source, belief.relation_type, belief.target
                            )
                            .to_ascii_lowercase();
                            entity_terms
                                .iter()
                                .any(|term| haystack.contains(&term.to_ascii_lowercase()))
                        }) {
                            if let Some(memory_id) = belief.source_memory_id.as_deref() {
                                if seen_ids.insert(memory_id.to_string()) {
                                    if let Some(record) =
                                        Self::load_record_by_id(&conn, workspace_id, memory_id)?
                                    {
                                        if Self::row_matches_filters(workspace_id, &record, filters)
                                        {
                                            kg_rank += 1;
                                            Self::merge_rrf_result(
                                                &mut scored,
                                                FusionSource::Kg,
                                                rrf_k,
                                                kg_rank,
                                                None,
                                                record,
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let mut results: Vec<_> = scored.into_values().collect();
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        if results.is_empty() && include_text && !trimmed_query.is_empty() {
            return Ok(filter_records(
                self.list(workspace_id).await?,
                workspace_id,
                trimmed_query,
                filters,
            )?
            .into_iter()
            .take(limit)
            .enumerate()
            .map(|(idx, record)| HybridSearchResult {
                record,
                score: 1.0 / (Self::configured_rrf_k() as f32 + (idx + 1) as f32),
                vector_score: 0.0,
                lexical_score: 0.0,
                kg_score: 0.0,
                bm25: None,
            })
            .collect());
        }

        results.truncate(limit);
        Ok(results)
    }

    /// Multi-hop traversal from a memory node using recursive CTE expansion over the KG.
    pub async fn graph_hops(
        &self,
        workspace_id: &str,
        path_or_id: &str,
        max_hops: usize,
        query: &str,
    ) -> Result<GraphHopResult> {
        let source = self
            .get(workspace_id, path_or_id)
            .await?
            .with_context(|| format!("memory not found for graph traversal: {path_or_id}"))?;
        let conn = self.pool.get()?;
        let seed_ids = Self::resolve_graph_seed_entities(&conn, workspace_id, &source, query)?;

        if seed_ids.is_empty() {
            return Ok(GraphHopResult {
                source,
                hops: max_hops,
                query: query.to_string(),
                paths: Vec::new(),
            });
        }

        let placeholders = std::iter::repeat_n("?", seed_ids.len())
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            r#"
            WITH RECURSIVE graph_walk(root_id, current_id, current_name, depth, entity_path, relation_path) AS (
                SELECT e.id, e.id, e.name, 0, e.name, ''
                FROM entities e
                WHERE e.id IN ({placeholders})
                UNION ALL
                SELECT
                    graph_walk.root_id,
                    r.target_id,
                    target.name,
                    graph_walk.depth + 1,
                    graph_walk.entity_path || ' -> ' || target.name,
                    CASE
                        WHEN graph_walk.relation_path = '' THEN r.relation_type
                        ELSE graph_walk.relation_path || ' -> ' || r.relation_type
                    END
                FROM graph_walk
                JOIN relations r ON r.source_id = graph_walk.current_id
                JOIN entities target ON target.id = r.target_id
                WHERE graph_walk.depth < ?
                  AND instr(graph_walk.entity_path, target.name) = 0
            )
            SELECT current_id, current_name, depth, entity_path, relation_path
            FROM graph_walk
            WHERE depth > 0
            ORDER BY depth, entity_path
            "#
        );

        let mut params_vec: Vec<rusqlite::types::Value> = seed_ids
            .into_iter()
            .map(rusqlite::types::Value::from)
            .collect();
        params_vec.push(rusqlite::types::Value::from(max_hops as i64));
        let mut stmt = conn.prepare(&sql).context("graph_hops prepare failed")?;
        let mut rows = stmt.query(rusqlite::params_from_iter(params_vec))?;
        let mut paths = Vec::new();

        while let Some(row) = rows.next()? {
            let entity_id: String = row.get(0)?;
            let entity_name: String = row.get(1)?;
            let depth = row.get::<_, i64>(2)?.max(0) as usize;
            let entity_path: String = row.get(3)?;
            let relation_path: String = row.get(4)?;

            let mut hit_stmt = conn.prepare(&format!(
                "SELECT id, workspace_id, path, content, metadata, embedding, created_at, updated_at, revision, primary_flag, parent_id, revisions
                 FROM {}
                 WHERE workspace_id = ?
                   AND content LIKE '%' || ? || '%'
                 ORDER BY updated_at DESC
                 LIMIT 3",
                TABLE_MEMORIES
            ))?;
            let mut hit_rows = hit_stmt.query(params![workspace_id, &entity_name])?;
            let mut memory_hits = Vec::new();
            while let Some(hit_row) = hit_rows.next()? {
                if let Ok(record) = Self::deserialize_record(hit_row) {
                    if record.id != source.id {
                        memory_hits.push(record);
                    }
                }
            }

            paths.push(GraphHopPath {
                entity_id,
                entity_name,
                depth,
                entity_path,
                relation_path,
                memory_hits,
            });
        }

        Ok(GraphHopResult {
            source,
            hops: max_hops,
            query: query.to_string(),
            paths,
        })
    }

    /// Add an entity to the knowledge graph
    pub async fn add_entity(&self, id: &str, name: &str, entity_type: &str) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT OR REPLACE INTO entities (id, name, entity_type) VALUES (?, ?, ?)",
            params![id, name, entity_type],
        )?;
        Ok(())
    }

    /// Add a relation to the knowledge graph
    pub async fn add_relation(
        &self,
        id: &str,
        source_id: &str,
        target_id: &str,
        relation_type: &str,
    ) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT OR REPLACE INTO relations (id, source_id, target_id, relation_type) VALUES (?, ?, ?, ?)",
            params![id, source_id, target_id, relation_type],
        )?;
        Ok(())
    }
}

#[async_trait]
impl MemoryStore for VecSqliteMemoryStore {
    fn backend(&self) -> MemoryBackend {
        MemoryBackend::Vec
    }

    async fn health(&self) -> Result<String> {
        let conn = self.pool.get()?;
        conn.execute("SELECT 1", [])?;
        Ok(format!("vecsqlite {}", self.config.detail()))
    }

    async fn put(&self, record: MemoryRecord) -> Result<()> {
        // Compute content hash for tamper-evident hash chain
        let content_hash = format!("{:x}", Sha256::digest(record.content.as_bytes()));

        // Get the previous hash for chain linking
        let prev_hash: Option<String> = {
            let conn = self.pool.get()?;
            conn.query_row(
                "SELECT content_hash FROM memory_chain ORDER BY created_at DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .ok()
        };

        // Store in main table first
        {
            let conn = self.pool.get()?;
            let embedding_blob = if !record.embedding.is_empty()
                && Self::qjl_enabled_for_workspace(&conn, &record.workspace_id)
            {
                Self::serialize_embedding_qjl(&record.embedding)
            } else {
                Self::serialize_embedding(&record.embedding)
            };
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
                    embedding_blob,
                    record.created_at.to_rfc3339(),
                    record.updated_at.to_rfc3339(),
                    record.revision as i64,
                    record.primary as i32,
                    record.parent_id,
                    serde_json::to_string(&record.revisions).unwrap_or_default(),
                ],
            )?;

            // Sync to FTS5
            conn.execute("DELETE FROM memory_fts WHERE id = ?", params![&record.id])?;
            let code_tokens =
                Self::code_tokens(&format!("{} {}", &record.path, &record.content)).join(" ");
            conn.execute(
                "INSERT INTO memory_fts(id, path, content, code_tokens) VALUES (?, ?, ?, ?)",
                params![&record.id, &record.path, &record.content, code_tokens],
            )?;

            Self::sync_memory_entities(&conn, &record.workspace_id, &record)?;

            // Add to hash chain
            let chain_id = ulid::Ulid::new().to_string();
            conn.execute(
                "INSERT INTO memory_chain (id, prev_hash, content_hash) VALUES (?, ?, ?)",
                params![chain_id, prev_hash, content_hash],
            )?;
            Self::append_timeline_event(&conn, &record.workspace_id, &record)?;
        }

        // Store vector in sqlite-vec virtual table
        if !record.embedding.is_empty() {
            self.upsert_vector(&record.id, &record.workspace_id, &record.embedding)?;
        }

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
            crate::memory::surreal_store::revisioned_record(existing, record)
        } else if let Some(existing) = self.get(&record.workspace_id, &record.path).await? {
            crate::memory::surreal_store::revisioned_record(existing, record)
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

            // Delete from vector table
            conn.execute(
                "DELETE FROM memory_embeddings WHERE id = ? AND workspace_id = ?",
                params![&record.id, workspace_id],
            )?;

            // Delete from FTS5
            conn.execute("DELETE FROM memory_fts WHERE id = ?", params![&record.id])?;

            let memory_node_id = Self::memory_node_id(workspace_id, &record.id);
            conn.execute(
                "DELETE FROM memory_entities WHERE workspace_id = ? AND memory_id = ?",
                params![workspace_id, &record.id],
            )?;
            conn.execute(
                "DELETE FROM relations WHERE source_id = ?",
                params![&memory_node_id],
            )?;
            conn.execute(
                "DELETE FROM entities WHERE id = ?",
                params![&memory_node_id],
            )?;

            // Delete entities/relations linked to this memory (by name match)
            let pattern = format!("%{}%", record.content);
            conn.execute(
                "DELETE FROM relations WHERE source_id IN (SELECT id FROM entities WHERE name LIKE ?)",
                params![&pattern],
            )?;
            conn.execute(
                "DELETE FROM relations WHERE target_id IN (SELECT id FROM entities WHERE name LIKE ?)",
                params![&pattern],
            )?;
            conn.execute("DELETE FROM entities WHERE name LIKE ?", params![&pattern])?;
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
        // Try to embed the query for vector search
        let query_embedding = if std::env::var("XAVIER2_EMBEDDING_URL").is_ok() {
            match EmbeddingClient::from_env() {
                Ok(client) => client.embed(query).await.ok(),
                Err(_) => None,
            }
        } else {
            None
        };

        Ok(self
            .hybrid_search_with_embedding(
                workspace_id,
                query,
                HybridSearchMode::Both,
                query_embedding.as_deref(),
                filters,
                20,
            )
            .await?
            .into_iter()
            .map(|result| result.record)
            .collect())
    }

    async fn hybrid_search(
        &self,
        workspace_id: &str,
        query: &str,
        mode: HybridSearchMode,
        filters: Option<&MemoryQueryFilters>,
        limit: usize,
    ) -> Result<Vec<HybridSearchResult>> {
        let query_embedding = if matches!(mode, HybridSearchMode::Vector | HybridSearchMode::Both)
            && std::env::var("XAVIER2_EMBEDDING_URL").is_ok()
        {
            match EmbeddingClient::from_env() {
                Ok(client) => client.embed(query).await.ok(),
                Err(_) => None,
            }
        } else {
            None
        };

        self.hybrid_search_with_embedding(
            workspace_id,
            query,
            mode,
            query_embedding.as_deref(),
            filters,
            limit,
        )
        .await
    }

    async fn graph_hops(
        &self,
        workspace_id: &str,
        path_or_id: &str,
        hops: usize,
        query: &str,
    ) -> Result<GraphHopResult> {
        VecSqliteMemoryStore::graph_hops(self, workspace_id, path_or_id, hops, query).await
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
        let now = chrono::Utc::now();
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
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    expires_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
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
            params![
                belief_key,
                workspace_id,
                beliefs_json,
                chrono::Utc::now().to_rfc3339()
            ],
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
            params![workspace_id, chrono::Utc::now().to_rfc3339()],
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
        let now = chrono::Utc::now().to_rfc3339();

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
            params![
                checkpoint_key,
                workspace_id,
                checkpoint.task_id,
                checkpoint.name,
                data_json
            ],
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

// Re-export filter_records from surreal_store for use in hybrid search
fn filter_records(
    records: Vec<MemoryRecord>,
    workspace_id: &str,
    query: &str,
    filters: Option<&MemoryQueryFilters>,
) -> Result<Vec<MemoryRecord>> {
    crate::memory::surreal_store::filter_records(records, workspace_id, query, filters)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn sqlite_vec_extension_is_active_on_new_connections() {
        VecSqliteMemoryStore::register_sqlite_vec_extension().unwrap();

        let conn = Connection::open_in_memory().unwrap();
        let version: String = conn
            .query_row("SELECT vec_version()", [], |row| row.get(0))
            .unwrap();

        assert!(version.starts_with('v'));
    }

    fn test_record(
        workspace_id: &str,
        path: &str,
        content: &str,
        embedding: Vec<f32>,
    ) -> MemoryRecord {
        MemoryRecord {
            id: stable_key("memory", &[workspace_id, path]),
            workspace_id: workspace_id.to_string(),
            path: path.to_string(),
            content: content.to_string(),
            metadata: serde_json::json!({}),
            embedding,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            revision: 1,
            primary: true,
            parent_id: None,
            revisions: Vec::new(),
        }
    }

    #[test]
    fn dynamic_rrf_k_scales_with_dataset_size() {
        assert_eq!(VecSqliteMemoryStore::dynamic_rrf_k(100), DEFAULT_RRF_K);
        assert_eq!(
            VecSqliteMemoryStore::dynamic_rrf_k(5_000),
            DEFAULT_RRF_K + 5
        );
    }

    #[tokio::test]
    async fn hybrid_search_rrf_fuses_vector_fts_and_belief_signals() {
        let temp = tempdir().unwrap();
        let store = VecSqliteMemoryStore::new(VecSqliteStoreConfig {
            path: temp.path().join("rrf.db"),
            embedding_dimensions: 3,
        })
        .await
        .unwrap();

        let workspace_id = "ws-hybrid";
        let lexical_winner = test_record(
            workspace_id,
            "memory/account-renewal",
            "Customer account ACCT-9F3A renewal approved by Alice Johnson.",
            vec![0.0, 1.0, 0.0],
        );
        let semantic_distractor = test_record(
            workspace_id,
            "memory/renewal-summary",
            "Enterprise renewal planning notes for the customer account.",
            vec![1.0, 0.0, 0.0],
        );

        store.put(lexical_winner.clone()).await.unwrap();
        store.put(semantic_distractor.clone()).await.unwrap();
        store
            .save_beliefs(
                workspace_id,
                vec![BeliefRelation {
                    id: ulid::Ulid::new().to_string(),
                    source: "ACCT-9F3A".to_string(),
                    target: "Alice Johnson".to_string(),
                    relation_type: "approved_by".to_string(),
                    weight: 0.9,
                    confidence: 0.9,
                    source_memory_id: Some(lexical_winner.id.clone()),
                    valid_from: None,
                    valid_until: None,
                    superseded_by: None,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                }],
            )
            .await
            .unwrap();

        let results = store
            .hybrid_search_with_embedding(
                workspace_id,
                "ACCT-9F3A renewal",
                HybridSearchMode::Both,
                Some(&[1.0, 0.0, 0.0]),
                None,
                5,
            )
            .await
            .unwrap();

        assert_eq!(
            results.first().map(|entry| entry.record.id.as_str()),
            Some(lexical_winner.id.as_str())
        );
        assert!(results.len() >= 2);
        assert!(results[0].score > results[1].score);
    }

    #[tokio::test]
    async fn graph_hops_returns_recursive_paths_for_memory() {
        let temp = tempdir().unwrap();
        let store = VecSqliteMemoryStore::new(VecSqliteStoreConfig {
            path: temp.path().join("graph-hops.db"),
            embedding_dimensions: 3,
        })
        .await
        .unwrap();

        let workspace_id = "ws-graph";
        let source = test_record(
            workspace_id,
            "memory/decision",
            "ACCT-9F3A was approved by Alice Johnson in Q1.",
            vec![0.0, 1.0, 0.0],
        );
        let target = test_record(
            workspace_id,
            "memory/follow-up",
            "Alice Johnson briefed Finance Committee about the renewal.",
            vec![1.0, 0.0, 0.0],
        );

        store.put(source.clone()).await.unwrap();
        store.put(target).await.unwrap();
        store
            .add_entity(
                &VecSqliteMemoryStore::memory_node_id(workspace_id, &source.id),
                &source.path,
                "memory",
            )
            .await
            .unwrap();
        store
            .add_entity("acct", "ACCT-9F3A", "account")
            .await
            .unwrap();
        store
            .add_entity("alice", "Alice Johnson", "person")
            .await
            .unwrap();
        store
            .add_entity("finance", "Finance Committee", "team")
            .await
            .unwrap();
        store
            .add_relation(
                &ulid::Ulid::new().to_string(),
                &VecSqliteMemoryStore::memory_node_id(workspace_id, &source.id),
                "acct",
                "mentions",
            )
            .await
            .unwrap();
        store
            .add_relation(
                &ulid::Ulid::new().to_string(),
                "acct",
                "alice",
                "approved_by",
            )
            .await
            .unwrap();
        store
            .add_relation(
                &ulid::Ulid::new().to_string(),
                "alice",
                "finance",
                "briefed",
            )
            .await
            .unwrap();

        let result = store
            .graph_hops(workspace_id, &source.path, 3, "ACCT-9F3A")
            .await
            .unwrap();

        assert_eq!(result.source.id, source.id);
        assert!(result
            .paths
            .iter()
            .any(|path| path.entity_name == "Alice Johnson"));
        assert!(result
            .paths
            .iter()
            .any(|path| path.entity_path.contains("Finance Committee")));
    }

    #[tokio::test]
    async fn put_extracts_entities_and_links_memory_nodes() {
        let temp = tempdir().unwrap();
        let store = VecSqliteMemoryStore::new(VecSqliteStoreConfig {
            path: temp.path().join("entities.db"),
            embedding_dimensions: 3,
        })
        .await
        .unwrap();

        let workspace_id = "ws-entities";
        let record = test_record(
            workspace_id,
            "memory/entities",
            "Ping @alice about #rrf before 2026-04-13 and review https://example.com/spec",
            vec![0.0, 1.0, 0.0],
        );

        store.put(record.clone()).await.unwrap();

        let conn = store.pool.get().unwrap();
        let link_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM memory_entities WHERE workspace_id = ? AND memory_id = ?",
                params![workspace_id, &record.id],
                |row| row.get::<_, i64>(0),
            )
            .unwrap();

        assert!(link_count >= 4);
    }

    #[tokio::test]
    async fn new_rebuilds_stale_fts_schema_without_code_tokens() {
        let temp = tempdir().unwrap();
        let db_path = temp.path().join("stale-fts.db");
        VecSqliteMemoryStore::register_sqlite_vec_extension().unwrap();

        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute_batch(&format!(
                r#"
                CREATE TABLE memory_records (
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
                CREATE VIRTUAL TABLE memory_fts USING fts5(
                    id UNINDEXED,
                    path UNINDEXED,
                    content
                );
                "#
            ))
            .unwrap();
        }

        let store = VecSqliteMemoryStore::new(VecSqliteStoreConfig {
            path: db_path,
            embedding_dimensions: 3,
        })
        .await
        .unwrap();

        let record = test_record(
            "ws-stale",
            "memory/stale",
            "schema rebuild should restore code tokens",
            vec![1.0, 0.0, 0.0],
        );

        store.put(record.clone()).await.unwrap();

        let conn = store.pool.get().unwrap();
        let columns = VecSqliteMemoryStore::virtual_table_columns(&conn, "memory_fts").unwrap();
        assert!(columns.iter().any(|column| column == "code_tokens"));

        let indexed: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM memory_fts WHERE id = ?",
                params![record.id],
                |row| row.get::<_, i64>(0),
            )
            .unwrap();
        assert_eq!(indexed, 1);
    }

    #[tokio::test]
    async fn put_appends_auditable_timeline_events() {
        let temp = tempdir().unwrap();
        let store = VecSqliteMemoryStore::new(VecSqliteStoreConfig {
            path: temp.path().join("timeline.db"),
            embedding_dimensions: 3,
        })
        .await
        .unwrap();

        let workspace_id = "ws-timeline";
        let mut record = test_record(
            workspace_id,
            "memory/audit",
            "auditable event content",
            vec![0.0, 1.0, 0.0],
        );
        record.metadata = serde_json::json!({
            "_audit": {
                "agent_id": "http",
                "operation": "memory.add"
            }
        });

        store.put(record.clone()).await.unwrap();
        record.path = "memory/audit-2".to_string();
        record.id = stable_key("memory", &[workspace_id, &record.path]);
        store.put(record.clone()).await.unwrap();

        let conn = store.pool.get().unwrap();
        let event_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM timeline_events WHERE workspace_id = ?",
                params![workspace_id],
                |row| row.get::<_, i64>(0),
            )
            .unwrap();
        let chained_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM timeline_events WHERE workspace_id = ? AND prev_hash IS NOT NULL",
                params![workspace_id],
                |row| row.get::<_, i64>(0),
            )
            .unwrap();

        assert_eq!(event_count, 2);
        assert_eq!(chained_count, 1);
    }

    #[test]
    fn qjl_roundtrip_preserves_embedding_shape() {
        let original = vec![0.5, -1.25, 3.75, 0.0, 8.0];
        let encoded = VecSqliteMemoryStore::serialize_embedding_qjl(&original);
        let decoded = VecSqliteMemoryStore::deserialize_embedding(&encoded);

        assert_eq!(decoded.len(), original.len());
        assert!(decoded
            .iter()
            .zip(original.iter())
            .all(|(a, b)| (a - b).abs() < 0.2));
    }
}
