use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
};

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use tokio::{
    fs,
    sync::{broadcast, Mutex, RwLock},
};

use crate::{
    agents::{router::RouteCategory, AgentRuntime, RuntimeConfig},
    checkpoint::CheckpointManager,
    memory::{
        belief_graph::{BeliefGraph, SharedBeliefGraph},
        embedder::EmbeddingClient,
        entity_graph::{EntityGraph, SharedEntityGraph},
        qmd_memory::{estimate_document_bytes, MemoryUsage, QmdMemory},
        schema::MemoryQueryFilters,
        semantic::SemanticMemory,
        session_store::SessionStore,
        sqlite_vec_store::VecSqliteMemoryStore,
        store::SqliteMemoryStore,
        surreal_store::{
            FileMemoryStore, InMemoryMemoryStore, MemoryBackend, MemoryRecord, MemoryStore,
            SessionTokenRecord, SurrealMemoryStore,
        },
    },
};
use chrono::{DateTime, Duration, Utc};

const MB: u64 = 1024 * 1024;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlanTier {
    Community,
    Free,
    Personal,
    Pro,
}

impl PlanTier {
    pub fn from_env(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "free" => Self::Free,
            "personal" => Self::Personal,
            "pro" => Self::Pro,
            _ => Self::Community,
        }
    }

    pub fn default_storage_limit_bytes(self) -> Option<u64> {
        match self {
            Self::Community => None,
            Self::Free => Some(100 * MB),
            Self::Personal => Some(500 * MB),
            Self::Pro => Some(2 * 1024 * MB),
        }
    }

    pub fn default_request_limit(self) -> Option<usize> {
        match self {
            Self::Community => None,
            Self::Free => Some(5_000),
            Self::Personal => Some(50_000),
            Self::Pro => Some(250_000),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionToken {
    pub token: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EmbeddingProviderMode {
    BringYourOwn,
    Managed,
}

impl EmbeddingProviderMode {
    fn from_env(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "managed" => Self::Managed,
            _ => Self::BringYourOwn,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SyncPolicy {
    LocalOnly,
    CloudMirror,
    MetadataOnly,
    CloudHotCache,
    GitChunk,
}

impl SyncPolicy {
    fn from_env(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "cloud_mirror" => Self::CloudMirror,
            "metadata_only" => Self::MetadataOnly,
            "cloud_hot_cache" => Self::CloudHotCache,
            "git_chunk" => Self::GitChunk,
            _ => Self::LocalOnly,
        }
    }

    pub fn supported() -> &'static [SyncPolicy] {
        &[
            SyncPolicy::LocalOnly,
            SyncPolicy::CloudMirror,
            SyncPolicy::MetadataOnly,
            SyncPolicy::CloudHotCache,
            SyncPolicy::GitChunk,
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub id: String,
    pub token: String,
    pub plan: PlanTier,
    pub memory_backend: MemoryBackend,
    pub storage_limit_bytes: Option<u64>,
    pub request_limit: Option<usize>,
    pub request_unit_limit: Option<u64>,
    pub embedding_provider_mode: EmbeddingProviderMode,
    pub managed_google_embeddings: bool,
    pub sync_policy: SyncPolicy,
}

impl WorkspaceConfig {
    pub fn from_env() -> Self {
        let plan = std::env::var("XAVIER2_DEFAULT_PLAN")
            .map(|value| PlanTier::from_env(&value))
            .unwrap_or(PlanTier::Community);

        let storage_limit_bytes = std::env::var("XAVIER2_STORAGE_LIMIT_BYTES")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .or_else(|| plan.default_storage_limit_bytes());

        let request_limit = std::env::var("XAVIER2_REQUEST_LIMIT")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .or_else(|| plan.default_request_limit());
        let request_unit_limit = std::env::var("XAVIER2_REQUEST_UNIT_LIMIT")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .or_else(|| request_limit.map(|value| value as u64 * 2));

        Self {
            id: std::env::var("XAVIER2_DEFAULT_WORKSPACE_ID")
                .unwrap_or_else(|_| "default".to_string()),
            token: std::env::var("XAVIER2_TOKEN")
                .expect("XAVIER2_TOKEN environment variable must be set"),
            plan,
            memory_backend: std::env::var("XAVIER2_MEMORY_BACKEND")
                .map(|value| MemoryBackend::from_env(&value))
                .unwrap_or(MemoryBackend::Vec),
            storage_limit_bytes,
            request_limit,
            request_unit_limit,
            embedding_provider_mode: std::env::var("XAVIER2_EMBEDDING_PROVIDER_MODE")
                .map(|value| EmbeddingProviderMode::from_env(&value))
                .unwrap_or(EmbeddingProviderMode::BringYourOwn),
            managed_google_embeddings: std::env::var("XAVIER2_MANAGED_GOOGLE_EMBEDDINGS")
                .ok()
                .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "TRUE")),
            sync_policy: std::env::var("XAVIER2_SYNC_POLICY")
                .map(|value| SyncPolicy::from_env(&value))
                .unwrap_or(SyncPolicy::LocalOnly),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum UsageCategory {
    Read,
    Write,
    Sync,
    AgentRun,
    Code,
    Account,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageCountersSnapshot {
    pub category: UsageCategory,
    pub requests: u64,
    pub units: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceUsageSnapshot {
    pub workspace_id: String,
    pub plan: PlanTier,
    pub document_count: usize,
    pub storage_bytes_used: u64,
    pub storage_bytes_limit: Option<u64>,
    pub storage_bytes_remaining: Option<u64>,
    pub requests_used: usize,
    pub request_limit: Option<usize>,
    pub request_units_used: u64,
    pub request_unit_limit: Option<u64>,
    pub sync_policy: SyncPolicy,
    pub counters: Vec<UsageCountersSnapshot>,
    pub optimization: OptimizationUsageSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelCallSnapshot {
    pub model: String,
    pub calls: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OptimizationUsageSnapshot {
    pub router_direct_count: u64,
    pub router_retrieved_count: u64,
    pub router_complex_count: u64,
    pub semantic_cache_hits: u64,
    pub semantic_cache_misses: u64,
    pub llm_calls: u64,
    pub llm_calls_by_model: Vec<ModelCallSnapshot>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceLimitsSnapshot {
    pub workspace_id: String,
    pub plan: PlanTier,
    pub storage_limit_bytes: Option<u64>,
    pub request_limit: Option<usize>,
    pub request_unit_limit: Option<u64>,
    pub embedding_provider_mode: EmbeddingProviderMode,
    pub managed_google_embeddings: bool,
    pub sync_policy: SyncPolicy,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncPolicySnapshot {
    pub workspace_id: String,
    pub current: SyncPolicy,
    pub supported: Vec<SyncPolicy>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EmbeddingProviderSnapshot {
    pub workspace_id: String,
    pub mode: EmbeddingProviderMode,
    pub managed_google_embeddings: bool,
    pub configured_model: Option<String>,
    pub configured_url: Option<String>,
    pub configured: bool,
    pub available: bool,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct UsageEvent {
    pub category: UsageCategory,
    pub units: u64,
}

impl UsageEvent {
    pub fn from_request(method: &str, path: &str) -> Self {
        match (method, path) {
            ("GET", "/v1/account/usage")
            | ("GET", "/v1/account/limits")
            | ("GET", "/v1/sync/policies")
            | ("GET", "/v1/providers/embeddings/status") => Self {
                category: UsageCategory::Account,
                units: 1,
            },
            ("POST", "/memory/add") | ("POST", "/memory/delete") | ("POST", "/memory/reset") => {
                Self {
                    category: UsageCategory::Write,
                    units: 2,
                }
            }
            ("POST", "/memory/consolidate") | ("POST", "/memory/reflect") => Self {
                category: UsageCategory::Write,
                units: 3,
            },
            ("POST", "/memory/search")
            | ("POST", "/memory/hybrid-search")
            | ("POST", "/memory/hybrid")
            | ("POST", "/memory/query")
            | ("POST", "/memory/graph/hops")
            | ("GET", "/memory/graph") => Self {
                category: UsageCategory::Read,
                units: 1,
            },
            ("POST", "/agents/run") => Self {
                category: UsageCategory::AgentRun,
                units: 10,
            },
            ("POST", "/sync") => Self {
                category: UsageCategory::Sync,
                units: 5,
            },
            ("POST", "/code/scan") => Self {
                category: UsageCategory::Code,
                units: 4,
            },
            ("POST", "/code/find") | ("GET", "/code/stats") => Self {
                category: UsageCategory::Code,
                units: 1,
            },
            _ => {
                let category = if method == "GET" {
                    UsageCategory::Read
                } else {
                    UsageCategory::Other
                };
                Self { category, units: 1 }
            }
        }
    }
}

struct UsageCounter {
    requests: AtomicU64,
    units: AtomicU64,
}

impl UsageCounter {
    fn new() -> Self {
        Self {
            requests: AtomicU64::new(0),
            units: AtomicU64::new(0),
        }
    }

    fn add(&self, units: u64) {
        self.requests.fetch_add(1, Ordering::Relaxed);
        self.units.fetch_add(units, Ordering::Relaxed);
    }

    fn snapshot(&self, category: UsageCategory) -> UsageCountersSnapshot {
        UsageCountersSnapshot {
            category,
            requests: self.requests.load(Ordering::Relaxed),
            units: self.units.load(Ordering::Relaxed),
        }
    }
}

struct UsageMetrics {
    total_units: AtomicU64,
    counters: HashMap<UsageCategory, UsageCounter>,
}

struct OptimizationMetrics {
    router_direct_count: AtomicU64,
    router_retrieved_count: AtomicU64,
    router_complex_count: AtomicU64,
    semantic_cache_hits: AtomicU64,
    semantic_cache_misses: AtomicU64,
    llm_calls: AtomicU64,
    llm_calls_by_model: RwLock<HashMap<String, u64>>,
}

impl OptimizationMetrics {
    fn new() -> Self {
        Self {
            router_direct_count: AtomicU64::new(0),
            router_retrieved_count: AtomicU64::new(0),
            router_complex_count: AtomicU64::new(0),
            semantic_cache_hits: AtomicU64::new(0),
            semantic_cache_misses: AtomicU64::new(0),
            llm_calls: AtomicU64::new(0),
            llm_calls_by_model: RwLock::new(HashMap::new()),
        }
    }

    async fn record(
        &self,
        route_category: RouteCategory,
        semantic_cache_hit: bool,
        llm_used: bool,
        model: Option<&str>,
    ) {
        match route_category {
            RouteCategory::Direct => {
                self.router_direct_count.fetch_add(1, Ordering::Relaxed);
            }
            RouteCategory::Retrieved => {
                self.router_retrieved_count.fetch_add(1, Ordering::Relaxed);
            }
            RouteCategory::Complex => {
                self.router_complex_count.fetch_add(1, Ordering::Relaxed);
            }
        }

        if semantic_cache_hit {
            self.semantic_cache_hits.fetch_add(1, Ordering::Relaxed);
        } else if route_category != RouteCategory::Direct {
            self.semantic_cache_misses.fetch_add(1, Ordering::Relaxed);
        }

        if llm_used {
            self.llm_calls.fetch_add(1, Ordering::Relaxed);
            if let Some(model) = model.filter(|value| !value.trim().is_empty()) {
                let mut calls = self.llm_calls_by_model.write().await;
                *calls.entry(model.to_string()).or_insert(0) += 1;
            }
        }
    }

    async fn hydrate(&self, snapshot: &OptimizationUsageSnapshot) {
        self.router_direct_count
            .store(snapshot.router_direct_count, Ordering::Relaxed);
        self.router_retrieved_count
            .store(snapshot.router_retrieved_count, Ordering::Relaxed);
        self.router_complex_count
            .store(snapshot.router_complex_count, Ordering::Relaxed);
        self.semantic_cache_hits
            .store(snapshot.semantic_cache_hits, Ordering::Relaxed);
        self.semantic_cache_misses
            .store(snapshot.semantic_cache_misses, Ordering::Relaxed);
        self.llm_calls.store(snapshot.llm_calls, Ordering::Relaxed);

        let mut model_calls = self.llm_calls_by_model.write().await;
        model_calls.clear();
        for entry in &snapshot.llm_calls_by_model {
            model_calls.insert(entry.model.clone(), entry.calls);
        }
    }

    async fn snapshot(&self) -> OptimizationUsageSnapshot {
        let mut llm_calls_by_model = self
            .llm_calls_by_model
            .read()
            .await
            .iter()
            .map(|(model, calls)| ModelCallSnapshot {
                model: model.clone(),
                calls: *calls,
            })
            .collect::<Vec<_>>();
        llm_calls_by_model.sort_by(|left, right| left.model.cmp(&right.model));

        OptimizationUsageSnapshot {
            router_direct_count: self.router_direct_count.load(Ordering::Relaxed),
            router_retrieved_count: self.router_retrieved_count.load(Ordering::Relaxed),
            router_complex_count: self.router_complex_count.load(Ordering::Relaxed),
            semantic_cache_hits: self.semantic_cache_hits.load(Ordering::Relaxed),
            semantic_cache_misses: self.semantic_cache_misses.load(Ordering::Relaxed),
            llm_calls: self.llm_calls.load(Ordering::Relaxed),
            llm_calls_by_model,
        }
    }
}

impl UsageMetrics {
    fn new() -> Self {
        let counters = [
            UsageCategory::Read,
            UsageCategory::Write,
            UsageCategory::Sync,
            UsageCategory::AgentRun,
            UsageCategory::Code,
            UsageCategory::Account,
            UsageCategory::Other,
        ]
        .into_iter()
        .map(|category| (category, UsageCounter::new()))
        .collect();

        Self {
            total_units: AtomicU64::new(0),
            counters,
        }
    }

    fn record(&self, event: UsageEvent) {
        self.total_units.fetch_add(event.units, Ordering::Relaxed);
        if let Some(counter) = self.counters.get(&event.category) {
            counter.add(event.units);
        }
    }

    fn total_units(&self) -> u64 {
        self.total_units.load(Ordering::Relaxed)
    }

    fn hydrate(&self, total_units: u64, counters: &[UsageCountersSnapshot]) {
        self.total_units.store(total_units, Ordering::Relaxed);
        for snapshot in counters {
            if let Some(counter) = self.counters.get(&snapshot.category) {
                counter.requests.store(snapshot.requests, Ordering::Relaxed);
                counter.units.store(snapshot.units, Ordering::Relaxed);
            }
        }
    }

    fn snapshots(&self) -> Vec<UsageCountersSnapshot> {
        let mut counters: Vec<_> = self
            .counters
            .iter()
            .map(|(category, counter)| counter.snapshot(*category))
            .collect();
        counters.sort_by_key(|entry| match entry.category {
            UsageCategory::Read => 0,
            UsageCategory::Write => 1,
            UsageCategory::Sync => 2,
            UsageCategory::AgentRun => 3,
            UsageCategory::Code => 4,
            UsageCategory::Account => 5,
            UsageCategory::Other => 6,
        });
        counters
    }
}

pub struct WorkspaceState {
    config: WorkspaceConfig,
    pub memory: Arc<QmdMemory>,
    pub runtime: Arc<AgentRuntime>,
    pub belief_graph: SharedBeliefGraph,
    pub entity_graph: SharedEntityGraph,
    pub semantic_memory: Arc<SemanticMemory>,
    pub memory_manager: Arc<crate::memory::manager::MemoryManager>,
    pub panel_store: Arc<SessionStore>,
    pub checkpoint_manager: Arc<CheckpointManager>,
    store: Arc<dyn MemoryStore>,
    store_migrated_from_file: bool,
    store_migration_detail: String,
    usage_state_path: PathBuf,
    persist_lock: Mutex<()>,
    requests_used: AtomicUsize,
    usage_metrics: UsageMetrics,
    optimization_metrics: OptimizationMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedUsageState {
    requests_used: usize,
    total_units: u64,
    counters: Vec<UsageCountersSnapshot>,
    #[serde(default)]
    optimization: OptimizationUsageSnapshot,
}

impl WorkspaceState {
    pub async fn new(
        config: WorkspaceConfig,
        runtime_config: RuntimeConfig,
        workspace_root: impl Into<PathBuf>,
    ) -> Result<Self> {
        let workspace_root = workspace_root.into();
        fs::create_dir_all(&workspace_root).await?;
        let panel_root = workspace_root.join("panel_threads");
        let usage_state_path = workspace_root.join("usage.json");
        let file_store_path = resolve_file_store_path(&workspace_root);
        let migration_marker_path = durable_migration_marker_path(&file_store_path);
        let (store, store_migrated_from_file, store_migration_detail): (
            Arc<dyn MemoryStore>,
            bool,
            String,
        ) = match config.memory_backend {
            MemoryBackend::File => (
                Arc::new(FileMemoryStore::new(file_store_path.clone()).await?),
                false,
                format!("file backend using {}", file_store_path.display()),
            ),
            MemoryBackend::Memory => (
                Arc::new(InMemoryMemoryStore::new()),
                false,
                "ephemeral in-memory backend".to_string(),
            ),
            MemoryBackend::Surreal => {
                let store: Arc<dyn MemoryStore> = Arc::new(SurrealMemoryStore::from_env().await?);
                let migration = migrate_file_store_if_needed(
                    &config.id,
                    &file_store_path,
                    &migration_marker_path,
                    Arc::clone(&store),
                )
                .await?;
                (store, migration.migrated, migration.detail)
            }
            MemoryBackend::Sqlite => {
                let store: Arc<dyn MemoryStore> = Arc::new(SqliteMemoryStore::from_env().await?);
                let migration = migrate_file_store_if_needed(
                    &config.id,
                    &file_store_path,
                    &migration_marker_path,
                    Arc::clone(&store),
                )
                .await?;
                (store, migration.migrated, migration.detail)
            }
            MemoryBackend::Vec => {
                let store: Arc<dyn MemoryStore> = Arc::new(VecSqliteMemoryStore::from_env().await?);
                let migration = migrate_file_store_if_needed(
                    &config.id,
                    &file_store_path,
                    &migration_marker_path,
                    Arc::clone(&store),
                )
                .await?;
                (store, migration.migrated, migration.detail)
            }
        };
        let durable_state = store.load_workspace_state(&config.id).await?;
        let docs = Arc::new(RwLock::new(
            durable_state
                .memories
                .iter()
                .map(MemoryRecord::to_document)
                .collect(),
        ));
        let memory = Arc::new(QmdMemory::new_with_workspace(docs, config.id.clone()));
        memory.set_store(Arc::clone(&store)).await;
        memory.init().await?;

        let belief_graph = Arc::new(RwLock::new(BeliefGraph::new()));
        belief_graph
            .read()
            .await
            .replace_relations(durable_state.beliefs.clone());
        let entity_graph = Arc::new(EntityGraph::new());
        for document in memory.all_documents().await {
            let memory_id = document
                .id
                .as_deref()
                .unwrap_or(document.path.as_str())
                .to_string();
            if let Err(error) = entity_graph
                .upsert_memory(&memory_id, &document.content, Some(&document.metadata))
                .await
            {
                tracing::warn!(%error, memory_id = %memory_id, "failed to index entity graph from existing memory");
            }
        }
        let semantic_memory = Arc::new(SemanticMemory::new());
        let memory_manager = Arc::new(crate::memory::manager::MemoryManager::new(
            Arc::clone(&memory),
            Some(Arc::clone(&belief_graph)),
        ));
        let checkpoint_manager = Arc::new(CheckpointManager::with_store(
            config.id.clone(),
            Arc::clone(&store),
        ));

        let state = Self {
            runtime: Arc::new(
                AgentRuntime::new(
                    Arc::clone(&memory),
                    Some(Arc::clone(&belief_graph)),
                    runtime_config,
                )?
                .with_checkpoint_manager(Arc::clone(&checkpoint_manager)),
            ),
            belief_graph,
            entity_graph,
            semantic_memory,
            memory_manager,
            panel_store: Arc::new(SessionStore::new(panel_root).await?),
            checkpoint_manager,
            store,
            store_migrated_from_file,
            store_migration_detail,
            usage_state_path,
            persist_lock: Mutex::new(()),
            config,
            memory,
            requests_used: AtomicUsize::new(0),
            usage_metrics: UsageMetrics::new(),
            optimization_metrics: OptimizationMetrics::new(),
        };

        state.load_usage_state().await?;
        Ok(state)
    }

    pub fn config(&self) -> &WorkspaceConfig {
        &self.config
    }

    pub fn durable_store_backend(&self) -> &'static str {
        self.store.backend().as_str()
    }

    pub fn durable_store(&self) -> Arc<dyn MemoryStore> {
        Arc::clone(&self.store)
    }

    pub fn durable_store_migrated_from_file(&self) -> bool {
        self.store_migrated_from_file
    }

    pub fn durable_store_migration_detail(&self) -> &str {
        &self.store_migration_detail
    }

    pub async fn durable_store_health(&self) -> Result<String> {
        self.store.health().await
    }

    /// Safely extract event_tx from the underlying store if it's a VecSqliteMemoryStore
    pub fn event_tx_channel(
        &self,
    ) -> Option<&broadcast::Sender<crate::server::events::RealtimeEvent>> {
        // Use Any trait for safe downcasting
        let store = match self
            .store
            .as_ref()
            .as_any()
            .downcast_ref::<VecSqliteMemoryStore>()
        {
            Some(s) => s,
            None => return None,
        };
        store.event_tx_ref()
    }

    pub async fn record_request(&self, event: UsageEvent) -> Result<()> {
        self.requests_used.fetch_add(1, Ordering::Relaxed);
        self.usage_metrics.record(event);
        self.persist_usage_state().await
    }

    pub async fn usage_snapshot(&self) -> WorkspaceUsageSnapshot {
        let usage = self.memory.usage().await;
        WorkspaceUsageSnapshot {
            workspace_id: self.config.id.clone(),
            plan: self.config.plan,
            document_count: usage.document_count,
            storage_bytes_used: usage.storage_bytes,
            storage_bytes_limit: self.config.storage_limit_bytes,
            storage_bytes_remaining: self
                .config
                .storage_limit_bytes
                .map(|limit| limit.saturating_sub(usage.storage_bytes)),
            requests_used: self.requests_used.load(Ordering::Relaxed),
            request_limit: self.config.request_limit,
            request_units_used: self.usage_metrics.total_units(),
            request_unit_limit: self.config.request_unit_limit,
            sync_policy: self.config.sync_policy,
            counters: self.usage_metrics.snapshots(),
            optimization: self.optimization_metrics.snapshot().await,
        }
    }

    pub async fn export_sync(&self) -> Result<String> {
        let sync_dir = self.usage_state_path.parent().unwrap().join("sync");
        let mut manifest = crate::sync::chunks::load_manifest(&sync_dir)?;
        let docs = self.memory.all_documents().await;

        crate::sync::chunks::export_to_chunk(&sync_dir, &docs, &mut manifest)
    }

    pub async fn import_sync(&self) -> Result<usize> {
        let sync_dir = self.usage_state_path.parent().unwrap().join("sync");
        let manifest = crate::sync::chunks::load_manifest(&sync_dir)?;
        let mut total_imported = 0;

        for hash in manifest.chunks.keys() {
            let docs = crate::sync::chunks::import_from_chunk(&sync_dir, hash)?;
            for doc in docs {
                // Check if already exists to avoid duplicates
                if self
                    .memory
                    .get(doc.id.as_deref().unwrap_or(&doc.path))
                    .await?
                    .is_none()
                {
                    self.memory.add(doc).await?;
                    total_imported += 1;
                }
            }
        }

        Ok(total_imported)
    }

    pub async fn record_optimization(
        &self,
        route_category: RouteCategory,
        semantic_cache_hit: bool,
        llm_used: bool,
        model: Option<&str>,
    ) -> Result<()> {
        self.optimization_metrics
            .record(route_category, semantic_cache_hit, llm_used, model)
            .await;
        self.persist_usage_state().await
    }

    pub fn limits_snapshot(&self) -> WorkspaceLimitsSnapshot {
        WorkspaceLimitsSnapshot {
            workspace_id: self.config.id.clone(),
            plan: self.config.plan,
            storage_limit_bytes: self.config.storage_limit_bytes,
            request_limit: self.config.request_limit,
            request_unit_limit: self.config.request_unit_limit,
            embedding_provider_mode: self.config.embedding_provider_mode,
            managed_google_embeddings: self.config.managed_google_embeddings,
            sync_policy: self.config.sync_policy,
        }
    }

    pub fn sync_policy_snapshot(&self) -> SyncPolicySnapshot {
        SyncPolicySnapshot {
            workspace_id: self.config.id.clone(),
            current: self.config.sync_policy,
            supported: SyncPolicy::supported().to_vec(),
        }
    }

    pub async fn embedding_provider_snapshot(&self) -> EmbeddingProviderSnapshot {
        let configured_url = std::env::var("XAVIER2_EMBEDDING_URL").ok();
        let configured_model = std::env::var("XAVIER2_EMBEDDING_MODEL").ok();
        let configured = configured_url.is_some();
        let (available, last_error) = if configured {
            match EmbeddingClient::from_env() {
                Ok(client) => match client.health().await {
                    Ok(true) => (true, None),
                    Ok(false) => (
                        false,
                        Some("embedding service returned empty vectors".to_string()),
                    ),
                    Err(error) => (false, Some(error.to_string())),
                },
                Err(error) => (false, Some(error.to_string())),
            }
        } else {
            (false, None)
        };

        EmbeddingProviderSnapshot {
            workspace_id: self.config.id.clone(),
            mode: self.config.embedding_provider_mode,
            managed_google_embeddings: self.config.managed_google_embeddings,
            configured_model,
            configured_url,
            configured,
            available,
            last_error,
        }
    }

    pub async fn ensure_within_request_limit(&self) -> Result<()> {
        if let Some(limit) = self.config.request_limit {
            let current = self.requests_used.load(Ordering::Relaxed);
            if current > limit {
                return Err(anyhow!(
                    "request quota exceeded for workspace {}: {} > {}",
                    self.config.id,
                    current,
                    limit
                ));
            }
        }

        if let Some(limit) = self.config.request_unit_limit {
            let current = self.usage_metrics.total_units();
            if current > limit {
                return Err(anyhow!(
                    "request unit quota exceeded for workspace {}: {} > {}",
                    self.config.id,
                    current,
                    limit
                ));
            }
        }
        Ok(())
    }

    pub async fn ensure_within_storage_limit(
        &self,
        path: &str,
        content: &str,
        metadata: &serde_json::Value,
    ) -> Result<()> {
        let Some(limit) = self.config.storage_limit_bytes else {
            return Ok(());
        };

        let MemoryUsage { storage_bytes, .. } = self.memory.usage().await;
        let projected = storage_bytes + estimate_document_bytes(path, content, metadata);

        if projected > limit {
            return Err(anyhow!(
                "storage quota exceeded for workspace {}: projected {} bytes exceeds limit {} bytes",
                self.config.id,
                projected,
                limit
            ));
        }

        Ok(())
    }

    pub async fn ingest(
        &self,
        path: String,
        content: String,
        metadata: serde_json::Value,
        auto_curate: bool,
    ) -> Result<String> {
        self.ingest_typed(path, content, metadata, None, None, auto_curate)
            .await
    }

    pub async fn ingest_typed(
        &self,
        path: String,
        content: String,
        metadata: serde_json::Value,
        typed: Option<crate::memory::schema::TypedMemoryPayload>,
        content_vector: Option<Vec<f32>>,
        auto_curate: bool,
    ) -> Result<String> {
        self.ensure_within_storage_limit(&path, &content, &metadata)
            .await?;

        let doc_id = if let Some(content_vector) = content_vector {
            self.memory
                .add_document_typed_with_embedding(
                    path,
                    content.clone(),
                    metadata.clone(),
                    typed,
                    Some(content_vector),
                )
                .await?
        } else {
            self.memory
                .add_document_typed(path, content.clone(), metadata.clone(), typed)
                .await?
        };

        self.index_memory_layers(&doc_id, &content, &metadata).await;

        if auto_curate {
            let action = crate::memory::manager::MemoryAction::Curate {
                doc_id: doc_id.clone(),
            };
            let _ = self.memory_manager.execute_actions(vec![action]).await;
        }

        Ok(doc_id)
    }

    pub async fn persist_beliefs(&self) -> Result<()> {
        let beliefs = self.belief_graph.read().await.get_relations();
        self.store.save_beliefs(&self.config.id, beliefs).await
    }

    pub async fn index_memory_entities(
        &self,
        memory_id: &str,
        content: &str,
        metadata: &serde_json::Value,
    ) -> Result<()> {
        self.entity_graph
            .upsert_memory(memory_id, content, Some(metadata))
            .await
            .map(|_| ())
    }

    pub async fn index_memory_layers(
        &self,
        memory_id: &str,
        content: &str,
        metadata: &serde_json::Value,
    ) {
        if let Err(error) = self
            .index_memory_entities(memory_id, content, metadata)
            .await
        {
            tracing::warn!(%error, memory_id = %memory_id, "failed to index memory entities");
        }

        if let Err(error) = self.semantic_memory.index_memory(memory_id, content).await {
            tracing::warn!(%error, memory_id = %memory_id, "failed to index semantic memory");
        }
    }

    pub async fn remove_memory_entities(&self, memory_id: &str) -> Result<()> {
        self.entity_graph.remove_memory(memory_id).await
    }

    pub async fn list_memory_records(&self) -> Result<Vec<MemoryRecord>> {
        self.store.list(&self.config.id).await
    }

    pub async fn list_memory_records_filtered(
        &self,
        filters: MemoryQueryFilters,
        limit: usize,
    ) -> Result<Vec<MemoryRecord>> {
        self.store
            .list_filtered(&self.config.id, &filters, limit)
            .await
    }

    pub async fn get_memory_record(&self, id_or_path: &str) -> Result<Option<MemoryRecord>> {
        self.store.get(&self.config.id, id_or_path).await
    }

    pub async fn delete_memory_record(&self, id: &str) -> Result<Option<MemoryRecord>> {
        self.store.delete(&self.config.id, id).await
    }

    pub async fn update_primary_memory(
        &self,
        id: &str,
        path: String,
        content: String,
        metadata: serde_json::Value,
        typed: Option<crate::memory::schema::TypedMemoryPayload>,
    ) -> Result<Option<String>> {
        let Some(existing) = self.memory.get(id).await? else {
            return Ok(None);
        };

        let normalized =
            crate::memory::schema::normalize_metadata(&path, metadata, &self.config.id, typed)?;
        let mut document = crate::memory::qmd_memory::MemoryDocument {
            id: existing.id.clone(),
            path,
            content,
            metadata: normalized,
            content_vector: Some(existing.embedding.clone()),
            embedding: existing.embedding.clone(),
        };

        if let Some(object) = document.metadata.as_object_mut() {
            let revision = existing
                .metadata
                .get("revision")
                .and_then(|value| value.as_u64())
                .unwrap_or(1)
                + 1;
            let created_at = existing
                .metadata
                .get("created_at")
                .cloned()
                .unwrap_or_else(|| serde_json::json!(Utc::now().to_rfc3339()));
            object.insert("revision".to_string(), serde_json::json!(revision));
            object.insert("created_at".to_string(), created_at);
            object.insert(
                "updated_at".to_string(),
                serde_json::json!(Utc::now().to_rfc3339()),
            );
        }

        self.memory.update(document.clone()).await?;
        let memory_id = document.id.clone().unwrap_or_else(|| document.path.clone());
        self.index_memory_layers(&memory_id, &document.content, &document.metadata)
            .await;
        Ok(document.id)
    }

    pub async fn record_session_exchange(
        &self,
        session_id: &str,
        source_app: &str,
        user_message: &str,
        assistant_message: &str,
    ) -> Result<String> {
        let timestamp = Utc::now();
        let path = format!(
            "sessions/{}/{}",
            session_id,
            timestamp.format("%Y%m%dT%H%M%S%.3fZ")
        );
        let content = format!("User: {user_message}\nAssistant: {assistant_message}");

        self.memory
            .add_document_typed(
                path,
                content,
                serde_json::json!({
                    "session_time": timestamp.to_rfc3339(),
                    "source": source_app,
                }),
                Some(crate::memory::schema::TypedMemoryPayload {
                    kind: Some(crate::memory::schema::MemoryKind::Episodic),
                    evidence_kind: Some(crate::memory::schema::EvidenceKind::SessionSummary),
                    namespace: Some(crate::memory::schema::MemoryNamespace {
                        session_id: Some(session_id.to_string()),
                        ..crate::memory::schema::MemoryNamespace::default()
                    }),
                    provenance: Some(crate::memory::schema::MemoryProvenance {
                        source_app: Some(source_app.to_string()),
                        source_type: Some("session_exchange".to_string()),
                        recorded_at: Some(timestamp.to_rfc3339()),
                        ..crate::memory::schema::MemoryProvenance::default()
                    }),
                }),
            )
            .await
    }
}

#[derive(Debug)]
struct FileMigrationResult {
    migrated: bool,
    detail: String,
}

fn resolve_file_store_path(workspace_root: &std::path::Path) -> PathBuf {
    std::env::var("XAVIER2_MEMORY_FILE_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| workspace_root.join("memory-store.json"))
}

fn durable_migration_marker_path(file_store_path: &std::path::Path) -> PathBuf {
    file_store_path.with_extension("durable.migrated.json")
}

async fn migrate_file_store_if_needed(
    workspace_id: &str,
    file_store_path: &std::path::Path,
    marker_path: &std::path::Path,
    target_store: Arc<dyn MemoryStore>,
) -> Result<FileMigrationResult> {
    let legacy_marker_path = file_store_path.with_extension("surreal.migrated.json");
    let active_marker_path = if fs::try_exists(marker_path).await.unwrap_or(false) {
        marker_path.to_path_buf()
    } else if fs::try_exists(&legacy_marker_path).await.unwrap_or(false) {
        legacy_marker_path
    } else {
        marker_path.to_path_buf()
    };

    if fs::try_exists(&active_marker_path).await.unwrap_or(false) {
        let detail = match fs::read_to_string(&active_marker_path).await {
            Ok(detail) => detail,
            Err(_) => "legacy durable-store migration already recorded".to_string(),
        };
        return Ok(FileMigrationResult {
            migrated: detail.contains("\"migrated\":true"),
            detail,
        });
    }

    if !fs::try_exists(file_store_path).await.unwrap_or(false) {
        return Ok(FileMigrationResult {
            migrated: false,
            detail: "no legacy file store found".to_string(),
        });
    }

    let legacy_store = FileMemoryStore::new(file_store_path).await?;
    let legacy_state = legacy_store.load_workspace_state(workspace_id).await?;
    let target_state = target_store.load_workspace_state(workspace_id).await?;

    let should_import = target_state.memories.is_empty()
        && target_state.beliefs.is_empty()
        && target_state.session_tokens.is_empty()
        && target_state.checkpoints.is_empty()
        && (!legacy_state.memories.is_empty()
            || !legacy_state.beliefs.is_empty()
            || !legacy_state.session_tokens.is_empty()
            || !legacy_state.checkpoints.is_empty());

    if should_import {
        for record in legacy_state.memories.clone() {
            target_store.put(record).await?;
        }
        target_store
            .save_beliefs(workspace_id, legacy_state.beliefs.clone())
            .await?;
        for token in legacy_state.session_tokens.clone() {
            target_store.save_session_token(workspace_id, token).await?;
        }
        for checkpoint in legacy_state.checkpoints.clone() {
            target_store
                .save_checkpoint(workspace_id, checkpoint)
                .await?;
        }
    }

    let detail = serde_json::json!({
        "migrated": should_import,
        "source": file_store_path.display().to_string(),
        "legacy_memories": legacy_state.memories.len(),
        "legacy_beliefs": legacy_state.beliefs.len(),
        "legacy_session_tokens": legacy_state.session_tokens.len(),
        "legacy_checkpoints": legacy_state.checkpoints.len(),
        "reason": if should_import {
            format!("imported legacy file store into {}", target_store.backend().as_str())
        } else {
            "skipped legacy import because target store already contained data or file was empty".to_string()
        }
    })
    .to_string();
    fs::write(marker_path, &detail).await?;

    Ok(FileMigrationResult {
        migrated: should_import,
        detail,
    })
}

impl WorkspaceState {
    async fn load_usage_state(&self) -> Result<()> {
        if !fs::try_exists(&self.usage_state_path)
            .await
            .unwrap_or(false)
        {
            return Ok(());
        }

        let payload = fs::read_to_string(&self.usage_state_path).await?;
        let persisted: PersistedUsageState = serde_json::from_str(&payload)?;
        self.requests_used
            .store(persisted.requests_used, Ordering::Relaxed);
        self.usage_metrics
            .hydrate(persisted.total_units, &persisted.counters);
        self.optimization_metrics
            .hydrate(&persisted.optimization)
            .await;
        Ok(())
    }

    async fn persist_usage_state(&self) -> Result<()> {
        let _guard = self.persist_lock.lock().await;
        let snapshot = PersistedUsageState {
            requests_used: self.requests_used.load(Ordering::Relaxed),
            total_units: self.usage_metrics.total_units(),
            counters: self.usage_metrics.snapshots(),
            optimization: self.optimization_metrics.snapshot().await,
        };
        let payload = serde_json::to_vec_pretty(&snapshot)?;
        fs::write(&self.usage_state_path, payload).await?;
        Ok(())
    }

    pub async fn generate_session_token(&self) -> Result<String> {
        let token = ulid::Ulid::new().to_string();
        let now = Utc::now();
        let expires_at = now + Duration::hours(12);

        let session = SessionToken {
            token: token.clone(),
            created_at: now,
            expires_at,
        };
        self.store
            .save_session_token(
                &self.config.id,
                SessionTokenRecord {
                    token: session.token,
                    created_at: session.created_at,
                    expires_at: session.expires_at,
                },
            )
            .await?;

        Ok(token)
    }

    pub async fn is_session_token_valid(&self, token_str: &str) -> bool {
        self.store
            .is_session_token_valid(&self.config.id, token_str)
            .await
            .unwrap_or(false)
    }
}

#[derive(Clone)]
pub struct WorkspaceContext {
    pub workspace_id: String,
    pub workspace: Arc<WorkspaceState>,
}

#[derive(Clone, Default)]
pub struct WorkspaceRegistry {
    workspaces: Arc<RwLock<HashMap<String, Arc<WorkspaceState>>>>,
    token_map: Arc<RwLock<HashMap<String, String>>>,
}

impl WorkspaceRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn insert(&self, workspace: WorkspaceState) -> Result<()> {
        let workspace = Arc::new(workspace);
        let workspace_id = workspace.config.id.clone();
        let token = workspace.config.token.clone();

        self.token_map
            .write()
            .await
            .insert(token, workspace_id.clone());
        self.workspaces
            .write()
            .await
            .insert(workspace_id, workspace);
        Ok(())
    }

    pub async fn authenticate(&self, token: &str) -> Option<WorkspaceContext> {
        // 1. Check static tokens (token_map)
        if let Some(workspace_id) = self.token_map.read().await.get(token).cloned() {
            if let Some(workspace) = self.workspaces.read().await.get(&workspace_id).cloned() {
                return Some(WorkspaceContext {
                    workspace_id,
                    workspace,
                });
            }
        }

        // 2. Check dynamic session tokens in all workspaces
        let workspaces = self.workspaces.read().await;
        for (id, workspace) in workspaces.iter() {
            if workspace.is_session_token_valid(token).await {
                return Some(WorkspaceContext {
                    workspace_id: id.clone(),
                    workspace: workspace.clone(),
                });
            }
        }

        None
    }

    pub async fn default_context(&self) -> Option<WorkspaceContext> {
        let preferred_id =
            std::env::var("XAVIER2_DEFAULT_WORKSPACE_ID").unwrap_or_else(|_| "default".to_string());
        let workspaces = self.workspaces.read().await;

        if let Some(workspace) = workspaces.get(&preferred_id).cloned() {
            return Some(WorkspaceContext {
                workspace_id: preferred_id,
                workspace,
            });
        }

        workspaces
            .iter()
            .next()
            .map(|(id, workspace)| WorkspaceContext {
                workspace_id: id.clone(),
                workspace: workspace.clone(),
            })
    }

    pub fn default_context_sync(&self) -> Option<WorkspaceContext> {
        let preferred_id =
            std::env::var("XAVIER2_DEFAULT_WORKSPACE_ID").unwrap_or_else(|_| "default".to_string());
        let workspaces = self.workspaces.blocking_read();

        if let Some(workspace) = workspaces.get(&preferred_id).cloned() {
            return Some(WorkspaceContext {
                workspace_id: preferred_id,
                workspace,
            });
        }

        workspaces
            .iter()
            .next()
            .map(|(id, workspace)| WorkspaceContext {
                workspace_id: id.clone(),
                workspace: workspace.clone(),
            })
    }

    pub async fn default_from_env(runtime_config: RuntimeConfig) -> Result<Self> {
        let registry = Self::new();
        let config = WorkspaceConfig::from_env();
        let panel_root = PathBuf::from("data").join("workspaces").join(&config.id);
        let workspace = WorkspaceState::new(config, runtime_config, panel_root).await?;
        seed_workspace(&workspace).await?;
        registry.insert(workspace).await?;
        Ok(registry)
    }
}

async fn seed_workspace(workspace: &WorkspaceState) -> Result<()> {
    let seed_docs = [
        (
            "system/xavier2",
            "Xavier2 is the central memory system for SWAL agents. Use /memory/add to store, /memory/search to find, /memory/query for AI responses.",
            serde_json::json!({"type": "system", "tags": ["xavier2", "memory"]}),
        ),
        (
            "system/swal",
            "SouthWest AI Labs (SWAL) builds AI agents. BELA is the developer. Projects: Xavier2 (memory), ZeroClaw (runtime), ManteniApp (SaaS), Trading Bot.",
            serde_json::json!({"type": "company", "tags": ["swal", "company"]}),
        ),
        (
            "docs/api",
            "Xavier2 API: POST /memory/add (content, path, metadata), POST /memory/search (query, limit), POST /memory/query (query). Auth: X-Xavier2-Token header.",
            serde_json::json!({"type": "docs", "tags": ["api"]}),
        ),
    ];

    for (path, content, metadata) in seed_docs {
        if workspace.memory.get(path).await?.is_some() {
            continue;
        }
        let normalized =
            crate::memory::schema::normalize_metadata(path, metadata, &workspace.config.id, None)?;
        workspace
            .memory
            .add(crate::memory::qmd_memory::MemoryDocument {
                id: Some(ulid::Ulid::new().to_string()),
                path: path.to_string(),
                content: content.to_string(),
                metadata: normalized,
                content_vector: Some(Vec::new()),
                embedding: Vec::new(),
            })
            .await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn personal_plan_defaults_to_500mb() {
        let config = WorkspaceConfig {
            id: "ws".to_string(),
            token: "token".to_string(),
            plan: PlanTier::Personal,
            memory_backend: MemoryBackend::File,
            storage_limit_bytes: PlanTier::Personal.default_storage_limit_bytes(),
            request_limit: PlanTier::Personal.default_request_limit(),
            request_unit_limit: Some(100_000),
            embedding_provider_mode: EmbeddingProviderMode::BringYourOwn,
            managed_google_embeddings: false,
            sync_policy: SyncPolicy::CloudMirror,
        };

        let workspace = WorkspaceState::new(
            config,
            RuntimeConfig::default(),
            std::env::temp_dir().join(format!("xavier2-ws-{}", ulid::Ulid::new())),
        )
        .await
        .unwrap();

        assert_eq!(workspace.config.storage_limit_bytes, Some(500 * MB));
        assert_eq!(workspace.config.request_limit, Some(50_000));
    }

    #[test]
    fn usage_event_weights_sync_and_agent_calls_higher() {
        let sync = UsageEvent::from_request("POST", "/sync");
        let agent = UsageEvent::from_request("POST", "/agents/run");
        let read = UsageEvent::from_request("POST", "/memory/search");

        assert_eq!(sync.category, UsageCategory::Sync);
        assert_eq!(sync.units, 5);
        assert_eq!(agent.category, UsageCategory::AgentRun);
        assert_eq!(agent.units, 10);
        assert_eq!(read.category, UsageCategory::Read);
        assert_eq!(read.units, 1);
    }

    #[tokio::test]
    async fn usage_state_persists_between_workspace_reloads() {
        let root = std::env::temp_dir().join(format!("xavier2-usage-{}", ulid::Ulid::new()));
        let workspace = WorkspaceState::new(
            WorkspaceConfig {
                id: "persist".to_string(),
                token: "token".to_string(),
                plan: PlanTier::Personal,
                memory_backend: MemoryBackend::File,
                storage_limit_bytes: Some(500 * MB),
                request_limit: Some(50_000),
                request_unit_limit: Some(100_000),
                embedding_provider_mode: EmbeddingProviderMode::BringYourOwn,
                managed_google_embeddings: false,
                sync_policy: SyncPolicy::CloudMirror,
            },
            RuntimeConfig::default(),
            &root,
        )
        .await
        .unwrap();

        workspace
            .record_request(UsageEvent::from_request("POST", "/sync"))
            .await
            .unwrap();
        workspace
            .record_request(UsageEvent::from_request("POST", "/agents/run"))
            .await
            .unwrap();

        let reloaded = WorkspaceState::new(
            WorkspaceConfig {
                id: "persist".to_string(),
                token: "token".to_string(),
                plan: PlanTier::Personal,
                memory_backend: MemoryBackend::File,
                storage_limit_bytes: Some(500 * MB),
                request_limit: Some(50_000),
                request_unit_limit: Some(100_000),
                embedding_provider_mode: EmbeddingProviderMode::BringYourOwn,
                managed_google_embeddings: false,
                sync_policy: SyncPolicy::CloudMirror,
            },
            RuntimeConfig::default(),
            &root,
        )
        .await
        .unwrap();

        let usage = reloaded.usage_snapshot().await;
        assert_eq!(usage.requests_used, 2);
        assert_eq!(usage.request_units_used, 15);
    }

    #[tokio::test]
    async fn durable_memory_rehydrates_between_workspace_reloads() {
        let root = std::env::temp_dir().join(format!("xavier2-memory-{}", ulid::Ulid::new()));
        let config = WorkspaceConfig {
            id: "persist-memory".to_string(),
            token: "token".to_string(),
            plan: PlanTier::Personal,
            memory_backend: MemoryBackend::File,
            storage_limit_bytes: Some(500 * MB),
            request_limit: Some(50_000),
            request_unit_limit: Some(100_000),
            embedding_provider_mode: EmbeddingProviderMode::BringYourOwn,
            managed_google_embeddings: false,
            sync_policy: SyncPolicy::CloudMirror,
        };

        let workspace = WorkspaceState::new(config.clone(), RuntimeConfig::default(), &root)
            .await
            .unwrap();
        let doc_id = workspace
            .memory
            .add_document_typed(
                "projects/xavier2/core".to_string(),
                "Durable memory survives restarts.".to_string(),
                serde_json::json!({"project":"xavier2"}),
                Some(crate::memory::schema::TypedMemoryPayload {
                    kind: Some(crate::memory::schema::MemoryKind::Semantic),
                    evidence_kind: Some(crate::memory::schema::EvidenceKind::Observation),
                    namespace: Some(crate::memory::schema::MemoryNamespace {
                        project: Some("xavier2".to_string()),
                        ..crate::memory::schema::MemoryNamespace::default()
                    }),
                    provenance: None,
                }),
            )
            .await
            .unwrap();

        let reloaded = WorkspaceState::new(config, RuntimeConfig::default(), &root)
            .await
            .unwrap();
        let doc = reloaded.memory.get(&doc_id).await.unwrap().unwrap();
        assert_eq!(doc.content, "Durable memory survives restarts.");
        assert_eq!(doc.metadata["kind"].as_str(), Some("semantic"));
    }

    #[tokio::test]
    async fn session_tokens_beliefs_and_checkpoints_persist_between_reloads() {
        let root = std::env::temp_dir().join(format!("xavier2-state-{}", ulid::Ulid::new()));
        let config = WorkspaceConfig {
            id: "persist-state".to_string(),
            token: "token".to_string(),
            plan: PlanTier::Personal,
            memory_backend: MemoryBackend::File,
            storage_limit_bytes: Some(500 * MB),
            request_limit: Some(50_000),
            request_unit_limit: Some(100_000),
            embedding_provider_mode: EmbeddingProviderMode::BringYourOwn,
            managed_google_embeddings: false,
            sync_policy: SyncPolicy::CloudMirror,
        };

        let workspace = WorkspaceState::new(config.clone(), RuntimeConfig::default(), &root)
            .await
            .unwrap();
        let session_token = workspace.generate_session_token().await.unwrap();
        workspace
            .belief_graph
            .read()
            .await
            .add_edge(
                "xavier2".to_string(),
                "memory".to_string(),
                "is_a".to_string(),
            )
            .await;
        workspace.persist_beliefs().await.unwrap();
        workspace
            .checkpoint_manager
            .save(crate::checkpoint::Checkpoint::new(
                "task-1".to_string(),
                "restore".to_string(),
                serde_json::json!({"ok": true}),
            ))
            .await
            .unwrap();

        let reloaded = WorkspaceState::new(config, RuntimeConfig::default(), &root)
            .await
            .unwrap();
        assert!(reloaded.is_session_token_valid(&session_token).await);
        assert_eq!(reloaded.belief_graph.read().await.get_relations().len(), 1);
        let checkpoint = reloaded
            .checkpoint_manager
            .load("task-1".to_string(), "restore".to_string())
            .await
            .unwrap();
        assert!(checkpoint.is_some());
    }
}
