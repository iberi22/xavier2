use std::{
    collections::HashMap,
    sync::{
        atomic::AtomicUsize,
        Arc,
    },
    time::Instant,
};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock as AsyncRwLock;
use std::sync::LazyLock;

/// CRITICAL FIX: Embedding cache with TTL to avoid re-embedding identical content
pub(crate) const EMBEDDING_CACHE_TTL_SECS: u64 = 3600; // 1 hour

pub(crate) struct EmbeddingCacheEntry {
    pub(crate) vector: Vec<f32>,
    pub(crate) cached_at: Instant,
}

/// Global embedding cache - shared across all QmdMemory instances
pub(crate) static EMBEDDING_CACHE: LazyLock<Arc<AsyncRwLock<HashMap<String, EmbeddingCacheEntry>>>> =
    LazyLock::new(|| Arc::new(AsyncRwLock::new(HashMap::new())));

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryDocument {
    pub id: Option<String>,
    pub path: String,
    pub content: String,
    pub metadata: serde_json::Value,
    #[serde(default)]
    pub content_vector: Option<Vec<f32>>,
    pub embedding: Vec<f32>,
}

impl MemoryDocument {
    pub fn estimated_bytes(&self) -> u64 {
        self.id
            .as_ref()
            .map(|value| value.len())
            .unwrap_or_default() as u64
            + self.path.len() as u64
            + self.content.len() as u64
            + self.metadata.to_string().len() as u64
            + self
                .content_vector
                .as_ref()
                .map(|value| value.len() * std::mem::size_of::<f32>())
                .unwrap_or_default() as u64
            + (self.embedding.len() * std::mem::size_of::<f32>()) as u64
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemoryUsage {
    pub document_count: usize,
    pub storage_bytes: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CacheMetrics {
    pub hits: usize,
    pub misses: usize,
    pub entries: usize,
}

#[derive(Debug, Clone)]
pub struct CachedSearchResult {
    pub documents: Vec<MemoryDocument>,
    pub cache_hit: bool,
}

/// CRITICAL FIX: Added workspace_id to prevent cross-workspace cache contamination
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub(crate) struct SearchCacheKey {
    pub(crate) workspace_id: String,
    pub(crate) query: String,
    pub(crate) limit: usize,
    pub(crate) filters: String,
}

#[derive(Default)]
pub(crate) struct CacheCounters {
    pub(crate) hits: AtomicUsize,
    pub(crate) misses: AtomicUsize,
}
