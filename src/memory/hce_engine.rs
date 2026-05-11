use std::sync::Arc;
use anyhow::Result;
use crate::memory::store::{MemoryRecord, MemoryStore};
use super::schema::{MemoryLevel, RelationKind};
use uuid::Uuid;
use chrono::Utc;
use tracing::{info, debug};
use std::collections::HashMap;

/// Hierarchical Context Engine (HCE)
/// Responsible for evolving raw memories into a structured hierarchical graph.
pub struct HceEngine {
    store: Arc<dyn MemoryStore>,
}

impl HceEngine {
    pub fn new(store: Arc<dyn MemoryStore>) -> Self {
        Self { store }
    }

    /// Process a workspace to build/update its hierarchical context tree.
    pub async fn process_workspace(&self, workspace_id: &str) -> Result<()> {
        info!("Starting HCE process for workspace: {}", workspace_id);

        // 1. Structural Analysis: Raw -> Section
        // Identify functions, classes, and meaningful blocks.
        self.map_structural_elements(workspace_id).await?;

        // 2. Hierarchical Clustering: Section -> Global
        // Group sections into thematic clusters (Communities).
        self.perform_hierarchical_clustering(workspace_id).await?;

        // 3. Recursive Summarization
        // Generate abstractive summaries for clusters.
        self.generate_community_summaries(workspace_id).await?;

        info!("HCE process completed for workspace: {}", workspace_id);
        Ok(())
    }

    /// Maps raw file memories into structural sections (functions, classes, etc.)
    async fn map_structural_elements(&self, workspace_id: &str) -> Result<()> {
        debug!("HCE: Mapping structural elements for {}", workspace_id);
        
        let all_memories = self.store.list(workspace_id).await?;
        
        // Filter for raw file memories
        let raw_files: Vec<_> = all_memories.iter()
            .filter(|m| m.level == MemoryLevel::Raw && m.path.ends_with(".rs")) // Start with Rust
            .collect();

        for raw in raw_files {
            // Naive sectioning by blank-line boundaries until tree-sitter / code-graph is wired.
            // Future: replace with code_graph::indexer::Indexer for AST-level sections.
            self.decompose_file(workspace_id, raw).await?;
        }

        Ok(())
    }

    async fn decompose_file(&self, workspace_id: &str, raw: &MemoryRecord) -> Result<()> {
        // This is where we would call tree-sitter or code-graph indexer
        // For the MVP, we'll just log and assume some sections exist if the file is large.
        if raw.content.len() > 1000 {
            debug!("Decomposing large file: {}", raw.path);
            // Split by double newline as a naive sectioning for now
            let sections = raw.content.split("\n\n").filter(|s| s.trim().len() > 100);
            
            for (i, content) in sections.enumerate() {
                let section_path = format!("{}:section-{}", raw.path, i);
                
                // Check if already exists
                if self.store.get(workspace_id, &section_path).await?.is_some() {
                    continue;
                }

                let section = MemoryRecord {
                    id: format!("mem_{}", Uuid::new_v4()),
                    workspace_id: workspace_id.to_string(),
                    path: section_path,
                    content: content.to_string(),
                    metadata: serde_json::json!({
                        "kind": "code_section",
                        "parent_file": raw.path,
                        "offset": i
                    }),
                    embedding: Vec::new(), // Will be filled by embedder
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    revision: 1,
                    primary: true,
                    parent_id: Some(raw.id.clone()),
                    cluster_id: None,
                    level: MemoryLevel::Section,
                    relation: Some(RelationKind::Contains),
                    revisions: Vec::new(),
                };

                // In a real system, we'd trigger embedding here or wait for background embedder
                self.store.put(section).await?;
            }
        }
        Ok(())
    }

    /// Clusters Section memories into Global communities
    async fn perform_hierarchical_clustering(&self, workspace_id: &str) -> Result<()> {
        debug!("HCE: Performing hierarchical clustering for {}", workspace_id);
        
        let all_memories = self.store.list(workspace_id).await?;
        let sections: Vec<_> = all_memories.iter()
            .filter(|m| m.level == MemoryLevel::Section)
            .collect();

        if sections.is_empty() {
            return Ok(());
        }

        // Simple clustering logic: group by parent directory or module
        let mut clusters: HashMap<String, Vec<String>> = HashMap::new();
        for section in sections {
            let base_path = section.path.split(':').next().unwrap_or(&section.path);
            let dir = std::path::Path::new(base_path)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "root".to_string());
            clusters.entry(dir).or_default().push(section.path.clone());
        }

        for (cluster_name, members) in clusters {
            let cluster_id = format!("cluster_{}_{}", workspace_id, cluster_name);
            
            for member_path in members {
                if let Some(mut record) = self.store.get(workspace_id, &member_path).await? {
                    record.cluster_id = Some(cluster_id.clone());
                    self.store.update(record).await?;
                }
            }
        }

        Ok(())
    }

    /// Generates summaries for each cluster/community
    async fn generate_community_summaries(&self, workspace_id: &str) -> Result<()> {
        debug!("HCE: Generating community summaries for {}", workspace_id);
        
        // Find unique cluster_ids
        let all_memories = self.store.list(workspace_id).await?;
        let mut cluster_members: HashMap<String, Vec<&MemoryRecord>> = HashMap::new();
        
        for m in &all_memories {
            if let Some(cid) = &m.cluster_id {
                cluster_members.entry(cid.clone()).or_default().push(m);
            }
        }

        for (cluster_id, members) in cluster_members {
            let summary_path = format!("summary/{}", cluster_id);
            
            if self.store.get(workspace_id, &summary_path).await?.is_some() {
                continue;
            }

            // Combine contents for summarization (limited)
            let mut combined_content = String::new();
            for m in members.iter().take(5) { // Limit to 5 members for now
                combined_content.push_str(&format!("--- {} ---\n{}\n", m.path, m.content));
            }

            // Deterministic template summary; an LLM-based abstractive pass is a follow-up.
            let summary_content = format!("Community Summary for {}: Includes {} elements such as {}.", 
                cluster_id, 
                members.len(),
                members.first().map(|m| m.path.as_str()).unwrap_or("unknown")
            );

            let summary_record = MemoryRecord {
                id: format!("mem_{}", Uuid::new_v4()),
                workspace_id: workspace_id.to_string(),
                path: summary_path,
                content: summary_content,
                metadata: serde_json::json!({
                    "kind": "community_summary",
                    "cluster_id": cluster_id,
                    "member_count": members.len()
                }),
                embedding: Vec::new(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                revision: 1,
                primary: true,
                parent_id: None,
                cluster_id: Some(cluster_id.clone()),
                level: MemoryLevel::Global,
                relation: None,
                revisions: Vec::new(),
            };

            self.store.put(summary_record).await?;
        }

        Ok(())
    }
}
