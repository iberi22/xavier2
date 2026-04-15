// GitCore SRC Tools for Xavier2
// MCP tools for querying GitCore documentation from Xavier2

use serde::{Deserialize, Serialize};
use crate::memory::store::MemoryStore;

/// Get SRC context for a query
/// Queries Xavier2 memory for documentation related to the query
#[derive(Debug, Serialize, Deserialize)]
pub struct GetSrcContextRequest {
    pub project_id: String,
    pub query: String,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SrcContextResult {
    pub doc_id: String,
    pub title: String,
    pub content: String,
    pub score: f32,
    pub module: String,
    pub doc_type: String,
}

/// Search SRC documents by keywords
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchSrcDocumentsRequest {
    pub project_id: String,
    pub keywords: Vec<String>,
    pub doc_type: Option<String>,
    pub module: Option<String>,
}

/// Get module specification
#[derive(Debug, Serialize, Deserialize)]
pub struct GetModuleSpecRequest {
    pub project_id: String,
    pub module: String,
}

/// List all SRC documents for a project
#[derive(Debug, Serialize, Deserialize)]
pub struct ListProjectSrcRequest {
    pub project_id: String,
    pub include_content: Option<bool>,
}

/// GitCore-specific memory tools
pub mod gitcore_tools {
    use super::*;

    /// Query Xavier2 for SRC context
    pub async fn get_src_context(
        store: &MemoryStore,
        project_id: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SrcContextResult>, String> {
        // Search in memory store
        let results = store.search(
            query,
            Some(&format!("project_id:{}", project_id)),
            Some(&["src".to_string()]),
            limit,
        ).await.map_err(|e| e.to_string())?;

        Ok(results.into_iter().map(|r| SrcContextResult {
            doc_id: r.id.clone(),
            title: r.title.unwrap_or_default(),
            content: r.text,
            score: r.score.unwrap_or(0.0),
            module: r.metadata.get("module")
                .cloned()
                .unwrap_or_else(|| "core".to_string()),
            doc_type: r.metadata.get("doc_type")
                .cloned()
                .unwrap_or_else(|| "general".to_string()),
        }).collect())
    }

    /// Search SRC documents by keywords
    pub async fn search_src_documents(
        store: &MemoryStore,
        project_id: &str,
        keywords: &[String],
        doc_type: Option<&str>,
        module: Option<&str>,
    ) -> Result<Vec<SrcContextResult>, String> {
        let query = keywords.join(" ");
        let mut filters = vec![format!("project_id:{}", project_id)];

        if let Some(dt) = doc_type {
            filters.push(format!("doc_type:{}", dt));
        }
        if let Some(m) = module {
            filters.push(format!("module:{}", m));
        }

        let results = store.search(
            &query,
            Some(&filters),
            Some(&["src".to_string()]),
            20,
        ).await.map_err(|e| e.to_string())?;

        Ok(results.into_iter().map(|r| SrcContextResult {
            doc_id: r.id.clone(),
            title: r.title.unwrap_or_default(),
            content: r.text,
            score: r.score.unwrap_or(0.0),
            module: r.metadata.get("module")
                .cloned()
                .unwrap_or_else(|| "core".to_string()),
            doc_type: r.metadata.get("doc_type")
                .cloned()
                .unwrap_or_else(|| "general".to_string()),
        }).collect())
    }

    /// Get module specification
    pub async fn get_module_spec(
        store: &MemoryStore,
        project_id: &str,
        module: &str,
    ) -> Result<Vec<SrcContextResult>, String> {
        let filters = vec![
            format!("project_id:{}", project_id),
            format!("module:{}", module),
        ];

        let results = store.search(
            "",
            Some(&filters),
            Some(&["src".to_string()]),
            10,
        ).await.map_err(|e| e.to_string())?;

        Ok(results.into_iter().map(|r| SrcContextResult {
            doc_id: r.id.clone(),
            title: r.title.unwrap_or_default(),
            content: r.text,
            score: r.score.unwrap_or(0.0),
            module: module.to_string(),
            doc_type: r.metadata.get("doc_type")
                .cloned()
                .unwrap_or_else(|| "general".to_string()),
        }).collect())
    }

    /// List all SRC documents for a project
    pub async fn list_project_src(
        store: &MemoryStore,
        project_id: &str,
        include_content: bool,
    ) -> Result<Vec<SrcContextResult>, String> {
        let filters = vec![format!("project_id:{}", project_id)];

        let results = store.search(
            "",
            Some(&filters),
            Some(&["src".to_string()]),
            100,
        ).await.map_err(|e| e.to_string())?;

        Ok(results.into_iter().map(|r| {
            let content = if include_content {
                r.text
            } else {
                String::new()
            };

            SrcContextResult {
                doc_id: r.id.clone(),
                title: r.title.unwrap_or_default(),
                content,
                score: r.score.unwrap_or(0.0),
                module: r.metadata.get("module")
                    .cloned()
                    .unwrap_or_else(|| "core".to_string()),
                doc_type: r.metadata.get("doc_type")
                    .cloned()
                    .unwrap_or_else(|| "general".to_string()),
            }
        }).collect())
    }
}
