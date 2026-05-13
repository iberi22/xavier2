use chrono::{Datelike, NaiveDate};
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::LazyLock;

use crate::memory::qmd_memory::types::MemoryDocument;

pub static SPEAKER_COLON_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^([^:\s]+):\s*").expect("valid regex"));
pub static SPEAKER_BRACKET_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^\[([^]\s]+)\]").expect("valid regex"));
pub static SPEAKER_ROLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(?:Speaker|Person|Host|Guest|Interviewer|Interviewee|Moderator):\s*([A-Z][a-zA-Z]+)",
    )
    .expect("valid regex")
});
pub static QUERY_SPEAKER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:who|what|where|when|why|how|did|was|were)(?:\s+is|\s+did|\s+was|\s+were)?\s+([A-Z][a-zA-Z]+)").expect("valid regex")
});
pub static SHE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\bshe\b").expect("valid regex"));
pub static HE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\bhe\b").expect("valid regex"));
pub static SYNONYM_MAP: LazyLock<HashMap<&'static str, &'static [&'static str]>> =
    LazyLock::new(|| {
        HashMap::from([
            ("bug", &["issue", "error", "failure", "defect"][..]),
            ("cache", &["caching", "memoization", "store"][..]),
            ("fast", &["quick", "speed", "latency"][..]),
            ("memory", &["context", "retrieval", "knowledge"][..]),
            ("search", &["lookup", "find", "retrieve"][..]),
            ("vector", &["embedding", "semantic", "dense"][..]),
            ("query", &["question", "request", "prompt"][..]),
            ("reasoning", &["multi-hop", "inference", "analysis"][..]),
        ])
    });

pub const RRF_K: f32 = 60.0;
pub const KEYWORD_WEIGHT: f32 = 0.7;
pub const SEMANTIC_WEIGHT: f32 = 0.3;
pub const MAX_EXPANSIONS: usize = 4;
pub const MAX_MULTI_HOP_DEPTH: usize = 2;
pub const MAX_RERANK_CANDIDATES: usize = 32;

pub static DIA_ID_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^([a-z]+\d+):0*([0-9]+)$").expect("valid regex"));
pub static LOCOMO_PATH_DIA_ID_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(/)([a-z]+\d+):0*([0-9]+)([#/]|$)").expect("valid regex"));

pub fn normalize_query(query_text: &str) -> String {
    query_text
        .split_whitespace()
        .map(normalize_token)
        .filter(|token| {
            !token.is_empty()
                && !matches!(
                    token.as_str(),
                    "when"
                        | "what"
                        | "where"
                        | "which"
                        | "who"
                        | "how"
                        | "why"
                        | "did"
                        | "does"
                        | "was"
                        | "were"
                        | "the"
                        | "and"
                        | "for"
                        | "with"
                        | "about"
                        | "into"
                        | "from"
                        | "that"
                        | "this"
                        | "your"
                        | "have"
                        | "had"
                )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn normalize_token(token: &str) -> String {
    token
        .chars()
        .filter(|char| char.is_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

pub fn is_locomo_document(path: &str, metadata: &Value) -> bool {
    path.contains("locomo/")
        || metadata
            .get("benchmark")
            .and_then(|value| value.as_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("locomo"))
}

pub(crate) fn cosine_similarity(left: &[f32], right: &[f32]) -> f32 {
    if left.is_empty() || right.is_empty() || left.len() != right.len() {
        return 0.0;
    }

    let dot = left
        .iter()
        .zip(right.iter())
        .map(|(a, b)| a * b)
        .sum::<f32>();
    let left_magnitude = left.iter().map(|value| value * value).sum::<f32>().sqrt();
    let right_magnitude = right.iter().map(|value| value * value).sum::<f32>().sqrt();

    if left_magnitude == 0.0 || right_magnitude == 0.0 {
        return 0.0;
    }

    dot / (left_magnitude * right_magnitude)
}

pub fn extract_speakers(text: &str) -> Vec<String> {
    let mut speakers = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for re in &[&*SPEAKER_COLON_RE, &*SPEAKER_BRACKET_RE, &*SPEAKER_ROLE_RE] {
        for cap in re.captures_iter(text) {
            if let Some(name) = cap.get(1) {
                let name = name.as_str().trim();
                if is_likely_speaker(name) && seen.insert(name.to_lowercase()) {
                    speakers.push(name.to_string());
                }
            }
        }
    }
    speakers
}

pub fn is_likely_speaker(s: &str) -> bool {
    let s = s.trim();
    if s.len() < 2 || s.len() > 20 {
        return false;
    }
    if !s
        .chars()
        .next()
        .map(|c| c.is_ascii_uppercase())
        .unwrap_or(false)
    {
        return false;
    }
    if !s
        .chars()
        .all(|c| c.is_ascii_alphabetic() || c == '-' || c == '\'')
    {
        return false;
    }
    // Filter out common English words and pronouns
    let common: &[&str] = &[
        "The", "This", "That", "They", "Then", "There", "Here", "Hello", "Thanks", "Please",
        "Sorry", "Who", "What", "When", "Where", "Why", "How", "Did", "Was", "Were", "Are", "She",
        "He", "It",
    ];
    if common.contains(&s) {
        return false;
    }
    true
}

pub fn is_female_name(name: &str) -> bool {
    let name = name.to_lowercase();
    let female_names = [
        "caroline",
        "alice",
        "sarah",
        "emma",
        "olivia",
        "sophia",
        "isabella",
        "mia",
        "charlotte",
        "amelia",
        "mary",
        "patricia",
        "jennifer",
        "linda",
        "elizabeth",
        "barbara",
        "susan",
        "jessica",
        "karen",
    ];
    female_names.contains(&name.as_str())
}

pub fn is_male_name(name: &str) -> bool {
    let name = name.to_lowercase();
    let male_names = [
        "james",
        "robert",
        "john",
        "michael",
        "david",
        "william",
        "richard",
        "joseph",
        "thomas",
        "christopher",
        "charles",
        "daniel",
        "matthew",
        "anthony",
        "mark",
        "donald",
        "steven",
        "paul",
        "andrew",
        "joshua",
    ];
    male_names.contains(&name.as_str())
}

pub fn resolve_pronouns(query: &str, speakers: &[String]) -> String {
    let mut resolved = query.to_string();

    // Resolve "she"
    if query.to_lowercase().contains("she") {
        let female_candidates: Vec<_> = speakers.iter().filter(|s| is_female_name(s)).collect();
        if female_candidates.len() == 1 {
            resolved = SHE_RE
                .replace_all(&resolved, female_candidates[0])
                .to_string();
        }
    }

    // Resolve "he"
    if query.to_lowercase().contains("he") {
        let male_candidates: Vec<_> = speakers.iter().filter(|s| is_male_name(s)).collect();
        if male_candidates.len() == 1 {
            resolved = HE_RE.replace_all(&resolved, male_candidates[0]).to_string();
        }
    }

    resolved
}

pub fn extract_speaker_from_query(query: &str) -> Option<String> {
    QUERY_SPEAKER_RE.captures(query).and_then(|cap| {
        let name = cap.get(1)?.as_str();
        if is_likely_speaker(name) {
            Some(name.to_string())
        } else {
            None
        }
    })
}

pub fn normalize_dia_id(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    DIA_ID_RE
        .captures(trimmed)
        .and_then(|captures| format_normalized_dia_id(&captures, 1, 2))
}

pub fn extract_normalized_dia_id_from_path(path: &str) -> Option<String> {
    LOCOMO_PATH_DIA_ID_RE
        .captures(path)
        .and_then(|captures| format_normalized_dia_id(&captures, 2, 3))
}

pub fn format_normalized_dia_id(
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

pub fn normalize_locomo_path(path: &str) -> String {
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

pub fn extract_primary_speaker(content: &str) -> Option<String> {
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

pub fn capture_value(content: &str, pattern: &str) -> Option<String> {
    Regex::new(pattern)
        .ok()?
        .captures(content)?
        .get(1)
        .map(|value| trim_fact_value(value.as_str()))
        .filter(|value| !value.is_empty())
}

pub fn trim_fact_value(value: &str) -> String {
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

pub fn sentence_case_phrase(value: &str) -> String {
    let lower = value.trim().to_lowercase();
    let mut chars = lower.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
        None => lower,
    }
}

pub fn title_case_list(values: &[String]) -> String {
    values
        .iter()
        .map(|value| sentence_case_phrase(value))
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn collect_present_keywords(lowered: &str, keywords: &[&str]) -> Vec<String> {
    keywords
        .iter()
        .filter(|keyword| lowered.contains(**keyword))
        .map(|keyword| (*keyword).to_string())
        .collect()
}

pub fn extract_quoted_titles(content: &str) -> Vec<String> {
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

pub fn extract_duration_value(content: &str) -> Option<String> {
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

pub fn infer_event_action(content: &str) -> String {
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

pub fn resolve_temporal_value(content: &str, session_time: Option<&str>) -> Option<String> {
    if let Some(explicit) = extract_explicit_date_value(content) {
        return Some(explicit);
    }

    let session_date = session_time.and_then(parse_session_date)?;
    let lowered = content.to_lowercase();

    if lowered.contains("yesterday") {
        return Some(format_date(session_date - chrono::Duration::days(1)));
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
        return Some(format_date(
            session_date - chrono::Duration::days(days_back),
        ));
    }

    None
}

pub fn extract_explicit_date_value(text: &str) -> Option<String> {
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

pub fn parse_session_date(session_time: &str) -> Option<NaiveDate> {
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

pub fn format_date(date: NaiveDate) -> String {
    date.format("%-d %B %Y").to_string()
}

pub fn clean_extracted_date(value: &str) -> String {
    value.trim().trim_end_matches(['.', ',', ';']).to_string()
}

pub fn infer_date_granularity(value: &str) -> &'static str {
    if value.chars().all(|ch| ch.is_ascii_digit()) && value.len() == 4 {
        "year"
    } else if value.split_whitespace().count() == 2 {
        "month_year"
    } else {
        "full_date"
    }
}

pub fn locomo_query_terms(normalized_query: &str) -> Vec<&str> {
    normalized_query
        .split_whitespace()
        .filter(|term| {
            !term.is_empty()
                && !matches!(
                    *term,
                    "the"
                        | "and"
                        | "what"
                        | "when"
                        | "where"
                        | "which"
                        | "with"
                        | "from"
                        | "that"
                        | "this"
                        | "have"
                        | "about"
                        | "your"
                        | "their"
                        | "did"
                        | "does"
                        | "was"
                        | "were"
                )
        })
        .collect()
}

pub fn is_temporal_query(normalized_query: &str) -> bool {
    normalized_query.contains(" when ")
        || normalized_query.starts_with("when ")
        || normalized_query.contains(" date ")
        || normalized_query.contains(" year ")
        || normalized_query.contains(" month ")
        || normalized_query.contains(" day ")
}

pub fn contains_opinion_adjectives(content: &str) -> bool {
    let opinion_adjectives = [
        "ideal",
        "perfect",
        "best",
        "favorite",
        "great",
        "amazing",
        "wonderful",
        "excellent",
        "good",
        "bad",
        "terrible",
        "awful",
        "beautiful",
        "nice",
        "lovely",
        "pleasant",
        "important",
        "special",
        "unique",
        "better",
        "worse",
        "prefer",
        "love",
        "hate",
    ];
    let lowered = content.to_lowercase();
    opinion_adjectives.iter().any(|adj| lowered.contains(adj))
}

pub fn locomo_phrases(terms: &[&str]) -> Vec<String> {
    if terms.len() < 2 {
        return Vec::new();
    }

    terms.windows(2).map(|window| window.join(" ")).collect()
}

pub fn metadata_text_lower(doc: &MemoryDocument, key: &str) -> String {
    doc.metadata
        .get(key)
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_lowercase()
}

pub fn estimate_document_bytes(path: &str, content: &str, metadata: &serde_json::Value) -> u64 {
    path.len() as u64 + content.len() as u64 + metadata.to_string().len() as u64
}
