//! Phase 4 consolidation layer.
//!
//! This module provides consolidation, decay, importance scoring, and reflection
//! on top of the existing memory store.

pub mod merger;
pub mod reflection;

use anyhow::Result;
use chrono::Utc;
use serde::Serialize;
use std::collections::HashSet;
use tracing::{info, warn};

use crate::{
    memory::{
        manager::ManagedMemory,
        qmd_memory::MemoryDocument,
        schema::{EvidenceKind, MemoryKind, TypedMemoryPayload},
    },
    workspace::WorkspaceContext,
};

#[derive(Debug, Clone)]
pub struct ConsolidationTask {
    pub batch_size: usize,
    pub similarity_threshold: f32,
    pub decay_rate: f32,
    pub min_importance_for_decay: f32,
    pub reflection_batch_size: usize,
    pub reflection_age_days: i64,
    pub cleanup_similarity_threshold: f32,
}

impl Default for ConsolidationTask {
    fn default() -> Self {
        Self {
            batch_size: 32,
            similarity_threshold: 0.88,
            decay_rate: 0.94,
            min_importance_for_decay: 0.30,
            reflection_batch_size: 8,
            reflection_age_days: 30,
            cleanup_similarity_threshold: 0.91,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ConsolidationStats {
    pub selected: usize,
    pub grouped: usize,
    pub merged_documents: usize,
    pub decayed_documents: usize,
    pub deleted_redundant_documents: usize,
    pub importance_updates: usize,
    pub errors: usize,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ReflectionStats {
    pub selected: usize,
    pub summarized_documents: usize,
    pub summary_documents_created: usize,
    pub redundant_documents_removed: usize,
    pub llm_used: bool,
    pub errors: usize,
    pub duration_ms: u64,
}

impl ConsolidationTask {
    pub async fn consolidate(&self, workspace: &WorkspaceContext) -> Result<ConsolidationStats> {
        let start = std::time::Instant::now();
        let mut stats = ConsolidationStats::default();
        let memories = workspace
            .workspace
            .memory_manager
            .get_all_memories()
            .await?;
        let mut selected = memories;
        selected.sort_by(|left, right| {
            right
                .quality
                .overall
                .partial_cmp(&left.quality.overall)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| right.access_count.cmp(&left.access_count))
                .then_with(|| left.doc.path.cmp(&right.doc.path))
        });
        let selected: Vec<ManagedMemory> = selected.into_iter().take(self.batch_size).collect();
        stats.selected = selected.len();

        let memory = workspace.workspace.memory_manager.memory();
        let clusters = merger::cluster_similar_memories(&selected, self.similarity_threshold);
        stats.grouped = clusters.iter().filter(|cluster| cluster.len() > 1).count();

        let mut seen_ids = HashSet::new();
        let mut removed_ids = HashSet::new();
        for cluster in clusters {
            if cluster.len() < 2 {
                continue;
            }

            let mut docs = Vec::new();
            for managed in cluster {
                let maybe_id = managed.doc.id.clone();
                if let Some(id) = maybe_id {
                    if seen_ids.insert(id) {
                        docs.push(managed);
                    }
                }
            }

            if docs.len() < 2 {
                continue;
            }

            let merge = merger::merge_documents(&docs)?;
            if let Err(error) = memory.update(merge.canonical.clone()).await {
                warn!(%error, "failed to persist merged memory");
                stats.errors += 1;
                continue;
            }

            for redundant in merge.redundant_ids {
                removed_ids.insert(redundant.clone());
                if memory.delete(&redundant).await.is_ok() {
                    stats.deleted_redundant_documents += 1;
                } else {
                    stats.errors += 1;
                }
            }

            stats.merged_documents += docs.len().saturating_sub(1);
            stats.importance_updates += 1;
        }

        let mut decay_updates = 0usize;
        for managed in selected {
            let Some(doc_id) = managed.doc.id.clone() else {
                continue;
            };
            if removed_ids.contains(&doc_id) {
                continue;
            }

            let importance = merger::importance_score(
                managed.access_count,
                managed.last_access,
                managed.created_at,
                &managed.doc.metadata,
            );
            let decayed = merger::decay_importance(
                importance,
                managed.last_access,
                managed.created_at,
                self.decay_rate,
            );

            if decayed < self.min_importance_for_decay {
                if memory.delete(&doc_id).await.is_ok() {
                    stats.deleted_redundant_documents += 1;
                    decay_updates += 1;
                } else {
                    stats.errors += 1;
                }
                continue;
            }

            if (decayed - importance).abs() >= 0.01 {
                let mut updated = managed.doc.clone();
                updated.metadata["memory_importance"] = serde_json::json!(decayed);
                updated.metadata["memory_decay_rate"] = serde_json::json!(self.decay_rate);
                updated.metadata["memory_last_consolidated_at"] =
                    serde_json::json!(Utc::now().to_rfc3339());
                if memory.update(updated).await.is_ok() {
                    decay_updates += 1;
                    stats.importance_updates += 1;
                } else {
                    stats.errors += 1;
                }
            }
        }
        stats.decayed_documents = decay_updates;

        stats.duration_ms = start.elapsed().as_millis() as u64;
        info!(
            processed = stats.selected,
            merged = stats.merged_documents,
            decayed = stats.decayed_documents,
            deleted = stats.deleted_redundant_documents,
            "memory consolidation complete"
        );
        Ok(stats)
    }

    pub async fn reflect(&self, workspace: &WorkspaceContext) -> Result<ReflectionStats> {
        let start = std::time::Instant::now();
        let mut stats = ReflectionStats::default();
        let memories = workspace
            .workspace
            .memory_manager
            .get_all_memories()
            .await?;
        let mut candidates: Vec<ManagedMemory> = memories
            .into_iter()
            .filter(|memory| {
                let importance = merger::importance_score(
                    memory.access_count,
                    memory.last_access,
                    memory.created_at,
                    &memory.doc.metadata,
                );
                let age_days = merger::age_days(memory.last_access, memory.created_at);
                importance < 0.65 || age_days >= self.reflection_age_days as f32
            })
            .collect();

        candidates.sort_by(|left, right| {
            left.quality
                .overall
                .partial_cmp(&right.quality.overall)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| right.access_count.cmp(&left.access_count))
        });
        candidates.truncate(self.reflection_batch_size);
        stats.selected = candidates.len();
        if candidates.is_empty() {
            stats.duration_ms = start.elapsed().as_millis() as u64;
            return Ok(stats);
        }

        let docs: Vec<MemoryDocument> =
            candidates.iter().map(|memory| memory.doc.clone()).collect();
        let reflection = reflection::reflect_memories(&docs).await?;
        stats.llm_used = reflection.llm_used;
        stats.summarized_documents = docs.len();

        let summary_path = format!(
            "reflections/{}/{}",
            workspace.workspace_id,
            Utc::now().format("%Y%m%dT%H%M%SZ")
        );
        workspace
            .workspace
            .memory
            .add_document_typed(
                summary_path,
                reflection.summary.clone(),
                serde_json::json!({
                    "memory_priority": "high",
                    "memory_importance": 0.86,
                    "memory_reflection": true,
                    "reflection_sources": docs.iter().filter_map(|doc| doc.id.clone()).collect::<Vec<_>>(),
                    "reflection_themes": reflection.themes,
                    "reflection_notes": reflection.notes,
                    "reflection_generated_at": Utc::now().to_rfc3339(),
                }),
                Some(TypedMemoryPayload {
                    kind: Some(MemoryKind::Document),
                    evidence_kind: Some(EvidenceKind::SummaryFact),
                    namespace: None,
                    provenance: None,
                }),
            )
            .await?;
        stats.summary_documents_created = 1;

        for candidate in candidates {
            let Some(candidate_id) = candidate.doc.id.as_ref() else {
                continue;
            };
            let should_remove = reflection
                .cleanup_targets
                .iter()
                .any(|target| target == candidate_id)
                || merger::similarity_to_summary(&candidate.doc.content, &reflection.summary)
                    >= self.cleanup_similarity_threshold;
            if should_remove
                && workspace
                    .workspace
                    .memory
                    .delete(candidate_id)
                    .await
                    .is_ok()
            {
                stats.redundant_documents_removed += 1;
            }
        }

        stats.duration_ms = start.elapsed().as_millis() as u64;
        info!(
            selected = stats.selected,
            removed = stats.redundant_documents_removed,
            "memory reflection complete"
        );
        Ok(stats)
    }
}
