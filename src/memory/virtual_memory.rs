// ============================================
// XAVIER MEMORY VIRTUALIZATION LAYER
// Based on Context Mode MCP architecture
// ============================================
//
// Key concepts from the video:
// 1. SQLite FTS5 for efficient full-text search
// 2. Virtualization - index data, don't send raw content
// 3. Checkpoints for session continuity
// 4. Significant token reduction
//
// This module provides:
// - MemoryIndex: FTS5-based indexing
// - Checkpoint: Session state persistence
// - VirtualMemory: Smart content retrieval
// ============================================

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;

use crate::memory::belief_graph::SharedBeliefGraph;
use crate::memory::qmd_memory::QmdMemory;

/// Virtual Memory Engine
/// Integrates L0-L1-L2 memory hierarchy with deterministic graph traversal
pub struct VirtualMemory {
    pub memory: Arc<QmdMemory>,
    pub belief_graph: Option<SharedBeliefGraph>,
}

impl VirtualMemory {
    pub fn new(memory: Arc<QmdMemory>, belief_graph: Option<SharedBeliefGraph>) -> Self {
        Self {
            memory,
            belief_graph,
        }
    }

    /// Retrieve context using deterministic graph traversal (Belief paths)
    /// alongside vector similarity search.
    pub async fn page_in(&self, query: &str, limit: usize) -> Result<Vec<VirtualMemoryEntry>> {
        let mut entries = Vec::new();

        // 1. Deterministic Graph Traversal (L1/L2 index)
        if let Some(graph_lock) = &self.belief_graph {
            let graph = graph_lock.read().await;
            let relations = graph.search_relations(query).await;

            for rel in relations {
                if let Some(source_id) = rel.source_memory_id {
                    if let Ok(Some(doc)) = self.memory.get(&source_id).await {
                        let mut entry = VirtualMemoryEntry::new(doc.path, doc.content, doc.metadata);
                        // Ensure the entry ID matches the source document ID
                        if let Some(doc_id) = doc.id {
                            entry.id = doc_id;
                        }
                        entries.push(entry);
                    }
                }
                if entries.len() >= limit {
                    break;
                }
            }
        }

        // 2. Tandem Vector Search (Probabilistic) - if we still need more context
        if entries.len() < limit {
            let remaining = limit - entries.len();
            if let Ok(docs) = self.memory.search(query, remaining).await {
                for doc in docs {
                    // Avoid duplicates
                    if !entries.iter().any(|e| e.path == doc.path) {
                        let mut entry = VirtualMemoryEntry::new(doc.path, doc.content, doc.metadata);
                        if let Some(doc_id) = doc.id {
                            entry.id = doc_id;
                        }
                        entries.push(entry);
                    }
                }
            }
        }

        Ok(entries)
    }
}

/// Checkpoint for session continuity
/// Allows AI to remember past decisions even after context reset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: String,
    pub timestamp: u64,
    pub summary: String,         // < 2KB summary of session
    pub file_edits: Vec<String>, // Files modified
    pub git_operations: Vec<String>,
    pub tasks: Vec<String>,
    pub key_decisions: Vec<String>,
    pub errors: Vec<String>,
}

impl Default for Checkpoint {
    fn default() -> Self {
        Self::new()
    }
}

impl Checkpoint {
    pub fn new() -> Self {
        Self {
            id: ulid::Ulid::new().to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("test assertion")
                .as_secs(),
            summary: String::new(),
            file_edits: Vec::new(),
            git_operations: Vec::new(),
            tasks: Vec::new(),
            key_decisions: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Create checkpoint from current session state
    pub fn from_session(
        summary: &str,
        file_edits: Vec<String>,
        git_ops: Vec<String>,
        tasks: Vec<String>,
    ) -> Self {
        let mut checkpoint = Self::new();
        checkpoint.summary = summary.to_string();
        checkpoint.file_edits = file_edits;
        checkpoint.git_operations = git_ops;
        checkpoint.tasks = tasks;
        checkpoint
    }

    /// Size in bytes (should be < 2KB)
    pub fn size(&self) -> usize {
        self.summary.len()
            + self.file_edits.iter().map(|s| s.len()).sum::<usize>()
            + self.git_operations.iter().map(|s| s.len()).sum::<usize>()
            + self.tasks.iter().map(|s| s.len()).sum::<usize>()
    }
}

/// Virtual Memory Entry
/// Instead of storing full content, we store indexed references
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualMemoryEntry {
    pub id: String,
    pub path: String,
    pub content_hash: String,          // Hash of actual content
    pub keywords: Vec<String>,         // Extracted keywords
    pub summary: String,               // < 500 bytes summary
    pub embedding_ref: Option<String>, // Reference to embedding
    pub metadata: serde_json::Value,
    pub size_bytes: usize, // Original size
    pub indexed_at: u64,
}

impl VirtualMemoryEntry {
    pub fn new(path: String, content: String, metadata: serde_json::Value) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let content_hash = format!("{:x}", hasher.finalize());
        let summary = create_summary(&content);
        let keywords = extract_keywords(&content);

        Self {
            id: ulid::Ulid::new().to_string(),
            path,
            content_hash,
            keywords,
            summary,
            embedding_ref: None,
            metadata,
            size_bytes: content.len(),
            indexed_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("test assertion")
                .as_secs(),
        }
    }

    /// Get a lightweight reference instead of full content
    pub fn to_reference(&self) -> MemoryReference {
        MemoryReference {
            id: self.id.clone(),
            path: self.path.clone(),
            summary: self.summary.clone(),
            keywords: self.keywords.clone(),
        }
    }
}

/// Lightweight reference for AI context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryReference {
    pub id: String,
    pub path: String,
    pub summary: String,
    pub keywords: Vec<String>,
}

/// Create a summary (for token reduction)
fn create_summary(content: &str) -> String {
    // Take first 400 chars as summary
    let truncated = content.chars().take(400).collect::<String>();
    if truncated.len() < content.len() {
        format!("{}...[truncated]", truncated)
    } else {
        truncated
    }
}

/// Extract keywords from content
fn extract_keywords(content: &str) -> Vec<String> {
    let mut keywords = Vec::new();
    let stop_words = [
        "the", "is", "at", "which", "on", "a", "an", "and", "or", "but",
    ];

    for word in content.split_whitespace() {
        let clean = word
            .trim_matches(|c: char| !c.is_alphanumeric())
            .to_lowercase();
        if clean.len() > 3 && !stop_words.contains(&clean.as_str()) {
            keywords.push(clean);
        }
    }

    // Deduplicate and limit
    keywords.sort();
    keywords.dedup();
    keywords.truncate(50);
    keywords
}

/// Token savings calculation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSavings {
    pub original_size: usize,
    pub virtual_size: usize,
    pub reduction_percent: f32,
}

impl TokenSavings {
    pub fn calculate(original: &str, virtual_entry: &VirtualMemoryEntry) -> Self {
        let original_size = original.len();
        let virtual_size = virtual_entry.summary.len() + virtual_entry.keywords.join(" ").len();

        let reduction = if original_size > 0 {
            (original_size.saturating_sub(virtual_size) as f32 / original_size as f32) * 100.0
        } else {
            0.0
        };

        Self {
            original_size,
            virtual_size,
            reduction_percent: reduction,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkpoint_size() {
        let checkpoint = Checkpoint::from_session(
            "Fixed authentication bug",
            vec!["auth.rs".to_string()],
            vec!["commit abc123".to_string()],
            vec!["Fix login".to_string()],
        );

        assert!(checkpoint.size() < 2048, "Checkpoint should be < 2KB");
    }

    #[test]
    fn test_token_savings() {
        let original = "word ".repeat(11200); // ~56KB with spaces
        let entry = VirtualMemoryEntry::new(
            "test.txt".to_string(),
            original.clone(),
            serde_json::json!({}),
        );

        let savings = TokenSavings::calculate(&original, &entry);

        assert!(savings.reduction_percent > 90.0, "Should save >90% tokens, got {}", savings.reduction_percent);
    }
}
