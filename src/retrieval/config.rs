//! Shared retrieval tuning defaults.

pub const DEFAULT_WORKING_WEIGHT: f32 = 0.3;
pub const DEFAULT_EPISODIC_WEIGHT: f32 = 0.3;
pub const DEFAULT_SEMANTIC_WEIGHT: f32 = 0.4;
pub const DEFAULT_RELEVANCE_THRESHOLD: f32 = 0.5;
pub const DEFAULT_RRF_K: u32 = 60;

pub fn configured_rrf_k() -> u32 {
    std::env::var("XAVIER2_RRF_K")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(DEFAULT_RRF_K)
}
pub const DEFAULT_MAX_RESULTS: usize = 20;
pub const DEFAULT_SEARCH_LIMIT: usize = 10;
pub const DEFAULT_KEYWORD_WEIGHT: f32 = 0.5;
pub const DEFAULT_VECTOR_WEIGHT: f32 = 0.5;

pub const WEIGHT_SUM_TOLERANCE: f32 = 0.001;
pub const MIN_RELEVANCE_THRESHOLD: f32 = 0.0;
pub const MAX_RELEVANCE_THRESHOLD: f32 = 1.0;

pub const EXACT_PHRASE_MATCH_BONUS: f32 = 0.5;
pub const TERM_MATCH_BONUS: f32 = 0.1;
pub const TERM_OCCURRENCE_BONUS: f32 = 0.05;
pub const MAX_TERM_OCCURRENCE_BONUS: f32 = 0.3;
pub const EVENT_PHRASE_MATCH_BONUS: f32 = 0.3;
pub const EVENT_TERM_MATCH_BONUS: f32 = 0.05;

pub const EXACT_ENTITY_MATCH_SCORE: f32 = 1.0;
pub const PARTIAL_ENTITY_MATCH_SCORE: f32 = 0.7;
pub const ENTITY_DESCRIPTION_MATCH_SCORE: f32 = 0.4;
pub const ENTITY_DESCRIPTION_TERM_BONUS: f32 = 0.1;
pub const ENTITY_ALIAS_MATCH_SCORE: f32 = 0.6;
pub const SEMANTIC_CONFIDENCE_MULTIPLIER: f32 = 0.5;
