//! Memory Manager - Intelligent Memory Management System
//!
//! Provides autonomous memory lifecycle management:
//! - Memory Prioritization (Critical → Ephemeral)
//! - Memory Decay based on access time
//! - Memory Quality Scoring
//! - Memory Consolidation (deduplication)
//! - Intelligent Forgetting/Eviction

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

use crate::memory::qmd_memory::{MemoryDocument, QmdMemory};

/// Memory priority levels - determines retention policy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryPriority {
    /// BELA's profile, client data, key decisions - NEVER evict
    Critical = 0,
    /// Project status, technical decisions - very long retention
    High = 1,
    /// Operations, cron jobs, monitoring - standard retention
    Medium = 2,
    /// Raw logs, temporary data - short retention
    Low = 3,
    /// Can be forgotten immediately after TTL
    Ephemeral = 4,
}

impl MemoryPriority {
    pub fn from_metadata(metadata: &serde_json::Value) -> Self {
        metadata
            .get("memory_priority")
            .and_then(|v| v.as_str())
            .and_then(|s| match s {
                "critical" => Some(Self::Critical),
                "high" => Some(Self::High),
                "medium" => Some(Self::Medium),
                "low" => Some(Self::Low),
                "ephemeral" => Some(Self::Ephemeral),
                _ => None,
            })
            .unwrap_or(Self::Medium)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
            Self::Ephemeral => "ephemeral",
        }
    }

    /// Returns base decay factor for this priority (higher = decay slower)
    pub fn decay_base(&self) -> f32 {
        match self {
            Self::Critical => 1.0,  // No decay
            Self::High => 0.98,     // 2% decay per day
            Self::Medium => 0.95,   // 5% decay per day
            Self::Low => 0.85,      // 15% decay per day
            Self::Ephemeral => 0.5, // 50% decay per day
        }
    }

    /// Maximum age in days before eviction candidate
    pub fn max_age_days(&self) -> f64 {
        match self {
            Self::Critical => 365.0 * 10.0, // 10 years
            Self::High => 365.0,            // 1 year
            Self::Medium => 90.0,           // 90 days
            Self::Low => 14.0,              // 14 days
            Self::Ephemeral => 1.0,         // 1 day
        }
    }

    /// Minimum relevance score before eviction
    pub fn min_relevance(&self) -> f32 {
        match self {
            Self::Critical => 0.0, // Never evict based on relevance
            Self::High => 0.1,
            Self::Medium => 0.2,
            Self::Low => 0.3,
            Self::Ephemeral => 0.5,
        }
    }
}

/// Memory Quality Score - composite score for retention decisions
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryQuality {
    /// 0-1 based on access frequency and priority
    pub relevance_score: f32,
    /// 0-1 based on belief graph verifications
    pub accuracy_score: f32,
    /// 0-1 based on time since last access
    pub freshness_score: f32,
    /// 0-1 based on metadata completeness
    pub completeness_score: f32,
    /// Weighted composite score
    pub overall: f32,
}

impl MemoryQuality {
    /// Weights for composite score
    const RELEVANCE_WEIGHT: f32 = 0.40;
    const ACCURACY_WEIGHT: f32 = 0.25;
    const FRESHNESS_WEIGHT: f32 = 0.20;
    const COMPLETENESS_WEIGHT: f32 = 0.15;

    pub fn calculate(
        doc: &MemoryDocument,
        priority: MemoryPriority,
        access_count: usize,
        last_access: Option<DateTime<Utc>>,
        verified: bool,
    ) -> Self {
        // Relevance: access frequency + priority boost
        let base_relevance = (access_count as f32 * 0.1).min(1.0);
        let priority_boost = match priority {
            MemoryPriority::Critical => 1.0,
            MemoryPriority::High => 0.8,
            MemoryPriority::Medium => 0.6,
            MemoryPriority::Low => 0.4,
            MemoryPriority::Ephemeral => 0.2,
        };
        let relevance_score = (base_relevance * 0.6 + priority_boost * 0.4).min(1.0);

        // Accuracy: based on verification in belief graph and memory level
        let level_accuracy = match doc.level {
            crate::memory::schema::MemoryLevel::Belief => 1.0,
            crate::memory::schema::MemoryLevel::Extracted => 0.8,
            crate::memory::schema::MemoryLevel::Processed => 0.7,
            crate::memory::schema::MemoryLevel::Raw => 0.5,
        };
        let accuracy_score = if verified { 1.0 } else { level_accuracy };

        // Freshness: based on days since last access
        let freshness_score = if let Some(last) = last_access {
            let days_since = (Utc::now() - last).num_days() as f32;
            let max_days = priority.max_age_days() as f32;
            (1.0 - days_since / max_days).clamp(0.0, 1.0)
        } else {
            // No access record = assume fresh
            0.8
        };

        // Completeness: based on metadata fields
        let completeness_score = {
            let meta = &doc.metadata;
            let mut score = 0.0;
            let mut count = 0;
            for key in ["kind", "namespace", "provenance", "source_path"] {
                if meta.get(key).is_some() {
                    score += 1.0;
                }
                count += 1;
            }
            if count > 0 {
                score / count as f32
            } else {
                0.5
            }
        };

        let overall = Self::RELEVANCE_WEIGHT * relevance_score
            + Self::ACCURACY_WEIGHT * accuracy_score
            + Self::FRESHNESS_WEIGHT * freshness_score
            + Self::COMPLETENESS_WEIGHT * completeness_score;

        Self {
            relevance_score,
            accuracy_score,
            freshness_score,
            completeness_score,
            overall: overall.clamp(0.0, 1.0),
        }
    }
}

/// Memory entry with metadata for management decisions
#[derive(Debug, Clone)]
pub struct ManagedMemory {
    pub doc: MemoryDocument,
    pub priority: MemoryPriority,
    pub quality: MemoryQuality,
    pub access_count: usize,
    pub last_access: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
    pub size_bytes: u64,
}

/// Memory statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    pub total_documents: usize,
    pub total_size_bytes: u64,
    pub by_priority: HashMap<String, usize>,
    pub by_quality_bucket: HashMap<String, usize>,
    pub low_quality_count: usize,
    pub ephemeral_count: usize,
    pub decayed_count: usize,
}

/// Action taken by memory manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryManagementAction {
    Decayed {
        doc_id: String,
        old_relevance: f32,
        new_relevance: f32,
    },
    Consolidated {
        doc_ids: Vec<String>,
        into_doc_id: String,
    },
    Evicted {
        doc_id: String,
        reason: String,
        priority: String,
    },
    Compressed {
        doc_id: String,
        old_size: u64,
        new_size: u64,
    },
    Archived {
        doc_id: String,
        archive_path: String,
    },
    Promoted {
        doc_id: String,
        old_priority: String,
        new_priority: String,
    },
    Demoted {
        doc_id: String,
        old_priority: String,
        new_priority: String,
    },
}

/// Result of a management operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagementResult {
    pub actions: Vec<MemoryManagementAction>,
    pub documents_affected: usize,
    pub bytes_freed: u64,
}

/// Configuration for memory manager
#[derive(Debug, Clone)]
pub struct MemoryManagerConfig {
    /// Maximum documents before eviction triggers
    pub max_documents: usize,
    /// Maximum storage bytes before eviction triggers
    pub max_storage_bytes: u64,
    /// Quality threshold below which documents are evicted
    pub quality_threshold: f32,
    /// Enable automatic decay
    pub auto_decay_enabled: bool,
    /// Enable automatic consolidation
    pub auto_consolidate_enabled: bool,
    /// Enable automatic eviction
    pub auto_evict_enabled: bool,
    /// Decay factor for all memories (can override per-priority)
    pub global_decay_factor: f32,
    /// Run auto-management every N hours
    pub auto_manage_interval_hours: u32,
    /// Compress memories larger than this size
    pub compression_threshold_bytes: usize,
}

impl Default for MemoryManagerConfig {
    fn default() -> Self {
        Self {
            max_documents: 10000,
            max_storage_bytes: 500 * 1024 * 1024, // 500MB
            quality_threshold: 0.25,
            auto_decay_enabled: true,
            auto_consolidate_enabled: true,
            auto_evict_enabled: true,
            global_decay_factor: 0.97,
            auto_manage_interval_hours: 24,
            compression_threshold_bytes: 2 * 1024, // 2KB
        }
    }
}

/// Intelligent Memory Manager - manages memory lifecycle autonomously
pub struct MemoryManager {
    memory: Arc<QmdMemory>,
    _belief_graph: Option<crate::memory::belief_graph::SharedBeliefGraph>,
    config: MemoryManagerConfig,
    /// Track access counts per document
    access_counts: std::sync::Mutex<HashMap<String, usize>>,
    /// Track last access times
    last_access_times: std::sync::Mutex<HashMap<String, DateTime<Utc>>>,
    /// Track created times
    created_times: std::sync::Mutex<HashMap<String, DateTime<Utc>>>,
    /// Relevance scores (can be decayed over time)
    relevance_scores: std::sync::Mutex<HashMap<String, f32>>,
}

impl MemoryManager {
    pub fn new(
        memory: Arc<QmdMemory>,
        belief_graph: Option<crate::memory::belief_graph::SharedBeliefGraph>,
    ) -> Self {
        Self {
            memory,
            _belief_graph: belief_graph,
            config: MemoryManagerConfig::default(),
            access_counts: std::sync::Mutex::new(HashMap::new()),
            last_access_times: std::sync::Mutex::new(HashMap::new()),
            created_times: std::sync::Mutex::new(HashMap::new()),
            relevance_scores: std::sync::Mutex::new(HashMap::new()),
        }
    }

    pub fn with_config(
        memory: Arc<QmdMemory>,
        belief_graph: Option<crate::memory::belief_graph::SharedBeliefGraph>,
        config: MemoryManagerConfig,
    ) -> Self {
        Self {
            memory,
            _belief_graph: belief_graph,
            config,
            access_counts: std::sync::Mutex::new(HashMap::new()),
            last_access_times: std::sync::Mutex::new(HashMap::new()),
            created_times: std::sync::Mutex::new(HashMap::new()),
            relevance_scores: std::sync::Mutex::new(HashMap::new()),
        }
    }

    /// Get configuration
    pub fn config(&self) -> &MemoryManagerConfig {
        &self.config
    }

    /// Expose the shared memory store so consolidation/reflection can mutate it.
    pub fn memory(&self) -> Arc<QmdMemory> {
        Arc::clone(&self.memory)
    }

    /// Update configuration
    pub fn set_config(&mut self, config: MemoryManagerConfig) {
        self.config = config;
    }

    /// Record a memory access for tracking
    pub fn record_access(&self, doc_id: &str) {
        let mut counts = self
            .access_counts
            .lock()
            .expect("manager: access_counts lock poisoned");
        *counts.entry(doc_id.to_string()).or_insert(0) += 1;

        let mut times = self
            .last_access_times
            .lock()
            .expect("manager: last_access_times lock poisoned");
        times.insert(doc_id.to_string(), Utc::now());
    }

    /// Initialize tracking for a new document
    pub fn track_new_document(&self, doc: &MemoryDocument) {
        if let Some(id) = &doc.id {
            let mut times = self
                .created_times
                .lock()
                .expect("manager: created_times lock poisoned");
            times.insert(id.clone(), Utc::now());

            let mut relevance = self
                .relevance_scores
                .lock()
                .expect("manager: relevance_scores lock poisoned");
            relevance.insert(id.clone(), 1.0); // Start at full relevance
        }
    }

    /// Get statistics about all memories
    pub async fn get_stats(&self) -> Result<MemoryStats> {
        let docs = self.memory.all_documents().await;
        let mut stats = MemoryStats {
            total_documents: docs.len(),
            total_size_bytes: docs.iter().map(|d| d.estimated_bytes()).sum(),
            by_priority: HashMap::new(),
            by_quality_bucket: HashMap::new(),
            low_quality_count: 0,
            ephemeral_count: 0,
            decayed_count: 0,
        };

        let counts = self
            .access_counts
            .lock()
            .expect("manager: access_counts lock poisoned");
        let times = self
            .last_access_times
            .lock()
            .expect("manager: last_access_times lock poisoned");

        for doc in docs {
            let priority = MemoryPriority::from_metadata(&doc.metadata);
            *stats
                .by_priority
                .entry(priority.as_str().to_string())
                .or_insert(0) += 1;

            let access_count = doc
                .id
                .as_ref()
                .and_then(|id| counts.get(id))
                .copied()
                .unwrap_or(0);
            let last_access = doc.id.as_ref().and_then(|id| times.get(id)).copied();

            let mut verified = false;
            if let (Some(graph_lock), Some(doc_id)) = (&self._belief_graph, &doc.id) {
                verified = graph_lock.read().await.has_supporting_beliefs(doc_id).await;
            }

            let quality =
                MemoryQuality::calculate(&doc, priority, access_count, last_access, verified);

            let bucket = if quality.overall >= 0.7 {
                "high"
            } else if quality.overall >= 0.4 {
                "medium"
            } else {
                "low"
            };
            *stats
                .by_quality_bucket
                .entry(bucket.to_string())
                .or_insert(0) += 1;

            if quality.overall < self.config.quality_threshold {
                stats.low_quality_count += 1;
            }
            if priority == MemoryPriority::Ephemeral {
                stats.ephemeral_count += 1;
            }
        }

        Ok(stats)
    }

    /// Get all managed memories with their quality scores
    pub async fn get_all_memories(&self) -> Result<Vec<ManagedMemory>> {
        let docs = self.memory.all_documents().await;
        let counts = self
            .access_counts
            .lock()
            .expect("manager: access_counts lock poisoned");
        let times = self
            .last_access_times
            .lock()
            .expect("manager: last_access_times lock poisoned");
        let created = self
            .created_times
            .lock()
            .expect("manager: created_times lock poisoned");
        let _relevance = self
            .relevance_scores
            .lock()
            .expect("manager: relevance_scores lock poisoned");

        let mut memories = Vec::new();
        for doc in docs {
            let priority = MemoryPriority::from_metadata(&doc.metadata);
            let access_count = doc
                .id
                .as_ref()
                .and_then(|id| counts.get(id))
                .copied()
                .unwrap_or(0);
            let last_access = doc.id.as_ref().and_then(|id| times.get(id)).copied();
            let created_at = doc.id.as_ref().and_then(|id| created.get(id)).copied();

            let mut verified = false;
            if let (Some(graph_lock), Some(doc_id)) = (&self._belief_graph, &doc.id) {
                verified = graph_lock.read().await.has_supporting_beliefs(doc_id).await;
            }

            let quality =
                MemoryQuality::calculate(&doc, priority, access_count, last_access, verified);

            memories.push(ManagedMemory {
                doc,
                priority,
                quality,
                access_count,
                last_access,
                created_at,
                size_bytes: 0, // Will be computed if needed
            });
        }

        Ok(memories)
    }

    /// Get memories below quality threshold
    pub async fn get_low_quality_memories(&self, threshold: f32) -> Result<Vec<ManagedMemory>> {
        let all = self.get_all_memories().await?;
        Ok(all
            .into_iter()
            .filter(|m| m.quality.overall < threshold)
            .collect())
    }

    /// Get memories by priority
    pub async fn get_memories_by_priority(
        &self,
        priority: MemoryPriority,
    ) -> Result<Vec<ManagedMemory>> {
        let all = self.get_all_memories().await?;
        Ok(all.into_iter().filter(|m| m.priority == priority).collect())
    }

    /// Apply decay to all memories based on time since last access
    pub async fn decay_memories(&self) -> Result<ManagementResult> {
        let docs = self.memory.all_documents().await;
        let mut actions = Vec::new();
        let mut decayed_count = 0;
        let mut relevance_map = self
            .relevance_scores
            .lock()
            .expect("manager: relevance_scores lock poisoned");

        for doc in docs {
            let Some(doc_id) = &doc.id else {
                continue;
            };
            let priority = MemoryPriority::from_metadata(&doc.metadata);
            let last_access = self
                .last_access_times
                .lock()
                .expect("manager: last_access_times lock poisoned")
                .get(doc_id)
                .copied();
            let created_at = self
                .created_times
                .lock()
                .expect("manager: created_times lock poisoned")
                .get(doc_id)
                .copied();

            // Calculate days since last access or creation
            let reference_time = last_access.or(created_at).unwrap_or_else(Utc::now);
            let days_since = (Utc::now() - reference_time).num_days() as f32;

            // Apply decay: relevance = base_relevance * decay_factor^(days_since)
            // Using priority-specific decay base
            let decay_base = priority.decay_base();
            let old_relevance = *relevance_map.get(doc_id).unwrap_or(&1.0);
            let new_relevance = old_relevance * decay_base.powf(days_since);

            if (old_relevance - new_relevance).abs() > 0.001 {
                relevance_map.insert(doc_id.clone(), new_relevance);
                actions.push(MemoryManagementAction::Decayed {
                    doc_id: doc_id.clone(),
                    old_relevance,
                    new_relevance,
                });
                decayed_count += 1;
            }
        }

        info!(
            "Decay applied to {} memories (threshold: {})",
            decayed_count, self.config.quality_threshold
        );

        Ok(ManagementResult {
            documents_affected: decayed_count,
            actions,
            bytes_freed: 0,
        })
    }

    /// Consolidate similar memories - merge duplicates and near-duplicates
    pub async fn consolidate_memories(&self) -> Result<ManagementResult> {
        let docs = self.memory.all_documents().await;
        let mut actions = Vec::new();
        let mut bytes_freed: u64 = 0;
        let mut seen_signatures: HashMap<String, String> = HashMap::new(); // signature -> doc_id

        for doc in docs {
            let Some(doc_id) = &doc.id else {
                continue;
            };

            // Create a signature for deduplication (normalized content hash)
            let signature = self.create_consolidation_signature(&doc);

            if let Some(existing_id) = seen_signatures.get(&signature) {
                // Duplicate found - keep the more recent one, archive the older one
                let existing_time = self
                    .created_times
                    .lock()
                    .expect("manager: created_times lock poisoned")
                    .get(existing_id)
                    .copied()
                    .unwrap_or_else(Utc::now);
                let doc_time = self
                    .created_times
                    .lock()
                    .expect("manager: created_times lock poisoned")
                    .get(doc_id)
                    .copied()
                    .unwrap_or_else(Utc::now);

                if doc_time < existing_time {
                    // Keep current doc, archive existing
                    bytes_freed += doc.estimated_bytes();
                    actions.push(MemoryManagementAction::Consolidated {
                        doc_ids: vec![existing_id.clone(), doc_id.clone()],
                        into_doc_id: doc_id.clone(),
                    });
                    // Delete the older duplicate
                    if self.memory.delete(existing_id).await?.is_some() {
                        info!("Consolidated duplicate {} into {}", existing_id, doc_id);
                    }
                } else {
                    // Keep existing, archive current
                    bytes_freed += doc.estimated_bytes();
                    actions.push(MemoryManagementAction::Consolidated {
                        doc_ids: vec![doc_id.clone(), existing_id.clone()],
                        into_doc_id: existing_id.clone(),
                    });
                    if self.memory.delete(doc_id).await?.is_some() {
                        info!("Consolidated duplicate {} into {}", doc_id, existing_id);
                    }
                }
            } else {
                seen_signatures.insert(signature, doc_id.clone());
            }
        }

        info!(
            "Consolidation complete: {} groups merged, {} bytes freed",
            actions.len(),
            bytes_freed
        );

        Ok(ManagementResult {
            documents_affected: actions.len(),
            actions,
            bytes_freed,
        })
    }

    /// Create a normalized signature for consolidation detection
    fn create_consolidation_signature(&self, doc: &MemoryDocument) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // Normalize content: lowercase, trim, remove extra whitespace
        let normalized: String = doc
            .content
            .to_lowercase()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");

        // Also include key metadata
        let kind = doc
            .metadata
            .get("kind")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let priority = doc
            .metadata
            .get("memory_priority")
            .and_then(|v| v.as_str())
            .unwrap_or("medium");

        let mut hasher = DefaultHasher::new();
        (normalized.len(), kind, priority).hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Evict memories based on quality threshold and priority
    pub async fn evict_low_quality(&self) -> Result<ManagementResult> {
        let threshold = self.config.quality_threshold;
        let low_quality = self.get_low_quality_memories(threshold).await?;

        let mut actions = Vec::new();
        let mut bytes_freed: u64 = 0;
        let mut evicted_count = 0;

        for memory in low_quality {
            let Some(doc_id) = &memory.doc.id else {
                continue;
            };
            let priority = memory.priority;

            // Never evict Critical memories
            if priority == MemoryPriority::Critical {
                continue;
            }

            // Check if we should evict based on priority rules
            let should_evict = match priority {
                MemoryPriority::Critical => false,
                MemoryPriority::High => memory.quality.overall < 0.1,
                MemoryPriority::Medium => memory.quality.overall < threshold,
                MemoryPriority::Low => true, // Always evict low priority low quality
                MemoryPriority::Ephemeral => true, // Always evict ephemeral when low quality
            };

            if should_evict {
                let size = memory.doc.estimated_bytes();
                if self.memory.delete(doc_id).await?.is_some() {
                    bytes_freed += size;
                    evicted_count += 1;
                    actions.push(MemoryManagementAction::Evicted {
                        doc_id: doc_id.clone(),
                        reason: format!(
                            "Quality {} below threshold {}",
                            memory.quality.overall, threshold
                        ),
                        priority: priority.as_str().to_string(),
                    });
                    info!(
                        "Evicted memory {} (priority={}, quality={:.2})",
                        doc_id,
                        priority.as_str(),
                        memory.quality.overall
                    );
                }
            }
        }

        info!(
            "Eviction complete: {} evicted, {} bytes freed",
            evicted_count, bytes_freed
        );

        Ok(ManagementResult {
            documents_affected: evicted_count,
            actions,
            bytes_freed,
        })
    }

    /// Evict memories by specific priority level
    pub async fn evict_by_priority(&self, priority: MemoryPriority) -> Result<ManagementResult> {
        let memories = self.get_memories_by_priority(priority).await?;

        let mut actions = Vec::new();
        let mut bytes_freed: u64 = 0;
        let mut evicted_count = 0;

        for memory in memories {
            let Some(doc_id) = &memory.doc.id else {
                continue;
            };

            // Never evict Critical
            if priority == MemoryPriority::Critical {
                info!("Skipping eviction of critical memory {}", doc_id);
                continue;
            }

            let size = memory.doc.estimated_bytes();
            if self.memory.delete(doc_id).await?.is_some() {
                bytes_freed += size;
                evicted_count += 1;
                actions.push(MemoryManagementAction::Evicted {
                    doc_id: doc_id.clone(),
                    reason: format!("Manual eviction by priority={}", priority.as_str()),
                    priority: priority.as_str().to_string(),
                });
            }
        }

        info!(
            "Priority eviction ({}): {} evicted, {} bytes freed",
            priority.as_str(),
            evicted_count,
            bytes_freed
        );

        Ok(ManagementResult {
            documents_affected: evicted_count,
            actions,
            bytes_freed,
        })
    }

    /// Compress large memories by splitting or summarizing
    pub async fn compress_large_memories(&self) -> Result<ManagementResult> {
        let docs = self.memory.all_documents().await;
        let threshold = self.config.compression_threshold_bytes;
        let mut actions = Vec::new();
        let mut bytes_freed: u64 = 0;

        for doc in docs {
            let size = doc.content.len();
            if size > threshold {
                // For now, we'll mark large memories for compression
                // In a full implementation, this would call an LLM to summarize
                let Some(doc_id) = &doc.id else {
                    continue;
                };

                // Simple compression: truncate to threshold with ellipsis
                let compressed_content = if doc.content.len() > threshold {
                    format!(
                        "{}...[compressed from {} chars]",
                        &doc.content[..threshold.saturating_sub(20)],
                        doc.content.len()
                    )
                } else {
                    doc.content.clone()
                };

                let old_size = doc.content.len() as u64;
                let new_size = compressed_content.len() as u64;
                let freed = old_size.saturating_sub(new_size);

                if freed > 0 {
                    let mut updated_doc = doc.clone();
                    updated_doc.content = compressed_content;
                    updated_doc.metadata["compressed"] = serde_json::json!(true);
                    updated_doc.metadata["original_size"] = serde_json::json!(old_size);

                    if self.memory.update(updated_doc).await.is_ok() {
                        bytes_freed += freed;
                        actions.push(MemoryManagementAction::Compressed {
                            doc_id: doc_id.clone(),
                            old_size,
                            new_size,
                        });
                    }
                }
            }
        }

        info!(
            "Compression complete: {} compressed, {} bytes freed",
            actions.len(),
            bytes_freed
        );

        Ok(ManagementResult {
            documents_affected: actions.len(),
            actions,
            bytes_freed,
        })
    }

    /// Full auto-management cycle: decay → consolidate → evict
    pub async fn auto_manage(&self) -> Result<usize> {
        let mut total_actions = 0;

        // 1. Apply decay
        if self.config.auto_decay_enabled {
            let decay_result = self.decay_memories().await?;
            total_actions += decay_result.documents_affected;
        }

        // 2. Consolidate duplicates
        if self.config.auto_consolidate_enabled {
            let consolidate_result = self.consolidate_memories().await?;
            total_actions += consolidate_result.documents_affected;
        }

        // 3. Evict low quality
        if self.config.auto_evict_enabled {
            let evict_result = self.evict_low_quality().await?;
            total_actions += evict_result.documents_affected;
        }

        // 4. Check storage limits
        let stats = self.get_stats().await?;
        if stats.total_size_bytes > self.config.max_storage_bytes {
            info!(
                "Storage limit exceeded ({} > {}), triggering aggressive eviction",
                stats.total_size_bytes, self.config.max_storage_bytes
            );
            let ratio = stats.total_size_bytes as f64 / self.config.max_storage_bytes as f64;
            // Evict more aggressively based on how much over limit
            let extra_threshold = self.config.quality_threshold * (ratio as f32);
            let low_quality = self.get_low_quality_memories(extra_threshold).await?;
            for memory in low_quality {
                if memory.priority != MemoryPriority::Critical {
                    if let Some(doc_id) = &memory.doc.id {
                        let _ = self.memory.delete(doc_id).await;
                        total_actions += 1;
                    }
                }
            }
        }

        info!(
            "Auto-manage cycle complete: {} total actions",
            total_actions
        );
        Ok(total_actions)
    }

    /// Promote a memory's priority
    pub async fn promote_memory(&self, doc_id: &str, new_priority: MemoryPriority) -> Result<()> {
        if let Some(mut doc) = self.memory.get(doc_id).await? {
            let old_priority = MemoryPriority::from_metadata(&doc.metadata);
            doc.metadata["memory_priority"] = serde_json::json!(new_priority.as_str());
            self.memory.update(doc).await?;

            let mut relevance = self
                .relevance_scores
                .lock()
                .expect("manager: relevance_scores lock poisoned");
            let current = relevance.get(doc_id).copied().unwrap_or(1.0);
            // Boost relevance on promotion
            relevance.insert(doc_id.to_string(), (current * 1.2).min(1.0));

            info!(
                "Promoted memory {} from {} to {}",
                doc_id,
                old_priority.as_str(),
                new_priority.as_str()
            );
        }
        Ok(())
    }

    /// Demote a memory's priority
    pub async fn demote_memory(&self, doc_id: &str, new_priority: MemoryPriority) -> Result<()> {
        if let Some(mut doc) = self.memory.get(doc_id).await? {
            let old_priority = MemoryPriority::from_metadata(&doc.metadata);
            doc.metadata["memory_priority"] = serde_json::json!(new_priority.as_str());
            self.memory.update(doc).await?;

            let mut relevance = self
                .relevance_scores
                .lock()
                .expect("manager: relevance_scores lock poisoned");
            let current = relevance.get(doc_id).copied().unwrap_or(1.0);
            // Reduce relevance on demotion
            relevance.insert(doc_id.to_string(), current * 0.8);

            info!(
                "Demoted memory {} from {} to {}",
                doc_id,
                old_priority.as_str(),
                new_priority.as_str()
            );
        }
        Ok(())
    }
}

// Backwards compatibility alias
pub use MemoryManager as XavierMemoryManager;

/// Legacy action types for backwards compatibility with existing code
#[derive(Debug, Clone)]
pub enum MemoryAction {
    Keep,
    Compress {
        doc_id: String,
        reason: String,
    },
    Delete {
        doc_id: String,
        reason: String,
    },
    Update {
        doc_id: String,
        new_content: String,
    },
    Consolidate {
        doc_ids: Vec<String>,
        reason: String,
    },
    Curate {
        doc_id: String,
    },
}

impl MemoryManager {
    /// Execute legacy action types for backwards compatibility
    pub async fn execute_actions(&self, actions: Vec<MemoryAction>) -> Result<usize> {
        let mut executed = 0;

        for action in actions {
            match action {
                MemoryAction::Delete { doc_id, reason } => {
                    info!("Deleting document {}: {}", doc_id, reason);
                    if self.memory.delete(&doc_id).await?.is_some() {
                        executed += 1;
                    }
                }
                MemoryAction::Compress { doc_id, reason } => {
                    info!("Compressing document {}: {}", doc_id, reason);
                    // Legacy compression - just mark as compressed
                    if let Some(mut doc) = self.memory.get(&doc_id).await? {
                        doc.metadata["compressed"] = serde_json::json!(true);
                        doc.metadata["compression_reason"] = serde_json::json!(reason);
                        let _ = self.memory.update(doc).await;
                        executed += 1;
                    }
                }
                MemoryAction::Update {
                    doc_id,
                    new_content,
                } => {
                    if let Some(mut doc) = self.memory.get(&doc_id).await? {
                        doc.content = new_content;
                        if self.memory.update(doc).await.is_ok() {
                            executed += 1;
                        }
                    }
                }
                MemoryAction::Curate { doc_id } => {
                    if let Some(mut doc) = self.memory.get(&doc_id).await? {
                        // Simple curation: ensure metadata has required fields
                        if let Some(meta) = doc.metadata.as_object_mut() {
                            if !meta.contains_key("memory_priority") {
                                meta.insert(
                                    "memory_priority".to_string(),
                                    serde_json::json!("medium"),
                                );
                            }
                            if !meta.contains_key("curated") {
                                meta.insert("curated".to_string(), serde_json::json!(true));
                                meta.insert(
                                    "curated_at".to_string(),
                                    serde_json::json!(chrono::Utc::now().to_rfc3339()),
                                );
                            }
                        }
                        if self.memory.update(doc).await.is_ok() {
                            executed += 1;
                        }
                    }
                }
                MemoryAction::Consolidate { doc_ids, reason } => {
                    info!(
                        "Consolidating documents: {} - {}",
                        doc_ids.join(", "),
                        reason
                    );
                    // Legacy consolidation just logs, actual consolidation done by consolidate_memories()
                    executed += 1;
                }
                MemoryAction::Keep => {
                    // No-op
                }
            }
        }

        Ok(executed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_from_metadata() {
        let critical_meta = serde_json::json!({"memory_priority": "critical"});
        assert_eq!(
            MemoryPriority::from_metadata(&critical_meta),
            MemoryPriority::Critical
        );

        let default_meta = serde_json::json!({});
        assert_eq!(
            MemoryPriority::from_metadata(&default_meta),
            MemoryPriority::Medium
        );
    }

    #[test]
    fn test_quality_calculation() {
        let doc = MemoryDocument {
            id: Some("test".to_string()),
            path: "test/path".to_string(),
            content: "Test content".to_string(),
            metadata: serde_json::json!({"kind": "fact"}),
            content_vector: Some(vec![0.0; 384]),
            embedding: vec![0.0; 384],
            ..Default::default()
        };

        let quality = MemoryQuality::calculate(
            &doc,
            MemoryPriority::Medium,
            5,
            Some(chrono::Utc::now()),
            true,
        );

        assert!(quality.overall >= 0.0 && quality.overall <= 1.0);
        assert!(quality.accuracy_score == 1.0); // verified = true
    }

    #[test]
    fn test_decay_calculation() {
        // Critical should not decay
        assert!((MemoryPriority::Critical.decay_base() - 1.0).abs() < 0.001);

        // Ephemeral decays fast
        assert!(MemoryPriority::Ephemeral.decay_base() < 0.6);
    }
}
