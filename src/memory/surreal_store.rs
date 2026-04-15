use std::{collections::HashMap, path::PathBuf, sync::Arc};

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use surrealdb::engine::any::{connect, Any};
use surrealdb::opt::auth::Root;
use tokio::{fs, sync::RwLock};

use crate::{
    checkpoint::Checkpoint,
    memory::{
        belief_graph::BeliefRelation,
        qmd_memory::MemoryDocument,
        schema::{resolve_metadata, MemoryQueryFilters},
    },
    utils::crypto::hex_encode,
};
use surrealdb::Surreal;
use surrealdb_types::SurrealValue;

pub(crate) const TABLE_MEMORIES: &str = "memory_records";
pub(crate) const TABLE_BELIEFS: &str = "belief_states";
pub(crate) const TABLE_SESSION_TOKENS: &str = "session_tokens";
pub(crate) const TABLE_CHECKPOINTS: &str = "checkpoint_records";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryBackend {
    File,
    Memory,
    Surreal,
    Sqlite,
    Vec, // SQLite + sqlite-vec vector search (HNSW-like ANN)
}

impl MemoryBackend {
    pub fn from_env(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "memory" => Self::Memory,
            "surreal" => Self::Surreal,
            "sqlite" => Self::Sqlite,
            "vec" | "sqlite-vec" => Self::Vec,
            "file" => Self::File,
            _ => Self::File,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Memory => "memory",
            Self::Surreal => "surreal",
            Self::Sqlite => "sqlite",
            Self::Vec => "vec",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
pub struct MemoryRevision {
    pub revision: u64,
    pub recorded_at: DateTime<Utc>,
    pub path: String,
    pub content: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
pub struct MemoryRecord {
    pub id: String,
    pub workspace_id: String,
    pub path: String,
    pub content: String,
    pub metadata: serde_json::Value,
    pub embedding: Vec<f32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub revision: u64,
    pub primary: bool,
    pub parent_id: Option<String>,
    #[serde(default)]
    pub revisions: Vec<MemoryRevision>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HybridSearchMode {
    Text,
    Vector,
    #[default]
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridSearchResult {
    pub record: MemoryRecord,
    pub score: f32,
    #[serde(default)]
    pub vector_score: f32,
    #[serde(default)]
    pub lexical_score: f32,
    #[serde(default)]
    pub kg_score: f32,
    #[serde(default)]
    pub bm25: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphHopPath {
    pub entity_id: String,
    pub entity_name: String,
    pub depth: usize,
    pub entity_path: String,
    pub relation_path: String,
    #[serde(default)]
    pub memory_hits: Vec<MemoryRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphHopResult {
    pub source: MemoryRecord,
    pub hops: usize,
    pub query: String,
    pub paths: Vec<GraphHopPath>,
}

impl MemoryRecord {
    pub fn from_document(
        workspace_id: &str,
        document: &MemoryDocument,
        primary: bool,
        parent_id: Option<String>,
    ) -> Self {
        let now = Utc::now();
        let revision = document
            .metadata
            .get("revision")
            .and_then(|value| value.as_u64())
            .unwrap_or(1);
        let created_at = document
            .metadata
            .get("created_at")
            .and_then(|value| value.as_str())
            .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
            .map(|value| value.with_timezone(&Utc))
            .unwrap_or(now);
        let updated_at = document
            .metadata
            .get("updated_at")
            .and_then(|value| value.as_str())
            .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
            .map(|value| value.with_timezone(&Utc))
            .unwrap_or(now);

        let id = document
            .id
            .clone()
            .unwrap_or_else(|| stable_key("memory", &[workspace_id, &document.path]));

        Self {
            id,
            workspace_id: workspace_id.to_string(),
            path: document.path.clone(),
            content: document.content.clone(),
            metadata: document.metadata.clone(),
            embedding: document
                .content_vector
                .clone()
                .unwrap_or_else(|| document.embedding.clone()),
            created_at,
            updated_at,
            revision,
            primary,
            parent_id,
            revisions: vec![MemoryRevision {
                revision,
                recorded_at: updated_at,
                path: document.path.clone(),
                content: document.content.clone(),
                metadata: document.metadata.clone(),
            }],
        }
    }

    pub fn to_document(&self) -> MemoryDocument {
        let mut metadata = self.metadata.clone();
        if let Some(object) = metadata.as_object_mut() {
            object.insert("revision".to_string(), serde_json::json!(self.revision));
            object.insert(
                "created_at".to_string(),
                serde_json::json!(self.created_at.to_rfc3339()),
            );
            object.insert(
                "updated_at".to_string(),
                serde_json::json!(self.updated_at.to_rfc3339()),
            );
            object.insert("primary".to_string(), serde_json::json!(self.primary));
            if let Some(parent_id) = &self.parent_id {
                object.insert("parent_id".to_string(), serde_json::json!(parent_id));
            }
        }

        MemoryDocument {
            id: Some(self.id.clone()),
            path: self.path.clone(),
            content: self.content.clone(),
            metadata,
            content_vector: Some(self.embedding.clone()),
            embedding: self.embedding.clone(),
        }
    }

    pub fn matches_query(&self, query: &str) -> bool {
        let lowered = query.trim().to_ascii_lowercase();
        lowered.is_empty()
            || self.path.to_ascii_lowercase().contains(&lowered)
            || self.content.to_ascii_lowercase().contains(&lowered)
            || self
                .metadata
                .to_string()
                .to_ascii_lowercase()
                .contains(&lowered)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTokenRecord {
    pub token: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DurableWorkspaceState {
    #[serde(default)]
    pub memories: Vec<MemoryRecord>,
    #[serde(default)]
    pub beliefs: Vec<BeliefRelation>,
    #[serde(default)]
    pub session_tokens: Vec<SessionTokenRecord>,
    #[serde(default)]
    pub checkpoints: Vec<Checkpoint>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct DurableStoreFile {
    #[serde(default)]
    workspaces: HashMap<String, DurableWorkspaceState>,
}

#[derive(Debug, Clone)]
pub struct SurrealStoreConfig {
    pub url: String,
    pub namespace: String,
    pub database: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

impl SurrealStoreConfig {
    pub fn from_env() -> Self {
        Self {
            url: std::env::var("XAVIER2_SURREALDB_URL")
                .or_else(|_| std::env::var("SURREALDB_URL"))
                .unwrap_or_else(|_| "http://127.0.0.1:8000".to_string()),
            namespace: std::env::var("XAVIER2_SURREALDB_NS")
                .unwrap_or_else(|_| "xavier2".to_string()),
            database: std::env::var("XAVIER2_SURREALDB_DB")
                .unwrap_or_else(|_| "memory".to_string()),
            username: std::env::var("XAVIER2_SURREALDB_USER")
                .ok()
                .or_else(|| std::env::var("SURREALDB_USER").ok()),
            password: std::env::var("XAVIER2_SURREALDB_PASS")
                .ok()
                .or_else(|| std::env::var("SURREALDB_PASS").ok()),
        }
    }

    fn detail(&self) -> String {
        format!("{} ns={} db={}", self.url, self.namespace, self.database)
    }
}

#[async_trait]
pub trait MemoryStore: Send + Sync {
    fn backend(&self) -> MemoryBackend;
    async fn health(&self) -> Result<String>;
    async fn put(&self, record: MemoryRecord) -> Result<()>;
    async fn get(&self, workspace_id: &str, id_or_path: &str) -> Result<Option<MemoryRecord>>;
    async fn update(&self, record: MemoryRecord) -> Result<()>;
    async fn delete(&self, workspace_id: &str, id_or_path: &str) -> Result<Option<MemoryRecord>>;
    async fn list(&self, workspace_id: &str) -> Result<Vec<MemoryRecord>>;
    async fn search(
        &self,
        workspace_id: &str,
        query: &str,
        filters: Option<&MemoryQueryFilters>,
    ) -> Result<Vec<MemoryRecord>>;
    async fn hybrid_search(
        &self,
        workspace_id: &str,
        query: &str,
        mode: HybridSearchMode,
        filters: Option<&MemoryQueryFilters>,
        limit: usize,
    ) -> Result<Vec<HybridSearchResult>> {
        let _ = (workspace_id, query, mode, filters, limit);
        anyhow::bail!(
            "hybrid search is not supported by the {} backend",
            self.backend().as_str()
        )
    }
    async fn graph_hops(
        &self,
        workspace_id: &str,
        path_or_id: &str,
        hops: usize,
        query: &str,
    ) -> Result<GraphHopResult> {
        let _ = (workspace_id, path_or_id, hops, query);
        anyhow::bail!(
            "graph hop traversal is not supported by the {} backend",
            self.backend().as_str()
        )
    }
    async fn load_workspace_state(&self, workspace_id: &str) -> Result<DurableWorkspaceState>;
    async fn save_beliefs(&self, workspace_id: &str, beliefs: Vec<BeliefRelation>) -> Result<()>;
    async fn save_session_token(&self, workspace_id: &str, token: SessionTokenRecord)
        -> Result<()>;
    async fn is_session_token_valid(&self, workspace_id: &str, token: &str) -> Result<bool>;
    async fn save_checkpoint(&self, workspace_id: &str, checkpoint: Checkpoint) -> Result<()>;
    async fn load_checkpoint(
        &self,
        workspace_id: &str,
        task_id: &str,
        name: &str,
    ) -> Result<Option<Checkpoint>>;
    async fn list_checkpoints(&self, workspace_id: &str, task_id: &str) -> Result<Vec<Checkpoint>>;
    async fn delete_checkpoint(&self, workspace_id: &str, task_id: &str, name: &str) -> Result<()>;
}

#[derive(Clone)]
pub struct FileMemoryStore {
    path: PathBuf,
    state: Arc<RwLock<DurableStoreFile>>,
}

impl FileMemoryStore {
    pub async fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let state = if fs::try_exists(&path).await.unwrap_or(false) {
            let payload = fs::read_to_string(&path).await?;
            serde_json::from_str(&payload).unwrap_or_default()
        } else {
            DurableStoreFile::default()
        };

        Ok(Self {
            path,
            state: Arc::new(RwLock::new(state)),
        })
    }

    async fn persist(&self) -> Result<()> {
        let payload = {
            let state = self.state.read().await;
            serde_json::to_vec_pretty(&*state)?
        };
        fs::write(&self.path, payload).await?;
        Ok(())
    }

    fn workspace_mut<'a>(
        state: &'a mut DurableStoreFile,
        workspace_id: &str,
    ) -> &'a mut DurableWorkspaceState {
        state
            .workspaces
            .entry(workspace_id.to_string())
            .or_default()
    }
}

#[async_trait]
impl MemoryStore for FileMemoryStore {
    fn backend(&self) -> MemoryBackend {
        MemoryBackend::File
    }

    async fn health(&self) -> Result<String> {
        Ok(format!("file store at {}", self.path.display()))
    }

    async fn put(&self, record: MemoryRecord) -> Result<()> {
        let workspace_id = record.workspace_id.clone();
        {
            let mut state = self.state.write().await;
            let workspace = Self::workspace_mut(&mut state, &workspace_id);
            workspace.memories.retain(|item| item.id != record.id);
            workspace.memories.push(record);
        }
        self.persist().await
    }

    async fn get(&self, workspace_id: &str, id_or_path: &str) -> Result<Option<MemoryRecord>> {
        let state = self.state.read().await;
        Ok(state
            .workspaces
            .get(workspace_id)
            .and_then(|workspace| {
                workspace
                    .memories
                    .iter()
                    .find(|item| item.id == id_or_path || item.path == id_or_path)
            })
            .cloned())
    }

    async fn update(&self, record: MemoryRecord) -> Result<()> {
        let workspace_id = record.workspace_id.clone();
        {
            let mut state = self.state.write().await;
            let workspace = Self::workspace_mut(&mut state, &workspace_id);
            if let Some(existing) = workspace
                .memories
                .iter_mut()
                .find(|item| item.id == record.id || item.path == record.path)
            {
                *existing = revisioned_record(existing.clone(), record);
            } else {
                workspace.memories.push(record);
            }
        }
        self.persist().await
    }

    async fn delete(&self, workspace_id: &str, id_or_path: &str) -> Result<Option<MemoryRecord>> {
        let removed = {
            let mut state = self.state.write().await;
            let workspace = Self::workspace_mut(&mut state, workspace_id);
            let removed = workspace
                .memories
                .iter()
                .position(|item| item.id == id_or_path || item.path == id_or_path)
                .map(|index| workspace.memories.remove(index));

            if let Some(removed_record) = removed.as_ref() {
                workspace.memories.retain(|item| {
                    item.parent_id.as_deref() != Some(&removed_record.id)
                        && item.parent_id.as_deref() != Some(&removed_record.path)
                });
            }

            removed
        };

        if removed.is_some() {
            self.persist().await?;
        }

        Ok(removed)
    }

    async fn list(&self, workspace_id: &str) -> Result<Vec<MemoryRecord>> {
        let state = self.state.read().await;
        Ok(state
            .workspaces
            .get(workspace_id)
            .map(|workspace| workspace.memories.clone())
            .unwrap_or_default())
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
        let state = self.state.read().await;
        Ok(state
            .workspaces
            .get(workspace_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn save_beliefs(&self, workspace_id: &str, beliefs: Vec<BeliefRelation>) -> Result<()> {
        {
            let mut state = self.state.write().await;
            let workspace = Self::workspace_mut(&mut state, workspace_id);
            workspace.beliefs = beliefs;
        }
        self.persist().await
    }

    async fn save_session_token(
        &self,
        workspace_id: &str,
        token: SessionTokenRecord,
    ) -> Result<()> {
        {
            let mut state = self.state.write().await;
            let workspace = Self::workspace_mut(&mut state, workspace_id);
            workspace
                .session_tokens
                .retain(|item| item.expires_at > Utc::now() && item.token != token.token);
            workspace.session_tokens.push(token);
        }
        self.persist().await
    }

    async fn is_session_token_valid(&self, workspace_id: &str, token: &str) -> Result<bool> {
        let state = self.state.read().await;
        Ok(state
            .workspaces
            .get(workspace_id)
            .map(|workspace| {
                workspace
                    .session_tokens
                    .iter()
                    .any(|item| item.token == token && item.expires_at > Utc::now())
            })
            .unwrap_or(false))
    }

    async fn save_checkpoint(&self, workspace_id: &str, checkpoint: Checkpoint) -> Result<()> {
        {
            let mut state = self.state.write().await;
            let workspace = Self::workspace_mut(&mut state, workspace_id);
            workspace.checkpoints.retain(|item| {
                !(item.task_id == checkpoint.task_id && item.name == checkpoint.name)
            });
            workspace.checkpoints.push(checkpoint);
        }
        self.persist().await
    }

    async fn load_checkpoint(
        &self,
        workspace_id: &str,
        task_id: &str,
        name: &str,
    ) -> Result<Option<Checkpoint>> {
        let state = self.state.read().await;
        Ok(state
            .workspaces
            .get(workspace_id)
            .and_then(|workspace| {
                workspace
                    .checkpoints
                    .iter()
                    .find(|item| item.task_id == task_id && item.name == name)
            })
            .cloned())
    }

    async fn list_checkpoints(&self, workspace_id: &str, task_id: &str) -> Result<Vec<Checkpoint>> {
        let state = self.state.read().await;
        Ok(state
            .workspaces
            .get(workspace_id)
            .map(|workspace| {
                workspace
                    .checkpoints
                    .iter()
                    .filter(|item| item.task_id == task_id)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn delete_checkpoint(&self, workspace_id: &str, task_id: &str, name: &str) -> Result<()> {
        {
            let mut state = self.state.write().await;
            let workspace = Self::workspace_mut(&mut state, workspace_id);
            workspace
                .checkpoints
                .retain(|item| !(item.task_id == task_id && item.name == name));
        }
        self.persist().await
    }
}

#[derive(Clone, Default)]
pub struct InMemoryMemoryStore {
    state: Arc<RwLock<DurableStoreFile>>,
}

impl InMemoryMemoryStore {
    pub fn new() -> Self {
        Self::default()
    }

    fn workspace_mut<'a>(
        state: &'a mut DurableStoreFile,
        workspace_id: &str,
    ) -> &'a mut DurableWorkspaceState {
        state
            .workspaces
            .entry(workspace_id.to_string())
            .or_default()
    }
}

#[async_trait]
impl MemoryStore for InMemoryMemoryStore {
    fn backend(&self) -> MemoryBackend {
        MemoryBackend::Memory
    }

    async fn health(&self) -> Result<String> {
        Ok("in-memory store".to_string())
    }

    async fn put(&self, record: MemoryRecord) -> Result<()> {
        let workspace_id = record.workspace_id.clone();
        let mut state = self.state.write().await;
        let workspace = Self::workspace_mut(&mut state, &workspace_id);
        workspace.memories.retain(|item| item.id != record.id);
        workspace.memories.push(record);
        Ok(())
    }

    async fn get(&self, workspace_id: &str, id_or_path: &str) -> Result<Option<MemoryRecord>> {
        let state = self.state.read().await;
        Ok(state.workspaces.get(workspace_id).and_then(|workspace| {
            workspace
                .memories
                .iter()
                .find(|item| item.id == id_or_path || item.path == id_or_path)
                .cloned()
        }))
    }

    async fn update(&self, record: MemoryRecord) -> Result<()> {
        self.put(record).await
    }

    async fn delete(&self, workspace_id: &str, id_or_path: &str) -> Result<Option<MemoryRecord>> {
        let mut state = self.state.write().await;
        let Some(workspace) = state.workspaces.get_mut(workspace_id) else {
            return Ok(None);
        };
        let previous_len = workspace.memories.len();
        workspace
            .memories
            .retain(|item| item.id != id_or_path && item.path != id_or_path);
        if workspace.memories.len() == previous_len {
            return Ok(None);
        }
        Ok(None)
    }

    async fn list(&self, workspace_id: &str) -> Result<Vec<MemoryRecord>> {
        let state = self.state.read().await;
        Ok(state
            .workspaces
            .get(workspace_id)
            .map(|workspace| workspace.memories.clone())
            .unwrap_or_default())
    }

    async fn search(
        &self,
        workspace_id: &str,
        query: &str,
        filters: Option<&MemoryQueryFilters>,
    ) -> Result<Vec<MemoryRecord>> {
        let records = self.list(workspace_id).await?;
        filter_records(records, workspace_id, query, filters)
    }

    async fn load_workspace_state(&self, workspace_id: &str) -> Result<DurableWorkspaceState> {
        let state = self.state.read().await;
        Ok(state
            .workspaces
            .get(workspace_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn save_beliefs(&self, workspace_id: &str, beliefs: Vec<BeliefRelation>) -> Result<()> {
        let mut state = self.state.write().await;
        let workspace = Self::workspace_mut(&mut state, workspace_id);
        workspace.beliefs = beliefs;
        Ok(())
    }

    async fn save_session_token(
        &self,
        workspace_id: &str,
        token: SessionTokenRecord,
    ) -> Result<()> {
        let mut state = self.state.write().await;
        let workspace = Self::workspace_mut(&mut state, workspace_id);
        workspace
            .session_tokens
            .retain(|existing| existing.token != token.token);
        workspace.session_tokens.push(token);
        Ok(())
    }

    async fn is_session_token_valid(&self, workspace_id: &str, token: &str) -> Result<bool> {
        let now = Utc::now();
        let state = self.state.read().await;
        Ok(state
            .workspaces
            .get(workspace_id)
            .map(|workspace| {
                workspace
                    .session_tokens
                    .iter()
                    .any(|item| item.token == token && item.expires_at > now)
            })
            .unwrap_or(false))
    }

    async fn save_checkpoint(&self, workspace_id: &str, checkpoint: Checkpoint) -> Result<()> {
        let mut state = self.state.write().await;
        let workspace = Self::workspace_mut(&mut state, workspace_id);
        workspace
            .checkpoints
            .retain(|item| !(item.task_id == checkpoint.task_id && item.name == checkpoint.name));
        workspace.checkpoints.push(checkpoint);
        Ok(())
    }

    async fn load_checkpoint(
        &self,
        workspace_id: &str,
        task_id: &str,
        name: &str,
    ) -> Result<Option<Checkpoint>> {
        let state = self.state.read().await;
        Ok(state.workspaces.get(workspace_id).and_then(|workspace| {
            workspace
                .checkpoints
                .iter()
                .find(|item| item.task_id == task_id && item.name == name)
                .cloned()
        }))
    }

    async fn list_checkpoints(&self, workspace_id: &str, task_id: &str) -> Result<Vec<Checkpoint>> {
        let state = self.state.read().await;
        Ok(state
            .workspaces
            .get(workspace_id)
            .map(|workspace| {
                workspace
                    .checkpoints
                    .iter()
                    .filter(|item| item.task_id == task_id)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn delete_checkpoint(&self, workspace_id: &str, task_id: &str, name: &str) -> Result<()> {
        let mut state = self.state.write().await;
        if let Some(workspace) = state.workspaces.get_mut(workspace_id) {
            workspace
                .checkpoints
                .retain(|item| !(item.task_id == task_id && item.name == name));
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct SurrealMemoryStore {
    db: Arc<Surreal<Any>>,
    config: SurrealStoreConfig,
}

impl SurrealMemoryStore {
    pub async fn from_env() -> Result<Self> {
        Self::new(SurrealStoreConfig::from_env()).await
    }

    pub async fn new(config: SurrealStoreConfig) -> Result<Self> {
        let db = connect(config.url.as_str())
            .await
            .with_context(|| format!("failed to connect to SurrealDB at {}", config.url))?;

        if let (Some(username), Some(password)) = (&config.username, &config.password) {
            db.signin(Root {
                username: username.to_string(),
                password: password.to_string(),
            })
            .await
            .with_context(|| format!("failed to sign in to SurrealDB at {}", config.url))?;
        }

        db.use_ns(config.namespace.as_str())
            .use_db(config.database.as_str())
            .await
            .with_context(|| {
                format!(
                    "failed to select SurrealDB namespace/database {}",
                    config.detail()
                )
            })?;

        // Initialize schema: create tables if they don't exist
        Self::init_schema(&db).await?;

        Ok(Self {
            db: Arc::new(db),
            config,
        })
    }

    async fn init_schema(db: &Surreal<Any>) -> Result<()> {
        // Create tables if they don't exist; ignore errors if already defined
        let schema_stmts = vec![
            "DEFINE TABLE memory_records SCHEMAFULL",
            "DEFINE TABLE belief_states SCHEMAFULL",
            "DEFINE TABLE session_tokens SCHEMAFULL",
            "DEFINE TABLE checkpoint_records SCHEMAFULL",
        ];
        for stmt in schema_stmts {
            if let Err(e) = db.query(stmt).await {
                tracing::debug!("schema init (may already exist): {}", e);
            }
        }
        Ok(())
    }

    async fn update_row<T>(&self, table: &str, key: &str, value: &T) -> Result<()>
    where
        T: Serialize + Send + Sync + SurrealValue,
    {
        let payload = serde_json::to_value(value)
            .with_context(|| format!("failed to serialize payload for {table}:{key}"))?;
        // Use db.update() which is SurrealDB's upsert operation
        let _: Option<serde_json::Value> = self
            .db
            .update((table, key))
            .content(payload)
            .await
            .with_context(|| format!("failed to update {table}:{key}"))?;
        Ok(())
    }

    async fn select_workspace_rows<T>(&self, table: &str, workspace_id: &str) -> Result<Vec<T>>
    where
        T: DeserializeOwned + SurrealValue,
    {
        let mut response = self
            .db
            .query(format!(
                "SELECT * FROM {table} WHERE workspace_id = $workspace_id"
            ))
            .bind(("workspace_id", workspace_id.to_string()))
            .await
            .with_context(|| format!("failed to query workspace rows from {table}"))?;
        response
            .take(0)
            .with_context(|| format!("failed to deserialize workspace rows from {table}"))
    }

    async fn select_row<T>(&self, table: &str, key: &str) -> Result<Option<T>>
    where
        T: DeserializeOwned + SurrealValue,
    {
        self.db
            .select((table, key))
            .await
            .with_context(|| format!("failed to select {table}:{key}"))
    }

    /// CRITICAL FIX: O(n) → O(1) lookup by id OR path within a workspace
    /// Previously used list() + find() which scanned ALL records
    async fn select_memory_by_id_or_path(
        &self,
        workspace_id: &str,
        id_or_path: &str,
    ) -> Result<Option<MemoryRecord>>
    where
        MemoryRecord: DeserializeOwned + SurrealValue,
    {
        // Use a direct WHERE id = $id query first (most common case)
        // This is O(1) because id is the primary key in SurrealDB
        if let Some(record) = self
            .select_row(TABLE_MEMORIES, &memory_row_key(workspace_id, id_or_path))
            .await?
        {
            return Ok(Some(record));
        }

        // Fallback: try querying by path (path is not the primary key, so this is still a targeted query)
        let mut response = self
            .db
            .query(
                "SELECT * FROM memory_records WHERE workspace_id = $workspace_id AND path = $path LIMIT 1",
            )
            .bind(("workspace_id", workspace_id.to_string()))
            .bind(("path", id_or_path.to_string()))
            .await
            .with_context(|| "failed to query memory by path")?;
        response
            .take::<Option<MemoryRecord>>(0)
            .with_context(|| "failed to deserialize memory record")
    }

    async fn delete_row(&self, table: &str, key: &str) -> Result<()> {
        let _: Option<serde_json::Value> = self
            .db
            .delete((table, key))
            .await
            .with_context(|| format!("failed to delete {table}:{key}"))?;
        Ok(())
    }

    async fn purge_expired_tokens(&self, workspace_id: &str) -> Result<()> {
        let now = Utc::now();
        for token in self
            .select_workspace_rows::<SessionTokenRow>(TABLE_SESSION_TOKENS, workspace_id)
            .await?
            .into_iter()
            .filter(|token| token.expires_at <= now)
        {
            self.delete_row(TABLE_SESSION_TOKENS, &token.storage_id)
                .await?;
        }
        Ok(())
    }
}

#[async_trait]
impl MemoryStore for SurrealMemoryStore {
    fn backend(&self) -> MemoryBackend {
        MemoryBackend::Surreal
    }

    async fn health(&self) -> Result<String> {
        self.db.health().await.with_context(|| {
            format!("SurrealDB health check failed for {}", self.config.detail())
        })?;
        Ok(format!("surrealdb {}", self.config.detail()))
    }

    async fn put(&self, record: MemoryRecord) -> Result<()> {
        self.update_row(
            TABLE_MEMORIES,
            &memory_row_key(&record.workspace_id, &record.id),
            &record,
        )
        .await
    }

    /// CRITICAL FIX: Changed from O(n) to O(1) using direct query
    /// Previously: list() + find() which scanned ALL records in workspace
    /// Now: Single targeted query with LIMIT 1
    async fn get(&self, workspace_id: &str, id_or_path: &str) -> Result<Option<MemoryRecord>> {
        self.select_memory_by_id_or_path(workspace_id, id_or_path)
            .await
    }

    async fn update(&self, record: MemoryRecord) -> Result<()> {
        let storage_key = memory_row_key(&record.workspace_id, &record.id);
        let record = if let Some(existing) = self.get(&record.workspace_id, &record.id).await? {
            revisioned_record(existing, record)
        } else if let Some(existing) = self.get(&record.workspace_id, &record.path).await? {
            revisioned_record(existing, record)
        } else {
            record
        };
        self.update_row(TABLE_MEMORIES, &storage_key, &record).await
    }

    async fn delete(&self, workspace_id: &str, id_or_path: &str) -> Result<Option<MemoryRecord>> {
        let removed = self.get(workspace_id, id_or_path).await?;
        if let Some(record) = removed.as_ref() {
            self.delete_row(TABLE_MEMORIES, &memory_row_key(workspace_id, &record.id))
                .await?;
            for child in self.list(workspace_id).await?.into_iter().filter(|item| {
                item.parent_id.as_deref() == Some(&record.id)
                    || item.parent_id.as_deref() == Some(&record.path)
            }) {
                self.delete_row(TABLE_MEMORIES, &memory_row_key(workspace_id, &child.id))
                    .await?;
            }
        }
        Ok(removed)
    }

    async fn list(&self, workspace_id: &str) -> Result<Vec<MemoryRecord>> {
        self.select_workspace_rows(TABLE_MEMORIES, workspace_id)
            .await
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
        self.purge_expired_tokens(workspace_id).await?;

        let memories = self.list(workspace_id).await?;
        let beliefs = self
            .select_row::<BeliefStateRow>(TABLE_BELIEFS, &belief_row_key(workspace_id))
            .await?
            .map(|row| row.beliefs)
            .unwrap_or_default();
        let session_tokens = self
            .select_workspace_rows::<SessionTokenRow>(TABLE_SESSION_TOKENS, workspace_id)
            .await?
            .into_iter()
            .filter(|row| row.expires_at > Utc::now())
            .map(SessionTokenRecord::from)
            .collect();
        let checkpoints = self
            .select_workspace_rows::<CheckpointRow>(TABLE_CHECKPOINTS, workspace_id)
            .await?
            .into_iter()
            .map(Checkpoint::from)
            .collect();

        Ok(DurableWorkspaceState {
            memories,
            beliefs,
            session_tokens,
            checkpoints,
        })
    }

    async fn save_beliefs(&self, workspace_id: &str, beliefs: Vec<BeliefRelation>) -> Result<()> {
        let row = BeliefStateRow {
            storage_id: belief_row_key(workspace_id),
            workspace_id: workspace_id.to_string(),
            beliefs,
            updated_at: Utc::now(),
        };
        self.update_row(TABLE_BELIEFS, &row.storage_id, &row).await
    }

    async fn save_session_token(
        &self,
        workspace_id: &str,
        token: SessionTokenRecord,
    ) -> Result<()> {
        self.purge_expired_tokens(workspace_id).await?;
        let row = SessionTokenRow::new(workspace_id, token);
        self.update_row(TABLE_SESSION_TOKENS, &row.storage_id, &row)
            .await
    }

    async fn is_session_token_valid(&self, workspace_id: &str, token: &str) -> Result<bool> {
        self.purge_expired_tokens(workspace_id).await?;
        Ok(self
            .select_workspace_rows::<SessionTokenRow>(TABLE_SESSION_TOKENS, workspace_id)
            .await?
            .into_iter()
            .any(|row| row.token == token && row.expires_at > Utc::now()))
    }

    async fn save_checkpoint(&self, workspace_id: &str, checkpoint: Checkpoint) -> Result<()> {
        let row = CheckpointRow::new(workspace_id, checkpoint);
        self.update_row(TABLE_CHECKPOINTS, &row.storage_id, &row)
            .await
    }

    async fn load_checkpoint(
        &self,
        workspace_id: &str,
        task_id: &str,
        name: &str,
    ) -> Result<Option<Checkpoint>> {
        Ok(self
            .select_row::<CheckpointRow>(
                TABLE_CHECKPOINTS,
                &checkpoint_row_key(workspace_id, task_id, name),
            )
            .await?
            .map(Checkpoint::from))
    }

    async fn list_checkpoints(&self, workspace_id: &str, task_id: &str) -> Result<Vec<Checkpoint>> {
        Ok(self
            .select_workspace_rows::<CheckpointRow>(TABLE_CHECKPOINTS, workspace_id)
            .await?
            .into_iter()
            .filter(|row| row.task_id == task_id)
            .map(Checkpoint::from)
            .collect())
    }

    async fn delete_checkpoint(&self, workspace_id: &str, task_id: &str, name: &str) -> Result<()> {
        self.delete_row(
            TABLE_CHECKPOINTS,
            &checkpoint_row_key(workspace_id, task_id, name),
        )
        .await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
pub(crate) struct BeliefStateRow {
    #[serde(rename = "id")]
    storage_id: String,
    workspace_id: String,
    beliefs: Vec<BeliefRelation>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
pub(crate) struct SessionTokenRow {
    #[serde(rename = "id")]
    pub storage_id: String,
    pub workspace_id: String,
    pub token: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl SessionTokenRow {
    fn new(workspace_id: &str, token: SessionTokenRecord) -> Self {
        Self {
            storage_id: session_token_row_key(workspace_id, &token.token),
            workspace_id: workspace_id.to_string(),
            token: token.token,
            created_at: token.created_at,
            expires_at: token.expires_at,
        }
    }
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

#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
pub(crate) struct CheckpointRow {
    #[serde(rename = "id")]
    storage_id: String,
    workspace_id: String,
    task_id: String,
    name: String,
    data: serde_json::Value,
}

impl CheckpointRow {
    fn new(workspace_id: &str, checkpoint: Checkpoint) -> Self {
        Self {
            storage_id: checkpoint_row_key(workspace_id, &checkpoint.task_id, &checkpoint.name),
            workspace_id: workspace_id.to_string(),
            task_id: checkpoint.task_id,
            name: checkpoint.name,
            data: checkpoint.data,
        }
    }
}

impl From<CheckpointRow> for Checkpoint {
    fn from(value: CheckpointRow) -> Self {
        Self {
            task_id: value.task_id,
            name: value.name,
            data: value.data,
        }
    }
}

pub(crate) fn revisioned_record(existing: MemoryRecord, mut next: MemoryRecord) -> MemoryRecord {
    next.id = existing.id;
    next.created_at = existing.created_at;
    next.updated_at = Utc::now();
    next.revision = existing.revision + 1;
    next.revisions = existing.revisions.clone();
    next.revisions.push(MemoryRevision {
        revision: next.revision,
        recorded_at: next.updated_at,
        path: next.path.clone(),
        content: next.content.clone(),
        metadata: next.metadata.clone(),
    });
    next
}

pub(crate) fn filter_records(
    records: Vec<MemoryRecord>,
    workspace_id: &str,
    query: &str,
    filters: Option<&MemoryQueryFilters>,
) -> Result<Vec<MemoryRecord>> {
    Ok(records
        .into_iter()
        .filter(|record| {
            if !record.matches_query(query) {
                return false;
            }

            filters.is_none_or(|filters| {
                resolve_metadata(&record.path, &record.metadata, workspace_id, None)
                    .map(|resolved| {
                        filters.workspace_id.as_deref().is_none_or(|value| {
                            resolved.namespace.workspace_id.as_deref() == Some(value)
                        }) && filters.project.as_deref().is_none_or(|value| {
                            resolved.namespace.project.as_deref() == Some(value)
                        }) && filters
                            .scope
                            .as_deref()
                            .is_none_or(|value| resolved.namespace.scope.as_deref() == Some(value))
                            && filters.session_id.as_deref().is_none_or(|value| {
                                resolved.namespace.session_id.as_deref() == Some(value)
                            })
                    })
                    .unwrap_or(false)
            })
        })
        .collect())
}

fn memory_row_key(workspace_id: &str, memory_id: &str) -> String {
    stable_key("memory_row", &[workspace_id, memory_id])
}

fn belief_row_key(workspace_id: &str) -> String {
    stable_key("belief_row", &[workspace_id])
}

fn session_token_row_key(workspace_id: &str, token: &str) -> String {
    stable_key("session_token_row", &[workspace_id, token])
}

fn checkpoint_row_key(workspace_id: &str, task_id: &str, name: &str) -> String {
    stable_key("checkpoint_row", &[workspace_id, task_id, name])
}

pub(crate) fn stable_key(kind: &str, parts: &[&str]) -> String {
    let mut digest = Sha256::new();
    digest.update(kind.as_bytes());
    for part in parts {
        digest.update([0u8]);
        digest.update(part.as_bytes());
    }
    hex_encode(&digest.finalize())
}
