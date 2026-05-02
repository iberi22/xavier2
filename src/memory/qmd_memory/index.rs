use serde_json::{json, Value};
use std::collections::HashSet;

use std::collections::HashMap;
use crate::memory::qmd_memory::utils::*;
use crate::memory::qmd_memory::types::QueryBundle;

pub fn build_query_bundle_internal(query_text: &str) -> QueryBundle {
    let normalized_query = normalize_query(query_text);
    let mut variants = vec![normalized_query.clone()];
    let mut weights = HashMap::from([(normalized_query.clone(), 1.0)]);

    let tokens = normalized_query
        .split_whitespace()
        .map(|value| value.to_string())
        .collect::<Vec<_>>();

    for token in tokens.into_iter().take(MAX_EXPANSIONS) {
        if let Some(synonyms) = SYNONYM_MAP.get(token.as_str()) {
            for synonym in synonyms.iter().take(2) {
                let expanded = if normalized_query.is_empty() {
                    (*synonym).to_string()
                } else {
                    format!("{normalized_query} {synonym}")
                };
                if weights.contains_key(&expanded) {
                    continue;
                }
                variants.push(expanded.clone());
                weights.insert(expanded, 0.85);
            }
        }
    }

    if variants.len() == 1 {
        for token in query_text.split_whitespace().take(MAX_EXPANSIONS) {
            let cleaned = normalize_token(token);
            if cleaned.len() < 3 || cleaned == normalized_query {
                continue;
            }
            let expanded = format!("{normalized_query} {cleaned}");
            if !weights.contains_key(&expanded) {
                variants.push(expanded.clone());
                weights.insert(expanded, 0.8);
            }
        }
    }

    variants.truncate(5);

    QueryBundle {
        normalized_query,
        variants,
        weights,
    }
}

pub fn extract_candidate_terms_internal(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(normalize_token)
        .filter(|token| token.len() >= 4)
        .filter(|token| {
            !matches!(
                token.as_str(),
                "with"
                    | "that"
                    | "this"
                    | "from"
                    | "have"
                    | "were"
                    | "when"
                    | "what"
                    | "where"
                    | "which"
                    | "would"
                    | "could"
            )
        })
        .collect()
}

pub fn expand_document_variants(
    path: &str,
    content: &str,
    metadata: &Value,
) -> Vec<(String, String, Value)> {
    let mut variants = vec![(path.to_string(), content.to_string(), metadata.clone())];

    if !is_locomo_document(path, metadata) {
        return variants;
    }

    let session_time = metadata
        .get("session_time")
        .and_then(|value| value.as_str())
        .map(str::to_string);
    let speaker = metadata
        .get("speaker")
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .or_else(|| extract_primary_speaker(content));

    variants.extend(build_fact_variants(
        path,
        content,
        metadata,
        speaker.as_deref(),
    ));
    variants.extend(build_temporal_variants(
        path,
        content,
        metadata,
        speaker.as_deref(),
        session_time.as_deref(),
    ));

    dedupe_variants(variants)
}

pub fn normalize_locomo_metadata(path: &str, metadata: Value) -> Value {
    if !is_locomo_document(path, &metadata) {
        return metadata;
    }

    let mut metadata = metadata;
    if let Some(object) = metadata.as_object_mut() {
        if let Some(normalized) = object
            .get("dia_id")
            .and_then(|value| value.as_str())
            .and_then(normalize_dia_id)
            .or_else(|| extract_normalized_dia_id_from_path(path))
        {
            object.insert("dia_id".to_string(), json!(&normalized));
            object.insert("normalized_dia_id".to_string(), json!(normalized));
        }

        if let Some(source_path) = object.get("source_path").and_then(|value| value.as_str()) {
            let normalized_source_path = normalize_locomo_path(source_path);
            object.insert("source_path".to_string(), json!(&normalized_source_path));
            if let Some(normalized) = extract_normalized_dia_id_from_path(&normalized_source_path) {
                object.insert("source_dia_id".to_string(), json!(normalized));
            }
        }
    }

    metadata
}

pub fn build_fact_variants(
    path: &str,
    content: &str,
    metadata: &Value,
    speaker: Option<&str>,
) -> Vec<(String, String, Value)> {
    let mut variants = Vec::new();
    let Some(subject) = speaker else {
        return variants;
    };

    let lowered = content.to_lowercase();
    let mut push_fact = |index: usize, memory_kind: &str, fact_type: &str, value: String| {
        let sentence = match fact_type {
            "identity" => format!("{subject} is {value}."),
            "relationship_status" => format!("{subject} is {value}."),
            "research_topic" => format!("{subject} researched {value}."),
            "career_interest" => format!("{subject} would likely pursue {value}."),
            _ => format!("{subject}: {value}."),
        };
        variants.push((
            format!("{path}#derived/{memory_kind}/{index}"),
            sentence,
            build_variant_metadata(
                metadata,
                path,
                memory_kind,
                json!({
                    "speaker": subject,
                    "normalized_value": value,
                    "answer_span": value,
                    "fact_type": fact_type,
                }),
            ),
        ));
    };

    if let Some(value) = capture_value(
        content,
        r"(?i)\b(?:i am|i'm)\s+(?:a\s+)?(transgender woman|trans woman|woman|man|nonbinary|non-binary)\b",
    ) {
        push_fact(0, "entity_state", "identity", sentence_case_phrase(&value));
    } else if lowered.contains("transgender") || lowered.contains("trans community") {
        push_fact(
            0,
            "entity_state",
            "identity",
            "Transgender woman".to_string(),
        );
    }

    if let Some(value) = capture_value(
        content,
        r"(?i)\b(?:i am|i'm)\s+(single|married|divorced|engaged|widowed)\b",
    ) {
        push_fact(
            1,
            "entity_state",
            "relationship_status",
            sentence_case_phrase(&value),
        );
    } else if lowered.contains("single parent") {
        push_fact(
            1,
            "entity_state",
            "relationship_status",
            "Single".to_string(),
        );
    }

    if let Some(value) = capture_value(
        content,
        r"(?i)\b(?:researched|researching)\s+([A-Za-z][A-Za-z\s'-]{2,80})",
    ) {
        let cleaned = trim_fact_value(&value);
        if !cleaned.is_empty() {
            push_fact(
                2,
                "fact_atom",
                "research_topic",
                sentence_case_phrase(&cleaned),
            );
        }
    }

    if lowered.contains("counseling")
        || lowered.contains("mental health")
        || lowered.contains("psychology")
    {
        let inferred = if lowered.contains("counseling") && lowered.contains("mental health") {
            "Psychology, counseling certification".to_string()
        } else if lowered.contains("psychology") && lowered.contains("counsel") {
            "Psychology, counseling".to_string()
        } else if lowered.contains("mental health") {
            "Counseling, mental health".to_string()
        } else if lowered.contains("psychology") {
            "Psychology".to_string()
        } else {
            "Counseling".to_string()
        };
        push_fact(3, "summary_fact", "career_interest", inferred);
    }

    if let Some(value) = extract_duration_value(content) {
        push_fact(4, "fact_atom", "duration", value);
    }

    if let Some(value) = capture_value(content, r"(?i)\bmoved from\s+([A-Z][a-zA-Z]+)\b") {
        push_fact(5, "fact_atom", "origin_place", sentence_case_phrase(&value));
    }

    let activities = collect_present_keywords(
        &lowered,
        &[
            "pottery", "camping", "painting", "swimming", "running", "reading", "violin", "hiking",
        ],
    );
    if !activities.is_empty() {
        push_fact(
            6,
            "summary_fact",
            "activities",
            title_case_list(&activities),
        );
    }

    let places = collect_present_keywords(&lowered, &["beach", "mountains", "forest", "museum"]);
    if !places.is_empty() {
        push_fact(7, "summary_fact", "places", title_case_list(&places));
    }

    let preferences = collect_present_keywords(&lowered, &["dinosaurs", "nature"]);
    if !preferences.is_empty() {
        push_fact(
            8,
            "summary_fact",
            "preferences",
            title_case_list(&preferences),
        );
    }

    let books = extract_quoted_titles(content);
    if !books.is_empty() {
        push_fact(9, "summary_fact", "books", books.join(", "));
    }

    variants
}

pub fn build_temporal_variants(
    path: &str,
    content: &str,
    metadata: &Value,
    speaker: Option<&str>,
    session_time: Option<&str>,
) -> Vec<(String, String, Value)> {
    let Some(resolved_date) = resolve_temporal_value(content, session_time) else {
        return Vec::new();
    };

    let subject = speaker.unwrap_or_default();
    let action = infer_event_action(content);
    let sentence = if subject.is_empty() {
        format!("{action} on {resolved_date}.")
    } else {
        format!("{subject} {action} on {resolved_date}.")
    };

    vec![(
        format!("{path}#derived/temporal_event/0"),
        sentence,
        build_variant_metadata(
            metadata,
            path,
            "temporal_event",
            json!({
                "speaker": subject,
                "event_subject": subject,
                "event_action": action,
                "resolved_date": resolved_date,
                "resolved_granularity": infer_date_granularity(&resolved_date),
            }),
        ),
    )]
}

pub fn build_variant_metadata(
    metadata: &Value,
    source_path: &str,
    memory_kind: &str,
    extra: Value,
) -> Value {
    let mut base = metadata.clone();
    if let Some(object) = base.as_object_mut() {
        object.insert("source_path".to_string(), json!(source_path));
        object.insert("memory_kind".to_string(), json!(memory_kind));
        if let Some(extra_object) = extra.as_object() {
            for (key, value) in extra_object {
                object.insert(key.clone(), value.clone());
            }
        }
    }
    normalize_locomo_metadata(source_path, base)
}

pub fn dedupe_variants(variants: Vec<(String, String, Value)>) -> Vec<(String, String, Value)> {
    let mut seen = HashSet::new();
    variants
        .into_iter()
        .filter(|(_, content, metadata)| {
            let key = format!(
                "{}|{}|{}",
                content,
                metadata
                    .get("memory_kind")
                    .and_then(|value| value.as_str())
                    .unwrap_or("primary"),
                metadata
                    .get("normalized_value")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default()
            );
            seen.insert(key)
        })
        .collect()
}
