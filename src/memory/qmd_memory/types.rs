use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::AtomicUsize;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct EmbeddingCacheEntry {
    pub vector: Vec<f32>,
    pub cached_at: Instant,
}

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

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct SearchCacheKey {
    pub workspace_id: String,
    pub query: String,
    pub limit: usize,
    pub filters: String,
}

#[derive(Default)]
pub struct CacheCounters {
    pub hits: AtomicUsize,
    pub misses: AtomicUsize,
}

#[derive(Debug, Clone)]
pub struct QueryBundle {
    pub normalized_query: String,
    pub variants: Vec<String>,
    pub weights: HashMap<String, f32>,
}

impl QueryBundle {
    pub fn weight_for(&self, query: &str) -> f32 {
        self.weights.get(query).copied().unwrap_or(1.0)
    }
}
