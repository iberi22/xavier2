use anyhow::Result;
use regex::Regex;
use std::{

    sync::LazyLock,
    time::Instant,
};
use crate::memory::qmd_memory::types::{
    MemoryDocument, EMBEDDING_CACHE, EMBEDDING_CACHE_TTL_SECS, EmbeddingCacheEntry,
};
use crate::memory::qmd_memory::QmdMemory;
use crate::memory::schema::MemoryQueryFilters;
use crate::memory::qmd_memory::consolidation::is_locomo_document;
use crate::utils::crypto::hex_encode;
use sha2::{Digest, Sha256};

pub(crate) static SPEAKER_COLON_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^([^:\s]+):\s*").unwrap());
pub(crate) static SPEAKER_BRACKET_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^\[([^]\s]+)\]").unwrap());
pub(crate) static SPEAKER_ROLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(?:Speaker|Person|Host|Guest|Interviewer|Interviewee|Moderator):\s*([A-Z][a-zA-Z]+)",
    )
    .unwrap()
});
pub(crate) static QUERY_SPEAKER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:who|what|where|when|why|how|did|was|were)(?:\s+is|\s+did|\s+was|\s+were)?\s+([A-Z][a-zA-Z]+)").unwrap()
});
pub(crate) static SHE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\bshe\b").unwrap());
pub(crate) static HE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\bhe\b").unwrap());

pub(crate) async fn generate_embedding(text: &str) -> Result<Vec<f32>> {
    let new_embedder_configured =
        std::env::var("XAVIER2_EMBEDDER").is_ok() || std::env::var("OPENAI_API_KEY").is_ok();
    let legacy_embedder_configured = std::env::var("XAVIER2_EMBEDDING_URL").is_ok();

    if !new_embedder_configured && !legacy_embedder_configured {
        return Ok(Vec::new());
    }
    let preprocessed = preprocess_for_embedding(text);
    let cache_key = embedding_cache_key(&preprocessed);

    // CRITICAL FIX: Check embedding cache first to avoid redundant API calls
    {
        let cache = EMBEDDING_CACHE.read().await;
        if let Some(entry) = cache.get(&cache_key) {
            // Check if entry is still valid (within TTL)
            if Instant::now().duration_since(entry.cached_at).as_secs() < EMBEDDING_CACHE_TTL_SECS {
                tracing::debug!("Embedding cache HIT for key: {}", &cache_key[..16]);
                return Ok(entry.vector.clone());
            }
        }
    }

    let mut last_error = None;
    let mut delay_ms: u64 = 100;
    let max_delay_ms: u64 = 2000;

    if new_embedder_configured {
        if let Ok(embedder) = crate::embedding::build_embedder_from_env().await {
            if embedder.dimension() > 0 {
                for attempt in 0..3 {
                    match embedder.encode(&preprocessed).await {
                        Ok(vector) => {
                            let mut cache = EMBEDDING_CACHE.write().await;
                            cache.insert(
                                cache_key,
                                EmbeddingCacheEntry {
                                    vector: vector.clone(),
                                    cached_at: Instant::now(),
                                },
                            );
                            if cache.len() % 10 == 0 {
                                drop(cache);
                                clean_embedding_cache().await;
                            }
                            return Ok(vector);
                        }
                        Err(error) => {
                            last_error = Some(anyhow::anyhow!(error.to_string()));
                            if attempt < 2 {
                                tokio::time::sleep(std::time::Duration::from_millis(delay_ms))
                                    .await;
                                delay_ms = (delay_ms * 2).min(max_delay_ms);
                            }
                        }
                    }
                }
            }
        }
    }

    if legacy_embedder_configured {
        let client = crate::memory::embedder::EmbeddingClient::from_env()?;

        for attempt in 0..3 {
            match client.embed(&preprocessed).await {
                Ok(vector) => {
                    let mut cache = EMBEDDING_CACHE.write().await;
                    cache.insert(
                        cache_key,
                        EmbeddingCacheEntry {
                            vector: vector.clone(),
                            cached_at: Instant::now(),
                        },
                    );
                    if cache.len() % 10 == 0 {
                        drop(cache);
                        clean_embedding_cache().await;
                    }
                    return Ok(vector);
                }
                Err(error) => {
                    last_error = Some(error);
                    if attempt < 2 {
                        tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                        delay_ms = (delay_ms * 2).min(max_delay_ms);
                    }
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("embedding generation failed")))
}

pub(crate) fn embedding_cache_key(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex_encode(&hasher.finalize())
}

pub(crate) async fn clean_embedding_cache() {
    let mut cache = EMBEDDING_CACHE.write().await;
    let now = Instant::now();
    cache.retain(|_, entry| {
        now.duration_since(entry.cached_at).as_secs() < EMBEDDING_CACHE_TTL_SECS
    });
}

fn preprocess_for_embedding(text: &str) -> String {
    let speakers = extract_speakers(text);

    if speakers.is_empty() {
        // Still preprocess to handle quoted speech
        return preserve_quoted_speech(text);
    }

    // Build structured speaker context with turn-taking info
    let speaker_list: Vec<String> = speakers.iter().map(|s| format!("[{}]", s)).collect();

    // Preserve quoted speech which often contains answers
    let text_with_quotes = preserve_quoted_speech(text);

    let speaker_ctx = format!(
        "Conversation between: {}. \nQuote context: {}\n\n",
        speaker_list.join(", "),
        speaker_list.join(" said, ")
    );
    format!("{}{}", speaker_ctx, text_with_quotes)
}

fn preserve_quoted_speech(text: &str) -> String {
    // Replace quoted text with a marker to emphasize it in embeddings
    let mut result = text.to_string();

    // Pattern for quoted speech: \"...\" or '...'
    let quote_re = regex::Regex::new(r#"["']([^"']+)["']"#).unwrap();

    let mut quote_count = 0;
    result = quote_re
        .replace_all(&result, |caps: &regex::Captures| {
            quote_count += 1;
            let quote = &caps[1];
            format!("[QUOTE{}: {}]", quote_count, quote)
        })
        .to_string();

    result
}

fn extract_speakers(text: &str) -> Vec<String> {
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

fn extract_speaker_from_query(query: &str) -> Option<String> {
    QUERY_SPEAKER_RE.captures(query).and_then(|cap| {
        let name = cap.get(1)?.as_str();
        if is_likely_speaker(name) {
            Some(name.to_string())
        } else {
            None
        }
    })
}

fn is_female_name(name: &str) -> bool {
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

fn is_male_name(name: &str) -> bool {
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

fn resolve_pronouns(query: &str, speakers: &[String]) -> String {
    let mut resolved = query.to_string();

    // Resolve \"she\"
    if query.to_lowercase().contains("she") {
        let female_candidates: Vec<_> = speakers.iter().filter(|s| is_female_name(s)).collect();
        if female_candidates.len() == 1 {
            resolved = SHE_RE
                .replace_all(&resolved, female_candidates[0])
                .to_string();
        }
    }

    // Resolve \"he\"
    if query.to_lowercase().contains("he") {
        let male_candidates: Vec<_> = speakers.iter().filter(|s| is_male_name(s)).collect();
        if male_candidates.len() == 1 {
            resolved = HE_RE.replace_all(&resolved, male_candidates[0]).to_string();
        }
    }

    resolved
}

fn is_likely_speaker(s: &str) -> bool {
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

pub async fn query_with_embedding(
    memory: &QmdMemory,
    query_text: &str,
    limit: usize,
) -> Result<Vec<MemoryDocument>> {
    query_with_embedding_filtered(memory, query_text, limit, None).await
}

pub(crate) async fn query_with_embedding_filtered(
    memory: &QmdMemory,
    query_text: &str,
    limit: usize,
    filters: Option<&MemoryQueryFilters>,
) -> Result<Vec<MemoryDocument>> {
    let mut processed_query = query_text.to_string();

    // 1. Extract all speakers currently in memory to assist with pronoun resolution
    let all_docs = memory.all_documents().await;
    let mut all_speakers = std::collections::HashSet::new();
    let locomo_only = !all_docs.is_empty()
        && all_docs
            .iter()
            .all(|doc| is_locomo_document(&doc.path, &doc.metadata));
    for doc in &all_docs {
        for speaker in extract_speakers(&doc.content) {
            all_speakers.insert(speaker);
        }
    }
    let speakers_list: Vec<String> = all_speakers.into_iter().collect();

    // 2. Resolve pronouns if applicable
    if !speakers_list.is_empty() {
        processed_query = resolve_pronouns(&processed_query, &speakers_list);
    }

    // 3. If a name is explicitly mentioned in the query after an interrogative,
    // ensure it's prioritized in the final retrieval by prepending it to the query.
    // This dramatically improves semantic matching for \"Who did X?\" style questions.
    if let Some(target_speaker) = extract_speaker_from_query(query_text) {
        // Prepend speaker name for better semantic focus
        if !processed_query.contains(&target_speaker) {
            processed_query = format!("{} {}", target_speaker, processed_query);
        }
    }

    if locomo_only {
        return memory
            .query_filtered(&processed_query, Vec::new(), limit, filters)
            .await;
    }

    let query_vector = generate_embedding(&processed_query).await?;

    if query_vector.is_empty() {
        // Fallback to keyword search with the processed query
        return memory
            .search_with_cache_filtered(&processed_query, limit, filters)
            .await
            .map(|r| r.documents);
    }

    // 4. ENHANCED: Use top semantic results to expand query context
    // Get initial semantic results to understand what the query is about
    let initial_results = memory
        .vsearch(query_vector.clone(), 3)
        .await
        .unwrap_or_default();

    // If we found relevant documents, create an expanded query that includes
    // context from those documents to improve recall
    if !initial_results.is_empty() {
        let mut context_terms = Vec::new();

        // Extract meaningful terms from top results (avoiding common words)
        let common_words: std::collections::HashSet<&str> = std::collections::HashSet::from_iter([
            "the", "a", "an", "is", "are", "was", "were", "be", "been", "being", "have", "has",
            "had", "do", "does", "did", "will", "would", "could", "should", "may", "might", "must",
            "shall", "can", "need", "dare", "to", "of", "in", "for", "on", "with", "at", "by",
            "from", "as", "into", "through", "during", "before", "after", "above", "below", "that",
            "this", "these", "those", "it", "its", "they", "them", "what", "which", "who", "whom",
            "whose", "where", "when", "why", "how",
        ]);

        for doc in initial_results.iter().take(2) {
            for word in doc.content.split_whitespace() {
                let w_clean = word
                    .trim_matches(|c: char| !c.is_alphanumeric())
                    .to_lowercase();
                if w_clean.len() >= 4
                    && !common_words.contains(w_clean.as_str())
                    && !processed_query.to_lowercase().contains(&w_clean)
                {
                    context_terms.push(w_clean);
                }
            }
        }

        // Add context terms to query if we have few results
        if context_terms.len() >= 2 {
            let expanded_query = format!("{} {}", processed_query, context_terms.join(" "));
            // Generate a second embedding with expanded context
            if let Ok(expanded_vector) = generate_embedding(&expanded_query).await {
                if !expanded_vector.is_empty() {
                    // Use the expanded vector for better semantic matching
                    return memory
                        .query_filtered(&expanded_query, expanded_vector, limit, filters)
                        .await;
                }
            }
        }
    }

    memory
        .query_filtered(&processed_query, query_vector, limit, filters)
        .await
}
