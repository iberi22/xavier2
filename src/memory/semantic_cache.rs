use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::memory::embedder::EmbeddingClient;
use crate::memory::qmd_memory::cosine_similarity;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResponse {
    pub query: String,
    pub query_embedding: Vec<f32>,
    pub response: String,
    pub confidence: f32, // Used to track how good this cached answer was
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub trait QueryEmbedder: Send + Sync {
    fn embed<'a>(
        &'a self,
        input: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<f32>>> + Send + 'a>>;
}

impl QueryEmbedder for EmbeddingClient {
    fn embed<'a>(
        &'a self,
        input: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<f32>>> + Send + 'a>> {
        Box::pin(async move { EmbeddingClient::embed(self, input).await })
    }
}

struct NoopQueryEmbedder;

impl QueryEmbedder for NoopQueryEmbedder {
    fn embed<'a>(
        &'a self,
        _input: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<f32>>> + Send + 'a>> {
        Box::pin(async { Ok(Vec::new()) })
    }
}

pub struct SemanticCache {
    entries: Arc<RwLock<Vec<CachedResponse>>>,
    similarity_threshold: f32,
    embedder: Arc<dyn QueryEmbedder>,
}

impl SemanticCache {
    pub fn new(similarity_threshold: f32) -> Result<Self> {
        let embedder: Arc<dyn QueryEmbedder> = match EmbeddingClient::from_env() {
            Ok(client) => Arc::new(client),
            Err(error) => {
                tracing::warn!(
                    "SemanticCache embedding backend unavailable, using no-op embedder: {}",
                    error
                );
                Arc::new(NoopQueryEmbedder)
            }
        };

        Ok(Self {
            entries: Arc::new(RwLock::new(Vec::new())),
            similarity_threshold,
            embedder,
        })
    }

    #[cfg(test)]
    pub fn new_with_embedder(similarity_threshold: f32, embedder: Arc<dyn QueryEmbedder>) -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
            similarity_threshold,
            embedder,
        }
    }

    pub async fn get(&self, query: &str) -> Result<Option<String>> {
        let query_embedding = self.embedder.embed(query).await?;
        if query_embedding.is_empty() {
            return Ok(None);
        }

        let entries = self.entries.read().await;

        let mut best_match = None;
        let mut max_similarity = 0.0;

        for entry in entries.iter() {
            let similarity = cosine_similarity(&query_embedding, &entry.query_embedding);
            if similarity > max_similarity {
                max_similarity = similarity;
                best_match = Some(entry);
            }
        }

        if max_similarity >= self.similarity_threshold {
            if let Some(match_entry) = best_match {
                tracing::info!(
                    "🎯 Semantic Cache HIT! Similarity: {:.2}. Query: '{}' matched '{}'",
                    max_similarity,
                    query,
                    match_entry.query
                );
                return Ok(Some(match_entry.response.clone()));
            }
        }

        tracing::info!("❌ Semantic Cache MISS for query: '{}'", query);
        Ok(None)
    }

    pub async fn put(&self, query: &str, response: &str) -> Result<()> {
        let query_embedding = self.embedder.embed(query).await?;
        if query_embedding.is_empty() {
            return Ok(());
        }

        let mut entries = self.entries.write().await;
        entries.push(CachedResponse {
            query: query.to_string(),
            query_embedding,
            response: response.to_string(),
            confidence: 1.0,
            timestamp: chrono::Utc::now(),
        });

        tracing::info!("💾 Stored answer in Semantic Cache for query: '{}'", query);
        Ok(())
    }

    pub async fn clear(&self) {
        self.entries.write().await.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct MockEmbedder {
        embeddings: HashMap<String, Vec<f32>>,
    }

    impl QueryEmbedder for MockEmbedder {
        fn embed<'a>(
            &'a self,
            input: &'a str,
        ) -> Pin<Box<dyn Future<Output = Result<Vec<f32>>> + Send + 'a>> {
            Box::pin(async move { Ok(self.embeddings.get(input).cloned().unwrap_or_default()) })
        }
    }

    fn cache() -> SemanticCache {
        let embeddings = HashMap::from([
            ("hello".to_string(), vec![1.0, 0.0]),
            ("hello again".to_string(), vec![0.99, 0.01]),
            ("different".to_string(), vec![0.0, 1.0]),
        ]);

        SemanticCache::new_with_embedder(0.95, Arc::new(MockEmbedder { embeddings }))
    }

    #[tokio::test]
    async fn returns_cached_response_on_high_similarity() {
        let cache = cache();
        cache.put("hello", "cached").await.expect("cache put");

        let result = cache.get("hello again").await.expect("cache get");
        assert_eq!(result.as_deref(), Some("cached"));
    }

    #[tokio::test]
    async fn misses_when_similarity_is_below_threshold() {
        let cache = cache();
        cache.put("hello", "cached").await.expect("cache put");

        let result = cache.get("different").await.expect("cache get");
        assert!(result.is_none());
    }
}
