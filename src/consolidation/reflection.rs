use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{
    agents::{provider::ModelProviderClient, system1::RetrievedDocument},
    consolidation::merger,
    memory::qmd_memory::MemoryDocument,
};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReflectionResult {
    pub summary: String,
    pub themes: Vec<String>,
    pub notes: Vec<String>,
    pub cleanup_targets: Vec<String>,
    pub llm_used: bool,
}

pub async fn reflect_memories(memories: &[MemoryDocument]) -> Result<ReflectionResult> {
    if memories.is_empty() {
        return Ok(ReflectionResult::default());
    }

    let provider = ModelProviderClient::from_env();
    let context = build_context(memories);
    let prompt = "Reflect on the provided memory context. Produce a concise synthesis of the recurring facts, tensions, and next-useful summary. Return plain text with a short overview followed by bullet points for themes and cleanup suggestions.";

    if let Ok(summary) = provider.generate_response(prompt, &context).await {
        let parsed = parse_reflection_text(&summary, memories);
        return Ok(ReflectionResult {
            llm_used: true,
            ..parsed
        });
    }

    Ok(fallback_reflection(memories))
}

fn build_context(memories: &[MemoryDocument]) -> Vec<RetrievedDocument> {
    memories
        .iter()
        .enumerate()
        .map(|(index, memory)| {
            let content = memory.content.clone();
            let token_count = content.split_whitespace().count();
            RetrievedDocument {
                id: memory
                    .id
                    .clone()
                    .unwrap_or_else(|| format!("reflection-{}", index)),
                path: memory.path.clone(),
                content,
                relevance_score: memory
                    .metadata
                    .get("memory_importance")
                    .and_then(|value| value.as_f64())
                    .unwrap_or(0.5) as f32,
                token_count,
                metadata: memory.metadata.clone(),
            }
        })
        .collect()
}

fn parse_reflection_text(summary: &str, memories: &[MemoryDocument]) -> ReflectionResult {
    let cleanup_targets = identify_cleanup_targets(summary, memories);
    let themes = summarize_themes(memories);
    let notes = summary
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();

    ReflectionResult {
        summary: summary.trim().to_string(),
        themes,
        notes,
        cleanup_targets,
        llm_used: false,
    }
}

fn fallback_reflection(memories: &[MemoryDocument]) -> ReflectionResult {
    let themes = summarize_themes(memories);
    let cleanup_targets = identify_cleanup_targets(&themes.join(" "), memories);
    let summary = format!(
        "Reflection generated at {}. Core themes: {}. The oldest or least distinct memories can be compressed or removed after the synthesis is stored.",
        Utc::now().to_rfc3339(),
        themes.join(", ")
    );

    ReflectionResult {
        summary,
        themes,
        notes: vec![
            "Fallback reflection used because no LLM provider was available.".to_string(),
            "The summary prioritizes repeated terms and low-entropy memories.".to_string(),
        ],
        cleanup_targets,
        llm_used: false,
    }
}

fn summarize_themes(memories: &[MemoryDocument]) -> Vec<String> {
    let mut counts: HashMap<String, usize> = HashMap::new();

    for token in memories.iter().flat_map(|memory| tokenize(&memory.content)) {
        *counts.entry(token).or_insert(0) += 1;
    }

    let mut ranked: Vec<(String, usize)> = counts
        .into_iter()
        .filter(|(token, count)| token.len() >= 4 && *count > 1)
        .collect();
    ranked.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));

    ranked.into_iter().take(5).map(|(token, _)| token).collect()
}

fn identify_cleanup_targets(summary: &str, memories: &[MemoryDocument]) -> Vec<String> {
    memories
        .iter()
        .filter_map(|memory| {
            let similarity = merger::similarity_to_summary(&memory.content, summary);
            if similarity >= 0.78 {
                memory.id.clone()
            } else {
                None
            }
        })
        .collect()
}

fn tokenize(value: &str) -> Vec<String> {
    value
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|token| token.len() >= 3)
        .map(ToOwned::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_reflection_produces_summary() {
        let memories = vec![MemoryDocument {
            id: Some("1".to_string()),
            path: "notes/1".to_string(),
            content: "BELA is the developer of SWAL".to_string(),
            metadata: serde_json::json!({"memory_importance": 0.2}),
            content_vector: None,
            embedding: Vec::new(),
        }];

        let result = fallback_reflection(&memories);
        assert!(result.summary.contains("Reflection"));
        assert!(!result.notes.is_empty());
    }
}
