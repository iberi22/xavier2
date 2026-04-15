use crate::memory::qmd_memory::{MemoryDocument, QmdMemory};
use anyhow::Result;

pub struct QmdSearchTools {
    memory: QmdMemory,
}

impl QmdSearchTools {
    pub fn new(memory: QmdMemory) -> Self {
        Self { memory }
    }

    /// Fast keyword match (BM25)
    pub async fn search(&self, query_text: &str, limit: usize) -> Result<Vec<MemoryDocument>> {
        self.memory.search(query_text, limit).await
    }

    /// Semantic similarity (vector)
    pub async fn vsearch(
        &self,
        query_vector: Vec<f32>,
        limit: usize,
    ) -> Result<Vec<MemoryDocument>> {
        self.memory.vsearch(query_vector, limit).await
    }

    /// Hybrid search + LLM reranking (fusion)
    pub async fn query(
        &self,
        query_text: &str,
        query_vector: Vec<f32>,
        limit: usize,
    ) -> Result<Vec<MemoryDocument>> {
        self.memory.query(query_text, query_vector, limit).await
    }

    /// Retrieve specific document by path or ID
    pub async fn get(&self, path_or_id: &str) -> Result<Option<MemoryDocument>> {
        self.memory.get(path_or_id).await
    }
}

pub fn semantic_search() {}
