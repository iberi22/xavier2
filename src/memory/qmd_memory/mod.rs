//! QMD Memory - lightweight in-memory document store with cached search.

mod types;
mod retrieval;
mod consolidation;
mod embeddings;
mod indexing;

use anyhow::Result;
use std::{
    collections::HashMap,
    sync::Arc,
};
use tokio::sync::RwLock as AsyncRwLock;

use crate::memory::surreal_store::MemoryStore;

// Re-export public API
pub use types::{MemoryDocument, MemoryUsage, CacheMetrics, CachedSearchResult};
pub use indexing::estimate_document_bytes;
pub use consolidation::extract_answer;
pub use embeddings::query_with_embedding;
pub use retrieval::cosine_similarity;

// Internal re-exports for submodules
pub(crate) use types::{SearchCacheKey, CacheCounters};
pub(crate) use embeddings::query_with_embedding_filtered;

#[derive(Clone)]
pub struct QmdMemory {
    pub(crate) workspace_id: String,
    pub(crate) docs: Arc<AsyncRwLock<Vec<MemoryDocument>>>,
    pub(crate) search_cache: Arc<AsyncRwLock<HashMap<SearchCacheKey, Vec<MemoryDocument>>>>,
    pub(crate) cache_counters: Arc<CacheCounters>,
    pub(crate) store: Arc<AsyncRwLock<Option<Arc<dyn MemoryStore>>>>,
}

impl QmdMemory {
    pub fn new(docs: Arc<AsyncRwLock<Vec<MemoryDocument>>>) -> Self {
        Self::new_with_workspace(docs, "default")
    }

    pub fn new_with_workspace(
        docs: Arc<AsyncRwLock<Vec<MemoryDocument>>>,
        workspace_id: impl Into<String>,
    ) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            docs,
            search_cache: Arc::new(AsyncRwLock::new(HashMap::new())),
            cache_counters: Arc::new(CacheCounters::default()),
            store: Arc::new(AsyncRwLock::new(None)),
        }
    }

    pub fn workspace_id(&self) -> &str {
        &self.workspace_id
    }

    pub async fn set_store(&self, store: Arc<dyn MemoryStore>) {
        *self.store.write().await = Some(store);
    }

    async fn store(&self) -> Option<Arc<dyn MemoryStore>> {
        self.store.read().await.clone()
    }

    /// Load workspace state from persistent store on startup.
    pub async fn init(&self) -> Result<()> {
        if let Some(store) = self.store().await {
            let state = store.load_workspace_state(&self.workspace_id).await?;
            let docs: Vec<MemoryDocument> = state
                .memories
                .into_iter()
                .map(|record| record.to_document())
                .collect();
            let loaded_memories = docs.len();
            *self.docs.write().await = docs;
            tracing::info!(
                workspace_id = %self.workspace_id,
                loaded_memories = loaded_memories,
                "QmdMemory loaded from persistent store"
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tokio::sync::RwLock as AsyncRwLock;

    #[tokio::test]
    async fn repeated_searches_hit_cache() {
        let memory = QmdMemory::new(Arc::new(AsyncRwLock::new(Vec::new())));
        memory
            .add_document(
                "docs/cache".to_string(),
                "cache acceleration for repeated searches".to_string(),
                serde_json::json!({}),
            )
            .await
            .unwrap();

        let first = memory
            .search_with_cache("cache acceleration", 5)
            .await
            .unwrap();
        let second = memory
            .search_with_cache("cache acceleration", 5)
            .await
            .unwrap();
        let metrics = memory.cache_metrics().await;

        assert!(!first.cache_hit);
        assert!(second.cache_hit);
        assert_eq!(metrics.misses, 1);
        assert_eq!(metrics.hits, 1);
        assert_eq!(metrics.entries, 1);
    }

    #[tokio::test]
    async fn mutating_memory_invalidates_cache() {
        let memory = QmdMemory::new(Arc::new(AsyncRwLock::new(Vec::new())));
        memory
            .add_document(
                "docs/original".to_string(),
                "performance tuning for xavier2".to_string(),
                serde_json::json!({}),
            )
            .await
            .unwrap();

        let _ = memory.search_with_cache("performance", 5).await.unwrap();
        assert_eq!(memory.cache_metrics().await.entries, 1);

        memory
            .add_document(
                "docs/new".to_string(),
                "new performance tuning guide".to_string(),
                serde_json::json!({}),
            )
            .await
            .unwrap();

        assert_eq!(memory.cache_metrics().await.entries, 0);
    }

    #[tokio::test]
    async fn add_document_skips_embedding_when_service_not_configured() {
        unsafe {
            env::remove_var("XAVIER2_EMBEDDING_URL");
            env::remove_var("XAVIER2_EMBEDDER");
            env::remove_var("OPENAI_API_KEY");
        }

        let memory = QmdMemory::new(Arc::new(AsyncRwLock::new(Vec::new())));
        memory
            .add_document(
                "docs/offline".to_string(),
                "offline startup should not require embeddings".to_string(),
                serde_json::json!({ "source": "test" }),
            )
            .await
            .unwrap();

        let stored = memory.get("docs/offline").await.unwrap().unwrap();
        assert!(stored.embedding.is_empty());
    }

    #[tokio::test]
    async fn add_document_creates_clean_locomo_derivatives() {
        let memory = QmdMemory::new(Arc::new(AsyncRwLock::new(Vec::new())));
        memory
            .add_document(
                "locomo/conv-26/session_1/D1:17".to_string(),
                "Caroline: I've been researching adoption agencies lately.".to_string(),
                serde_json::json!({
                    "benchmark": "locomo",
                    "speaker": "Caroline",
                    "session_time": "8 May, 2023"
                }),
            )
            .await
            .unwrap();

        let stored = memory.all_documents().await;
        assert!(stored.len() > 1);
        let derived = stored
            .iter()
            .find(|doc| {
                doc.metadata.get("memory_kind").and_then(|v| v.as_str()) == Some("fact_atom")
            })
            .expect("derived fact atom");
        assert_eq!(
            derived
                .metadata
                .get("normalized_value")
                .and_then(|v| v.as_str()),
            Some("Adoption agencies")
        );
        assert!(!derived.content.contains("source_path"));
    }

    #[tokio::test]
    async fn locomo_search_prioritizes_temporal_derivatives_over_session_summaries() {
        let memory = QmdMemory::new(Arc::new(AsyncRwLock::new(Vec::new())));
        memory
            .add_document(
                "locomo/conv-26/session_1/summary".to_string(),
                "Caroline and Melanie spoke on 8 May, 2023. Caroline discussed several LGBTQ experiences and many other summer memories.".to_string(),
                serde_json::json!({
                    "benchmark": "locomo",
                    "session_time": "1:56 pm on 8 May, 2023",
                    "category": "session_summary",
                }),
            )
            .await
            .unwrap();
        memory
            .add_document(
                "locomo/conv-26/session_1/D1:3".to_string(),
                "Caroline: I went to a LGBTQ support group yesterday and it was so powerful."
                    .to_string(),
                serde_json::json!({
                    "benchmark": "locomo",
                    "session_time": "1:56 pm on 8 May, 2023",
                    "speaker": "Caroline",
                    "category": "conversation",
                }),
            )
            .await
            .unwrap();

        let results = memory
            .search("When did Caroline go to the LGBTQ support group?", 5)
            .await
            .unwrap();

        assert!(!results.is_empty());
        assert_eq!(
            results[0]
                .metadata
                .get("memory_kind")
                .and_then(|value| value.as_str()),
            Some("temporal_event")
        );
        assert_eq!(
            results[0]
                .metadata
                .get("resolved_date")
                .and_then(|value| value.as_str()),
            Some("7 May 2023")
        );
    }

    #[tokio::test]
    async fn add_document_normalizes_locomo_dia_ids_for_primary_and_derived_docs() {
        let memory = QmdMemory::new(Arc::new(AsyncRwLock::new(Vec::new())));
        memory
            .add_document(
                "locomo/conv-26/session_1/D1:03".to_string(),
                "Caroline: I went to a LGBTQ support group yesterday and it was so powerful."
                    .to_string(),
                serde_json::json!({
                    "benchmark": "locomo",
                    "speaker": "Caroline",
                    "session_time": "1:56 pm on 8 May, 2023",
                    "dia_id": "d1:03",
                    "category": "conversation",
                }),
            )
            .await
            .unwrap();

        let stored = memory.all_documents().await;
        let primary = stored
            .iter()
            .find(|doc| doc.path == "locomo/conv-26/session_1/D1:03")
            .expect("primary locomo document");
        assert_eq!(
            primary
                .metadata
                .get("normalized_dia_id")
                .and_then(|value| value.as_str()),
            Some("D1:3")
        );
        assert_eq!(
            primary
                .metadata
                .get("dia_id")
                .and_then(|value| value.as_str()),
            Some("D1:3")
        );

        let derived = stored
            .iter()
            .find(|doc| doc.path.ends_with("#derived/temporal_event/0"))
            .expect("derived temporal event");
        assert_eq!(
            derived
                .metadata
                .get("source_path")
                .and_then(|value| value.as_str()),
            Some("locomo/conv-26/session_1/D1:3")
        );
        assert_eq!(
            derived
                .metadata
                .get("source_dia_id")
                .and_then(|value| value.as_str()),
            Some("D1:3")
        );
    }

    #[tokio::test]
    async fn hybrid_search_uses_rrf_to_combine_keyword_and_vector_hits() {
        let memory = QmdMemory::new(Arc::new(AsyncRwLock::new(Vec::new())));
        memory
            .add(MemoryDocument {
                id: Some("kw-doc".to_string()),
                path: "docs/keyword".to_string(),
                content: "Alice moved to Paris in 2020 to work as a software engineer.".to_string(),
                metadata: serde_json::json!({}),
                content_vector: Some(vec![0.0, 1.0]),
                embedding: vec![0.0, 1.0],
            })
            .await
            .unwrap();
        memory
            .add(MemoryDocument {
                id: Some("semantic-doc".to_string()),
                path: "docs/semantic".to_string(),
                content:
                    "Alice's favorite programming language is Rust, which she learned in 2021."
                        .to_string(),
                metadata: serde_json::json!({}),
                content_vector: Some(vec![1.0, 0.0]),
                embedding: vec![1.0, 0.0],
            })
            .await
            .unwrap();
        memory
            .add(MemoryDocument {
                id: Some("noise-doc".to_string()),
                path: "docs/noise".to_string(),
                content: "Bob studied design and architecture in Boston.".to_string(),
                metadata: serde_json::json!({}),
                content_vector: Some(vec![0.0, 0.2]),
                embedding: vec![0.0, 0.2],
            })
            .await
            .unwrap();

        let results = memory
            .query_with_hybrid_search("Where did Alice move in 2020?", vec![1.0, 0.0], 3)
            .await
            .unwrap();

        let paths: Vec<&str> = results.iter().map(|doc| doc.path.as_str()).collect();
        assert!(paths.iter().take(2).any(|path| *path == "docs/keyword"));
        assert!(paths.iter().take(2).any(|path| *path == "docs/semantic"));
    }

    #[test]
    fn test_extract_speakers() {
        use crate::memory::qmd_memory::embeddings::extract_speakers;
        let text = "Caroline: Hello\n[James]: Hi\nSpeaker: Alice\nPerson: Robert\nGuest: Emma";
        let speakers = extract_speakers(text);
        assert!(speakers.contains(&"Caroline".to_string()));
        assert!(speakers.contains(&"James".to_string()));
        assert!(speakers.contains(&"Alice".to_string()));
        assert!(speakers.contains(&"Robert".to_string()));
        assert!(speakers.contains(&"Emma".to_string()));
    }

    #[test]
    fn test_extract_speaker_from_query() {
        use crate::memory::qmd_memory::embeddings::extract_speaker_from_query;
        assert_eq!(
            extract_speaker_from_query("Who is Caroline?"),
            Some("Caroline".to_string())
        );
        assert_eq!(
            extract_speaker_from_query("What did James say?"),
            Some("James".to_string())
        );
        assert_eq!(
            extract_speaker_from_query("When was Alice there?"),
            Some("Alice".to_string())
        );
        assert_eq!(
            extract_speaker_from_query("Where is Robert?"),
            Some("Robert".to_string())
        );
        assert_eq!(
            extract_speaker_from_query("Why did Emma laugh?"),
            Some("Emma".to_string())
        );
    }

    #[test]
    fn test_resolve_pronouns() {
        use crate::memory::qmd_memory::embeddings::resolve_pronouns;
        let speakers = vec!["Caroline".to_string(), "James".to_string()];

        // Single female candidate
        assert_eq!(
            resolve_pronouns("What did she say?", &speakers),
            "What did Caroline say?"
        );

        // Single male candidate
        assert_eq!(
            resolve_pronouns("What did he say?", &speakers),
            "What did James say?"
        );

        // Multiple female candidates - no resolution
        let speakers_multiple = vec!["Caroline".to_string(), "Alice".to_string()];
        assert_eq!(
            resolve_pronouns("What did she say?", &speakers_multiple),
            "What did she say?"
        );
    }

    #[test]
    fn test_is_likely_speaker() {
        use crate::memory::qmd_memory::embeddings::is_likely_speaker;
        assert!(is_likely_speaker("Caroline"));
        assert!(is_likely_speaker("James"));
        assert!(!is_likely_speaker("Who"));
        assert!(!is_likely_speaker("What"));
        assert!(!is_likely_speaker("She"));
        assert!(!is_likely_speaker("The"));
    }
}
