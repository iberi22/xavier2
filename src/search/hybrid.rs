use crate::{
    embedding,
    memory::{
        qmd_memory::{MemoryDocument, QmdMemory},
        schema::MemoryQueryFilters,
    },
};

use super::rrf::{reciprocal_rank_fusion, ScoredResult};

#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error("embedding error: {0}")]
    Embedding(String),
    #[error("search error: {0}")]
    Search(String),
}

#[derive(Debug, Clone)]
pub struct HybridSearcher {
    pub keyword_weight: f32,
    pub vector_weight: f32,
    pub rrf_k: u32,
}

impl Default for HybridSearcher {
    fn default() -> Self {
        Self {
            keyword_weight: 0.5,
            vector_weight: 0.5,
            rrf_k: 60,
        }
    }
}

impl HybridSearcher {
    pub async fn search(
        &self,
        memory: &QmdMemory,
        query: &str,
        limit: usize,
        filters: Option<&MemoryQueryFilters>,
    ) -> Result<Vec<ScoredResult>, SearchError> {
        let keyword_results = self.keyword_search(memory, query, limit * 2, filters).await?;
        let vector_results = self
            .vector_search(memory, query, limit * 2, filters)
            .await
            .unwrap_or_default();

        if keyword_results.is_empty() && vector_results.is_empty() {
            return Ok(Vec::new());
        }

        if vector_results.is_empty() {
            return Ok(keyword_results.into_iter().take(limit).collect());
        }
        if keyword_results.is_empty() {
            return Ok(vector_results.into_iter().take(limit).collect());
        }

        let fused = reciprocal_rank_fusion(vec![keyword_results, vector_results], self.rrf_k);
        Ok(fused.into_iter().take(limit).collect())
    }

    async fn keyword_search(
        &self,
        memory: &QmdMemory,
        query: &str,
        limit: usize,
        filters: Option<&MemoryQueryFilters>,
    ) -> Result<Vec<ScoredResult>, SearchError> {
        let documents = memory
            .search_filtered(query, limit, filters)
            .await
            .map_err(|error| SearchError::Search(error.to_string()))?;

        Ok(self
            .convert_documents(documents, "keyword")
            .into_iter()
            .collect())
    }

    async fn vector_search(
        &self,
        memory: &QmdMemory,
        query: &str,
        limit: usize,
        filters: Option<&MemoryQueryFilters>,
    ) -> Result<Vec<ScoredResult>, SearchError> {
        let embedder = embedding::build_embedder_from_env()
            .await
            .map_err(|error| SearchError::Embedding(error.to_string()))?;

        let query_vector = embedder
            .encode(query)
            .await
            .map_err(|error| SearchError::Embedding(error.to_string()))?;

        if query_vector.is_empty() {
            return Ok(Vec::new());
        }

        let documents = memory
            .vsearch(query_vector, limit)
            .await
            .map_err(|error| SearchError::Search(error.to_string()))?
            .into_iter()
            .filter(|doc| {
                filters
                    .map(|filters| {
                        crate::memory::schema::matches_filters(
                            &doc.path,
                            &doc.metadata,
                            memory.workspace_id(),
                            Some(filters),
                        )
                    })
                    .unwrap_or(true)
            })
            .collect::<Vec<_>>();

        Ok(self.convert_documents(documents, "vector"))
    }

    fn convert_documents(
        &self,
        documents: Vec<MemoryDocument>,
        source: &str,
    ) -> Vec<ScoredResult> {
        let total = documents.len().max(1) as f32;
        documents
            .into_iter()
            .enumerate()
            .map(|(index, document)| {
                let score = 1.0 - (index as f32 / total);
                ScoredResult {
                    id: document.id.unwrap_or_else(|| document.path.clone()),
                    content: document.content,
                    score,
                    source: source.to_string(),
                }
            })
            .collect()
    }
}
