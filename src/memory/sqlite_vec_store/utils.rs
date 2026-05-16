use std::collections::HashSet;
use std::sync::OnceLock;
use regex::Regex;
use crate::memory::store::SessionTokenRecord;

pub struct SessionTokenRow {
    pub token: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

impl From<SessionTokenRow> for SessionTokenRecord {
    fn from(value: SessionTokenRow) -> Self {
        Self {
            token: value.token,
            created_at: value.created_at,
            expires_at: value.expires_at,
        }
    }
}

pub fn search_tokens(query: &str) -> Vec<String> {
    static TOKEN_RE: OnceLock<Regex> = OnceLock::new();
    let re = TOKEN_RE.get_or_init(|| {
        Regex::new(r"[A-Za-z0-9][A-Za-z0-9._:/#-]{1,}").expect("valid search token regex")
    });

    let mut seen = HashSet::new();
    re.find_iter(query)
        .filter_map(|m| {
            let token = m.as_str().trim_matches('"').trim().to_string();
            if token.len() < 2 {
                return None;
            }
            let lowered = token.to_ascii_lowercase();
            if seen.insert(lowered) {
                Some(token)
            } else {
                None
            }
        })
        .collect()
}

pub fn split_camel_case(token: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut previous_lower = false;
    for ch in token.chars() {
        if !ch.is_ascii_alphanumeric() {
            if !current.is_empty() {
                words.push(current.clone());
                current.clear();
            }
            previous_lower = false;
            continue;
        }

        let is_upper = ch.is_ascii_uppercase();
        if is_upper && previous_lower && !current.is_empty() {
            words.push(current.clone());
            current.clear();
        }
        previous_lower = ch.is_ascii_lowercase();
        current.push(ch.to_ascii_lowercase());
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

pub fn code_tokens(text: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut expanded = Vec::new();
    for token in search_tokens(text) {
        for segment in token
            .split(|ch: char| ['_', '-', '/', '.', ':'].contains(&ch))
            .filter(|segment| !segment.is_empty())
        {
            for part in split_camel_case(segment) {
                if part.len() > 1 && seen.insert(part.clone()) {
                    expanded.push(part);
                }
            }
        }
    }
    expanded
}
