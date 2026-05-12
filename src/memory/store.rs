//! Core memory store trait and shared types for Xavier.
//!
//! Defines the MemoryStore trait and all shared data structures
//! used by concrete store implementations (SqliteMemoryStore in sqlite_store.rs,
//! VecSqliteMemoryStore in sqlite_vec_store.rs, etc.).

use std::{any::Any as StdAny, collections::HashMap, path::PathBuf, sync::Arc};

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::{fs, sync::RwLock};

use crate::checkpoint::Checkpoint;
use crate::memory::belief_graph::BeliefRelation;
use crate::memory::qmd_memory::MemoryDocument;
use crate::memory::schema::{resolve_metadata, MemoryLevel, MemoryQueryFilters, RelationKind};
use crate::utils::crypto::hex_encode;

// ---------------------------------------------------------------------------
// Backend enum
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryBackend {
    File,
    Memory,
    Sqlite,
    Vec, // SQLite + sqlite-vec vector search
}

impl MemoryBackend {
    pub fn from_env(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "memory" => Self::Memory,
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
            Self::Sqlite => "sqlite",
            Self::Vec => "vec",
        }
    }
}

// ---------------------------------------------------------------------------
// Core data types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRevision {
    pub revision: u64,
    pub recorded_at: DateTime<Utc>,
    pub path: String,
    pub content: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub cluster_id: Option<String>,
    pub level: MemoryLevel,
    pub relation: Option<RelationKind>,
    #[serde(default)]
    pub revisions: Vec<MemoryRevision>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextNode {
    pub record: MemoryRecord,
    pub children: Vec<ContextNode>,
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
pub(crate) struct DurableStoreFile {
    #[serde(default)]
    pub workspaces: HashMap<String, DurableWorkspaceState>,
}

// ---------------------------------------------------------------------------
// MemoryRecord helpers
// ---------------------------------------------------------------------------

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

        let resolved =
            resolve_metadata(&document.path, &document.metadata, workspace_id, None).ok();
        let level = resolved
            .as_ref()
            .map(|r| r.level)
            .unwrap_or(MemoryLevel::Raw);
        let cluster_id = resolved
            .as_ref()
            .and_then(|r| r.provenance.cluster_id.clone());
        let relation = resolved.as_ref().and_then(|r| r.provenance.relation);
        let parent_id = parent_id.or_else(|| {
            resolved
                .as_ref()
                .and_then(|r| r.provenance.parent_id.clone())
        });

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
            cluster_id,
            level,
            relation,
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
            object.insert("level".to_string(), serde_json::json!(self.level.as_str()));
            if let Some(parent_id) = &self.parent_id {
                object.insert("parent_id".to_string(), serde_json::json!(parent_id));
            }
            if let Some(cluster_id) = &self.cluster_id {
                object.insert("cluster_id".to_string(), serde_json::json!(cluster_id));
            }
            if let Some(relation) = self.relation {
                object.insert("relation".to_string(), serde_json::json!(relation.as_str()));
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

    pub fn new_fact(path: String, content: String) -> Self {
        let now = Utc::now();
        Self {
            id: stable_key("memory", &["default", &path]),
            workspace_id: "default".to_string(),
            path: path.clone(),
            content: content.clone(),
            metadata: serde_json::json!({}),
            embedding: Vec::new(),
            created_at: now,
            updated_at: now,
            revision: 1,
            primary: true,
            parent_id: None,
            cluster_id: None,
            level: MemoryLevel::Raw,
            relation: None,
            revisions: vec![MemoryRevision {
                revision: 1,
                recorded_at: now,
                path,
                content,
                metadata: serde_json::json!({}),
            }],
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

// ---------------------------------------------------------------------------
// MemoryStore trait
// ---------------------------------------------------------------------------

#[async_trait]
pub trait MemoryStore: Send + Sync {
    fn backend(&self) -> MemoryBackend;
    fn as_any(&self) -> &dyn StdAny;
    async fn health(&self) -> Result<String>;
    async fn put(&self, record: MemoryRecord) -> Result<()>;
    async fn get(&self, workspace_id: &str, id_or_path: &str) -> Result<Option<MemoryRecord>>;
    async fn update(&self, record: MemoryRecord) -> Result<()>;
    async fn delete(&self, workspace_id: &str, id_or_path: &str) -> Result<Option<MemoryRecord>>;
    async fn list(&self, workspace_id: &str) -> Result<Vec<MemoryRecord>>;
    async fn export(&self, path: &std::path::Path) -> Result<()>;
    async fn export_tree(&self, workspace_id: &str, path: &std::path::Path) -> Result<()>;
    async fn import(&self, path: &std::path::Path) -> Result<()>;
    async fn list_filtered(
        &self,
        workspace_id: &str,
        filters: &MemoryQueryFilters,
        limit: usize,
    ) -> Result<Vec<MemoryRecord>> {
        let all = self.list(workspace_id).await?;
        Ok(filter_records(all, workspace_id, "", Some(filters))?
            .into_iter()
            .take(limit)
            .collect())
    }
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
    /// List timeline events for a workspace since the given ISO 8601 timestamp.
    async fn list_timeline_events(
        &self,
        workspace_id: &str,
        since: &str,
    ) -> Result<Vec<crate::server::events::RealtimeEvent>> {
        let _ = (workspace_id, since);
        anyhow::bail!(
            "timeline events are not supported by the {} backend",
            self.backend().as_str()
        )
    }
}

// ---------------------------------------------------------------------------
// FileMemoryStore
// ---------------------------------------------------------------------------

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

    pub async fn persist(&self) -> Result<()> {
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

    fn as_any(&self) -> &dyn StdAny {
        self
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

    async fn export(&self, path: &std::path::Path) -> Result<()> {
        let payload = {
            let state = self.state.read().await;
            serde_json::to_vec_pretty(&*state)?
        };
        tokio::fs::write(path, payload).await?;
        Ok(())
    }

    async fn export_tree(&self, workspace_id: &str, path: &std::path::Path) -> Result<()> {
        let records = self.list(workspace_id).await?;
        let tree = build_context_tree(records);
        let json = serde_json::to_string_pretty(&tree)?;
        tokio::fs::write(path, json).await?;
        Ok(())
    }

    async fn import(&self, path: &std::path::Path) -> Result<()> {
        let payload = tokio::fs::read_to_string(path).await?;
        let state: DurableStoreFile = serde_json::from_str(&payload)?;
        let mut current_state = self.state.write().await;
        *current_state = state;
        self.persist().await?;
        Ok(())
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

// ---------------------------------------------------------------------------
// InMemoryMemoryStore
// ---------------------------------------------------------------------------

#[derive(Clone, Default)]
pub struct InMemoryMemoryStore {
    state: Arc<RwLock<DurableStoreFile>>,
}

impl InMemoryMemoryStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl MemoryStore for InMemoryMemoryStore {
    fn backend(&self) -> MemoryBackend {
        MemoryBackend::Memory
    }

    fn as_any(&self) -> &dyn StdAny {
        self
    }

    async fn health(&self) -> Result<String> {
        Ok("in-memory store".to_string())
    }

    async fn put(&self, record: MemoryRecord) -> Result<()> {
        let workspace_id = record.workspace_id.clone();
        let mut state = self.state.write().await;
        let workspace = state.workspaces.entry(workspace_id).or_default();
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

    async fn export(&self, _path: &std::path::Path) -> Result<()> {
        anyhow::bail!("export not supported for in-memory store")
    }

    async fn export_tree(&self, workspace_id: &str, path: &std::path::Path) -> Result<()> {
        let records = self.list(workspace_id).await?;
        let tree = build_context_tree(records);
        let json = serde_json::to_string_pretty(&tree)?;
        tokio::fs::write(path, json).await?;
        Ok(())
    }

    async fn import(&self, _path: &std::path::Path) -> Result<()> {
        anyhow::bail!("import not supported for in-memory store")
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
        let workspace = state
            .workspaces
            .entry(workspace_id.to_string())
            .or_default();
        workspace.beliefs = beliefs;
        Ok(())
    }

    async fn save_session_token(
        &self,
        workspace_id: &str,
        token: SessionTokenRecord,
    ) -> Result<()> {
        let mut state = self.state.write().await;
        let workspace = state
            .workspaces
            .entry(workspace_id.to_string())
            .or_default();
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
        let workspace = state
            .workspaces
            .entry(workspace_id.to_string())
            .or_default();
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

// ---------------------------------------------------------------------------
// Shared helper functions
// ---------------------------------------------------------------------------

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

pub fn build_context_tree(records: Vec<MemoryRecord>) -> Vec<ContextNode> {
    let mut nodes: HashMap<String, ContextNode> = records
        .into_iter()
        .map(|r| {
            (
                r.id.clone(),
                ContextNode {
                    record: r,
                    children: vec![],
                },
            )
        })
        .collect();

    let mut child_ids = vec![];
    for node in nodes.values() {
        if let Some(parent_id) = &node.record.parent_id {
            child_ids.push((parent_id.clone(), node.record.id.clone()));
        }
    }

    for (parent_id, child_id) in child_ids {
        if let Some(child_node) = nodes.remove(&child_id) {
            if let Some(parent_node) = nodes.get_mut(&parent_id) {
                parent_node.children.push(child_node);
            } else {
                nodes.insert(child_id, child_node);
            }
        }
    }

    nodes.into_values().collect()
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

pub(crate) fn stable_key(kind: &str, parts: &[&str]) -> String {
    let mut digest = Sha256::new();
    digest.update(kind.as_bytes());
    for part in parts {
        digest.update([0u8]);
        digest.update(part.as_bytes());
    }
    hex_encode(&digest.finalize())
}
