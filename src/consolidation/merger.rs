use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::memory::{
    manager::ManagedMemory,
    qmd_memory::{cosine_similarity, MemoryDocument},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeOutcome {
    pub canonical: MemoryDocument,
    pub redundant_ids: Vec<String>,
    pub similarity: f32,
}

pub fn cluster_similar_memories(
    memories: &[ManagedMemory],
    similarity_threshold: f32,
) -> Vec<Vec<ManagedMemory>> {
    let mut clusters = Vec::new();
    let mut used = HashSet::new();

    for (index, seed) in memories.iter().enumerate() {
        let Some(seed_id) = seed.doc.id.as_ref() else {
            continue;
        };
        if used.contains(seed_id) {
            continue;
        }

        let mut cluster = vec![seed.clone()];
        used.insert(seed_id.clone());

        for candidate in memories.iter().skip(index + 1) {
            let Some(candidate_id) = candidate.doc.id.as_ref() else {
                continue;
            };
            if used.contains(candidate_id) {
                continue;
            }

            if similarity(&seed.doc, &candidate.doc) >= similarity_threshold {
                cluster.push(candidate.clone());
                used.insert(candidate_id.clone());
            }
        }

        clusters.push(cluster);
    }

    clusters
}

pub fn merge_documents(memories: &[ManagedMemory]) -> Result<MergeOutcome> {
    if memories.is_empty() {
        bail!("cannot merge an empty memory set");
    }

    let mut ordered: Vec<ManagedMemory> = memories.to_vec();
    ordered.sort_by(|left, right| {
        right
            .quality
            .overall
            .partial_cmp(&left.quality.overall)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| right.access_count.cmp(&left.access_count))
            .then_with(|| left.doc.path.cmp(&right.doc.path))
    });

    let canonical_source = ordered[0].clone();
    let canonical_id = canonical_source.doc.id.clone().unwrap_or_default();
    let mut combined_sentences = extract_sentences(&canonical_source.doc.content);
    let mut redundant_ids = Vec::new();
    let mut best_similarity = 1.0f32;

    for memory in ordered.iter().skip(1) {
        let similarity = similarity(&canonical_source.doc, &memory.doc);
        best_similarity = best_similarity.min(similarity);
        redundant_ids.push(memory.doc.id.clone().unwrap_or_default());

        for sentence in extract_sentences(&memory.doc.content) {
            if !combined_sentences
                .iter()
                .any(|existing| normalize_text(existing) == normalize_text(&sentence))
            {
                combined_sentences.push(sentence);
            }
        }
    }

    let mut canonical = canonical_source.doc.clone();
    canonical.content = cleanup_redundant_text(&combined_sentences.join(" "));
    canonical.metadata["memory_merged"] = serde_json::json!(true);
    canonical.metadata["memory_merged_from"] = serde_json::json!(memories
        .iter()
        .filter_map(|memory| memory.doc.id.clone())
        .collect::<Vec<_>>());
    canonical.metadata["memory_merge_count"] = serde_json::json!(memories.len());
    let importance = importance_score(
        canonical_source.access_count,
        canonical_source.last_access,
        canonical_source.created_at,
        &canonical.metadata,
    );
    canonical.metadata["memory_importance"] = serde_json::json!(importance);
    canonical.metadata["memory_last_consolidated_at"] = serde_json::json!(Utc::now().to_rfc3339());
    if canonical.id.is_none() {
        canonical.id = Some(canonical_id);
    }

    Ok(MergeOutcome {
        canonical,
        redundant_ids,
        similarity: best_similarity,
    })
}

pub fn similarity(left: &MemoryDocument, right: &MemoryDocument) -> f32 {
    let semantic = match (&left.content_vector, &right.content_vector) {
        (Some(a), Some(b)) if !a.is_empty() && !b.is_empty() => cosine_similarity(a, b),
        _ => 0.0,
    };
    let lexical = lexical_similarity(&left.content, &right.content);
    let path_boost = if left.path == right.path {
        1.0
    } else if normalize_text(&left.path) == normalize_text(&right.path) {
        0.95
    } else {
        0.0
    };

    (semantic * 0.50 + lexical * 0.40 + path_boost * 0.10).clamp(0.0, 1.0)
}

pub fn importance_score(
    access_count: usize,
    last_access: Option<DateTime<Utc>>,
    created_at: Option<DateTime<Utc>>,
    metadata: &serde_json::Value,
) -> f32 {
    let access_component = 1.0 - (-((access_count as f32) / 4.0)).exp();
    let recency_component = recency_boost(last_access);
    let age_component = age_boost(created_at);
    let priority_boost = priority_boost(metadata);

    (0.35 * access_component
        + 0.25 * recency_component
        + 0.20 * age_component
        + 0.20 * priority_boost)
        .clamp(0.0, 1.0)
}

pub fn decay_importance(
    importance: f32,
    last_access: Option<DateTime<Utc>>,
    created_at: Option<DateTime<Utc>>,
    decay_rate: f32,
) -> f32 {
    let age = age_days(last_access, created_at);
    let usage_decay = 1.0 / (1.0 + age / 20.0);
    let mut score = importance * decay_rate.powf(age / 7.0) * usage_decay;

    if let Some(last_access) = last_access {
        let days_since_access = (Utc::now() - last_access).num_days().max(0) as f32;
        score *= 1.0 / (1.0 + days_since_access / 30.0);
    }

    score.clamp(0.0, 1.0)
}

pub fn age_days(last_access: Option<DateTime<Utc>>, created_at: Option<DateTime<Utc>>) -> f32 {
    let reference = last_access.or(created_at).unwrap_or_else(Utc::now);
    (Utc::now() - reference).num_seconds().max(0) as f32 / 86_400.0
}

pub fn similarity_to_summary(content: &str, summary: &str) -> f32 {
    lexical_similarity(content, summary)
}

pub fn cleanup_redundant_text(content: &str) -> String {
    let mut seen = HashSet::new();
    let mut output = Vec::new();

    for sentence in extract_sentences(content) {
        let normalized = normalize_text(&sentence);
        if normalized.is_empty() || !seen.insert(normalized) {
            continue;
        }
        output.push(sentence.trim().to_string());
    }

    if output.is_empty() {
        normalize_whitespace(content)
    } else {
        output.join(". ")
    }
}

fn lexical_similarity(left: &str, right: &str) -> f32 {
    let left_tokens = tokenize(left);
    let right_tokens = tokenize(right);
    if left_tokens.is_empty() || right_tokens.is_empty() {
        return 0.0;
    }

    let left_set: HashSet<_> = left_tokens.into_iter().collect();
    let right_set: HashSet<_> = right_tokens.into_iter().collect();
    let intersection = left_set.intersection(&right_set).count() as f32;
    let union = left_set.union(&right_set).count() as f32;
    if union == 0.0 {
        0.0
    } else {
        intersection / union
    }
}

fn tokenize(value: &str) -> Vec<String> {
    value
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|token| token.len() >= 3)
        .map(ToOwned::to_owned)
        .collect()
}

fn extract_sentences(value: &str) -> Vec<String> {
    value
        .split(['.', '\n', '!', '?'])
        .map(str::trim)
        .filter(|sentence| !sentence.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn normalize_text(value: &str) -> String {
    normalize_whitespace(&value.to_lowercase())
}

fn normalize_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn recency_boost(last_access: Option<DateTime<Utc>>) -> f32 {
    match last_access {
        Some(last_access) => {
            let days = (Utc::now() - last_access).num_days().max(0) as f32;
            1.0 / (1.0 + days / 14.0)
        }
        None => 0.45,
    }
}

fn age_boost(created_at: Option<DateTime<Utc>>) -> f32 {
    match created_at {
        Some(created_at) => {
            let days = (Utc::now() - created_at).num_days().max(0) as f32;
            1.0 / (1.0 + days / 60.0)
        }
        None => 0.70,
    }
}

fn priority_boost(metadata: &serde_json::Value) -> f32 {
    match metadata
        .get("memory_priority")
        .and_then(|value| value.as_str())
        .unwrap_or("medium")
    {
        "critical" => 1.0,
        "high" => 0.85,
        "medium" => 0.60,
        "low" => 0.35,
        "ephemeral" => 0.10,
        _ => 0.60,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn memory(path: &str, content: &str, importance: f32) -> ManagedMemory {
        let doc = MemoryDocument {
            id: Some(path.to_string()),
            path: path.to_string(),
            content: content.to_string(),
            metadata: serde_json::json!({
                "memory_priority": "medium",
                "memory_importance": importance
            }),
            content_vector: None,
            embedding: Vec::new(),
        };

        ManagedMemory {
            doc,
            priority: crate::memory::manager::MemoryPriority::Medium,
            quality: Default::default(),
            access_count: 3,
            last_access: Some(Utc::now()),
            created_at: Some(Utc::now()),
            size_bytes: 0,
        }
    }

    #[test]
    fn clusters_similar_memories() {
        let memories = vec![
            memory("a", "BELA built Xavier for memory consolidation", 0.8),
            memory("b", "BELA built Xavier for memory consolidation", 0.7),
            memory("c", "Different note about deployment", 0.2),
        ];

        // Lexical-only matches top out at 0.4 when no path/vector similarity is present.
        let clusters = cluster_similar_memories(&memories, 0.4);
        assert!(clusters.iter().any(|cluster| cluster.len() == 2));
        assert!(clusters.iter().any(|cluster| cluster.len() == 1));
    }

    #[test]
    fn importance_increases_with_access_pattern() {
        let metadata = serde_json::json!({"memory_priority": "high"});
        let low = importance_score(0, None, None, &metadata);
        let high = importance_score(8, Some(Utc::now()), Some(Utc::now()), &metadata);
        assert!(high > low);
    }

    #[test]
    fn cleanup_removes_duplicate_sentences() {
        let text = "Alpha. Alpha. Beta.";
        let cleaned = cleanup_redundant_text(text);
        assert!(cleaned.contains("Alpha"));
        assert!(cleaned.contains("Beta"));
        assert_eq!(cleaned.matches("Alpha").count(), 1);
    }
}
