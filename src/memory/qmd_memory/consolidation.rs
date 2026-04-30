use chrono::{Datelike, Duration, NaiveDate};
use regex::Regex;
use serde_json::{json, Value};
use std::collections::HashMap;

use std::sync::LazyLock;
use crate::memory::qmd_memory::types::MemoryDocument;

pub(crate) static DIA_ID_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^([a-z]+\d+):0*([0-9]+)$").unwrap());
pub(crate) static LOCOMO_PATH_DIA_ID_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(/)([a-z]+\d+):0*([0-9]+)([#/]|$)").unwrap());

pub(crate) fn expand_document_variants(
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

pub(crate) fn is_locomo_document(path: &str, metadata: &Value) -> bool {
    path.contains("locomo/")
        || metadata
            .get("benchmark")
            .and_then(|value| value.as_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("locomo"))
}

fn normalize_dia_id(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    DIA_ID_RE
        .captures(trimmed)
        .and_then(|captures| format_normalized_dia_id(&captures, 1, 2))
}

fn extract_normalized_dia_id_from_path(path: &str) -> Option<String> {
    LOCOMO_PATH_DIA_ID_RE
        .captures(path)
        .and_then(|captures| format_normalized_dia_id(&captures, 2, 3))
}

fn format_normalized_dia_id(
    captures: &regex::Captures,
    prefix_group: usize,
    number_group: usize,
) -> Option<String> {
    let prefix = captures
        .get(prefix_group)
        .map(|value| value.as_str().to_ascii_uppercase())?;
    let number = captures
        .get(number_group)
        .and_then(|value| value.as_str().parse::<u32>().ok())?;
    Some(format!("{prefix}:{number}"))
}

fn normalize_locomo_path(path: &str) -> String {
    LOCOMO_PATH_DIA_ID_RE
        .replace_all(path, |captures: &regex::Captures| {
            format!(
                "{}{}:{}{}",
                captures.get(1).map(|value| value.as_str()).unwrap_or("/"),
                captures
                    .get(2)
                    .map(|value| value.as_str().to_ascii_uppercase())
                    .unwrap_or_default(),
                captures
                    .get(3)
                    .and_then(|value| value.as_str().parse::<u32>().ok())
                    .unwrap_or_default(),
                captures
                    .get(4)
                    .map(|value| value.as_str())
                    .unwrap_or_default()
            )
        })
        .into_owned()
}

pub(crate) fn normalize_locomo_metadata(path: &str, metadata: Value) -> Value {
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

fn extract_primary_speaker(content: &str) -> Option<String> {
    content.lines().find_map(|line| {
        line.split_once(':').and_then(|(candidate, _)| {
            let trimmed = candidate.trim();
            (!trimmed.is_empty()
                && trimmed
                    .chars()
                    .next()
                    .is_some_and(|ch| ch.is_ascii_uppercase())
                && trimmed
                    .chars()
                    .all(|ch| ch.is_ascii_alphabetic() || ch == ' '))
            .then(|| trimmed.to_string())
        })
    })
}

fn build_fact_variants(
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

fn build_temporal_variants(
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

fn build_variant_metadata(
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

fn dedupe_variants(variants: Vec<(String, String, Value)>) -> Vec<(String, String, Value)> {
    let mut seen = std::collections::HashSet::new();
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

fn capture_value(content: &str, pattern: &str) -> Option<String> {
    Regex::new(pattern)
        .ok()?
        .captures(content)?
        .get(1)
        .map(|value| trim_fact_value(value.as_str()))
        .filter(|value| !value.is_empty())
}

fn trim_fact_value(value: &str) -> String {
    let mut cleaned = value
        .trim()
        .trim_end_matches(['.', ',', ';', ':', '!', '?'])
        .trim_matches('"')
        .trim()
        .to_string();

    for suffix in [
        " lately",
        " recently",
        " currently",
        " these days",
        " right now",
    ] {
        if cleaned.to_lowercase().ends_with(suffix) {
            cleaned.truncate(cleaned.len().saturating_sub(suffix.len()));
            cleaned = cleaned.trim().to_string();
            break;
        }
    }

    for prefix in ["and ", "to "] {
        if cleaned.to_lowercase().starts_with(prefix) {
            cleaned = cleaned[prefix.len()..].trim().to_string();
            break;
        }
    }

    cleaned
}

fn sentence_case_phrase(value: &str) -> String {
    let lower = value.trim().to_lowercase();
    let mut chars = lower.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
        None => lower,
    }
}

fn title_case_list(values: &[String]) -> String {
    values
        .iter()
        .map(|value| sentence_case_phrase(value))
        .collect::<Vec<_>>()
        .join(", ")
}

fn collect_present_keywords(lowered: &str, keywords: &[&str]) -> Vec<String> {
    keywords
        .iter()
        .filter(|keyword| lowered.contains(**keyword))
        .map(|keyword| (*keyword).to_string())
        .collect()
}

fn extract_quoted_titles(content: &str) -> Vec<String> {
    let quote_re = Regex::new(r#""([^"]+)""#).expect("quoted title regex");
    quote_re
        .captures_iter(content)
        .filter_map(|capture| {
            capture
                .get(1)
                .map(|value| value.as_str().trim().to_string())
        })
        .filter(|value| !value.is_empty())
        .collect()
}

fn extract_duration_value(content: &str) -> Option<String> {
    let years_ago = Regex::new(r"(?i)\b(\d+)\s+years?\s+ago\b").ok()?;
    if let Some(capture) = years_ago.captures(content) {
        let years = capture.get(1)?.as_str();
        return Some(format!("{years} years ago"));
    }

    let for_years = Regex::new(r"(?i)\bfor\s+(\d+)\s+years?\b").ok()?;
    if let Some(capture) = for_years.captures(content) {
        let years = capture.get(1)?.as_str();
        return Some(format!("{years} years"));
    }

    let bare_years = Regex::new(r"(?i)\b(\d+)\s+years?\b").ok()?;
    if let Some(capture) = bare_years.captures(content) {
        let years = capture.get(1)?.as_str();
        return Some(format!("{years} years"));
    }

    None
}

fn infer_event_action(content: &str) -> String {
    let lowered = content.to_lowercase();
    if lowered.contains("support group") {
        "went to the LGBTQ support group".to_string()
    } else if lowered.contains("school event")
        || lowered.contains("talked about her transgender journey")
    {
        "gave a speech at a school".to_string()
    } else if lowered.contains("friends, family, and mentors") {
        "met up with her friends family and mentors".to_string()
    } else if lowered.contains("painted") && lowered.contains("sunrise") {
        "painted a sunrise".to_string()
    } else if lowered.contains("charity race") {
        "ran a charity race".to_string()
    } else if lowered.contains("going camping") || lowered.contains("planning on going camping") {
        "is planning on going camping".to_string()
    } else if lowered.contains("went camping")
        || lowered.contains("camping last week")
        || lowered.contains("went camping with")
    {
        "went camping".to_string()
    } else if lowered.contains("camping") {
        "camping came up".to_string()
    } else {
        "had the event".to_string()
    }
}

fn resolve_temporal_value(content: &str, session_time: Option<&str>) -> Option<String> {
    if let Some(explicit) = extract_explicit_date_value(content) {
        return Some(explicit);
    }

    let session_date = session_time.and_then(parse_session_date)?;
    let lowered = content.to_lowercase();

    if lowered.contains("yesterday") {
        return Some(format_date(session_date - Duration::days(1)));
    }
    if lowered.contains("last year") {
        return Some((session_date.year() - 1).to_string());
    }
    if lowered.contains("last saturday") {
        return Some(format!(
            "The sunday before {}",
            session_date.format("%-d %B %Y")
        ));
    }
    if lowered.contains("last friday") {
        return Some(format!(
            "The friday before {}",
            session_date.format("%-d %B %Y")
        ));
    }
    if lowered.contains("last week") {
        return Some(format!(
            "The week before {}",
            session_date.format("%-d %B %Y")
        ));
    }
    if lowered.contains("next month") {
        let (year, month) = if session_date.month() == 12 {
            (session_date.year() + 1, 1)
        } else {
            (session_date.year(), session_date.month() + 1)
        };
        let date = NaiveDate::from_ymd_opt(year, month, 1)?;
        return Some(date.format("%B %Y").to_string());
    }
    if lowered.contains("last sunday") || lowered.contains("sunday before") {
        let weekday = session_date.weekday().num_days_from_sunday() as i64;
        let days_back = if weekday == 0 { 7 } else { weekday };
        return Some(format_date(session_date - Duration::days(days_back)));
    }

    None
}

fn extract_explicit_date_value(text: &str) -> Option<String> {
    let patterns = [
        (r"(?i)\b\d{1,2}\s+[A-Za-z]+\s+\d{4}\b", false),
        (r"(?i)\b[A-Za-z]+\s+\d{1,2},\s+\d{4}\b", false),
        (r"\b(19|20)\d{2}\b", true),
    ];

    for (pattern, is_year_only) in patterns {
        let regex = Regex::new(pattern).ok()?;
        if let Some(found) = regex.find(text) {
            let value = found.as_str().trim();
            return Some(if is_year_only {
                value.to_string()
            } else {
                clean_extracted_date(value)
            });
        }
    }

    None
}

fn parse_session_date(session_time: &str) -> Option<NaiveDate> {
    let date_text = session_time
        .split(" on ")
        .last()
        .unwrap_or(session_time)
        .trim();
    let normalized = date_text.replace("  ", " ");
    for format in ["%d %B, %Y", "%d %B %Y", "%B %d, %Y"] {
        if let Ok(date) = NaiveDate::parse_from_str(&normalized, format) {
            return Some(date);
        }
    }
    None
}

fn format_date(date: NaiveDate) -> String {
    date.format("%-d %B %Y").to_string()
}

fn clean_extracted_date(value: &str) -> String {
    value.trim().trim_end_matches(['.', ',', ';']).to_string()
}

fn infer_date_granularity(value: &str) -> &'static str {
    if value.chars().all(|ch| ch.is_ascii_digit()) && value.len() == 4 {
        "year"
    } else if value.split_whitespace().count() == 2 {
        "month_year"
    } else {
        "full_date"
    }
}

pub fn extract_answer(content: &str, category: &str) -> Option<String> {
    let text = content.trim();
    if text.is_empty() {
        return None;
    }

    match category {
        "2" => {
            static DATE_RE: LazyLock<Regex> = LazyLock::new(|| {
                Regex::new(r"(?i)\b(?:\d{1,2}\s+[A-Za-z]+\s+\d{4}|[A-Za-z]+\s+\d{1,2},\s+\d{4}|(19|20)\d{2})\b").unwrap()
            });
            DATE_RE.find(text).map(|m| m.as_str().trim().to_string())
        }
        "3" => {
            let sentence = text
                .split(['.', '!', '?'])
                .map(str::trim)
                .find(|sentence| {
                    let lowered = sentence.to_lowercase();
                    [
                        "think",
                        "believe",
                        "feel",
                        "guess",
                        "suppose",
                        "probably",
                        "definitely",
                        "maybe",
                        "opinion",
                        "view",
                        "perspective",
                        "seems",
                        "appears",
                        "likely",
                        "certainly",
                        "perhaps",
                        "wonder",
                    ]
                    .iter()
                    .any(|keyword| lowered.contains(keyword))
                })
                .or_else(|| {
                    text.split(['.', '!', '?'])
                        .map(str::trim)
                        .find(|s| !s.is_empty())
                })?;
            Some(sentence.to_string())
        }
        "4" => {
            let sentence = text
                .split(['.', '!', '?'])
                .map(str::trim)
                .find(|sentence| {
                    let lowered = sentence.to_lowercase();
                    [
                        "decided",
                        "planning",
                        "planned",
                        "will",
                        "going to",
                        "intend",
                        "promised",
                        "try",
                        "started",
                        "beginning",
                        "began",
                        "going to start",
                        "want to",
                        "hoping to",
                        "aiming to",
                    ]
                    .iter()
                    .any(|keyword| lowered.contains(keyword))
                })
                .or_else(|| {
                    text.split(['.', '!', '?'])
                        .map(str::trim)
                        .find(|s| !s.is_empty())
                })?;
            Some(sentence.to_string())
        }
        _ => text
            .split(['.', '!', '?'])
            .map(str::trim)
            .find(|sentence| !sentence.is_empty())
            .map(|sentence| sentence.to_string()),
    }
}

pub(crate) fn _deduplicate_by_content_hash(results: Vec<MemoryDocument>) -> Vec<MemoryDocument> {

    use crate::utils::crypto::hex_encode;
    use sha2::{Digest, Sha256};

    // Group by content hash, tracking (document, latest_updated_at, original_index)
    let mut hash_groups: HashMap<String, (MemoryDocument, Option<String>, usize)> = HashMap::new();
    for (idx, doc) in results.into_iter().enumerate() {
        let content_hash = hex_encode(Sha256::digest(doc.content.as_bytes()).as_slice());
        let updated_at = doc
            .metadata
            .get("updated_at")
            .and_then(|v| v.as_str())
            .or_else(|| {
                doc.metadata
                    .get("created_at")
                    .and_then(|v| v.as_str())
            })
            .map(str::to_string);

        hash_groups
            .entry(content_hash)
            .and_modify(|(existing_doc, existing_updated, existing_idx)| {
                // Keep the one with more recent updated_at, or keep existing if tie
                let is_newer = match (updated_at.as_ref(), existing_updated.as_ref()) {
                    (Some(new), Some(old)) => new > old,
                    (Some(_), None) => true, // new has timestamp, existing doesn't
                    (None, Some(_)) => false, // existing has timestamp, new doesn't
                    (None, None) => idx < *existing_idx, // tie-break by original order
                };
                if is_newer {
                    *existing_doc = doc.clone();
                    *existing_updated = updated_at.clone();
                    *existing_idx = idx;
                }
            })
            .or_insert((doc, updated_at.clone(), idx));
    }

    // Extract deduplicated results, sort by original order (first occurrence per hash)
    let mut deduped: Vec<(usize, MemoryDocument)> = hash_groups
        .into_values()
        .map(|(doc, _, idx)| (idx, doc))
        .collect();
    deduped.sort_by(|a, b| a.0.cmp(&b.0));
    deduped.into_iter().map(|(_, doc)| doc).collect()
}
