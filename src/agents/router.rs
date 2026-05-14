use std::{
    fs,
    path::PathBuf,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::{
    agents::{system1::RetrievalResult, system2::ReasoningResult},
    memory::manager::MemoryPriority,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RouteCategory {
    Direct,
    Retrieved,
    Complex,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteDecision {
    pub category: RouteCategory,
    pub model_override: Option<String>,
    pub should_skip_retrieval: bool,
    pub should_skip_reasoning: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct RoutingPolicy {
    #[serde(default = "default_policy_version")]
    pub version: u32,
    #[serde(default)]
    pub models: RoutingPolicyModels,
    #[serde(default)]
    pub thresholds: RoutingPolicyThresholds,
    #[serde(default)]
    pub embeddings: Vec<EmbeddingCandidatePolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct RoutingPolicyModels {
    #[serde(default, deserialize_with = "deserialize_candidate_list")]
    pub fast: Vec<ModelCandidatePolicy>,
    #[serde(default, deserialize_with = "deserialize_candidate_list")]
    pub quality: Vec<ModelCandidatePolicy>,
}

fn deserialize_candidate_list<'de, D>(
    deserializer: D,
) -> Result<Vec<ModelCandidatePolicy>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum CandidateOrList {
        Single(ModelCandidatePolicy),
        List(Vec<ModelCandidatePolicy>),
    }

    Ok(match CandidateOrList::deserialize(deserializer)? {
        CandidateOrList::Single(single) => vec![single],
        CandidateOrList::List(list) => list,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelCandidatePolicy {
    pub name: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub benchmark_latency_ms: Option<u64>,
    #[serde(default)]
    pub quality_score: Option<f32>,
    #[serde(default)]
    pub health: Option<String>,
    #[serde(default)]
    pub cost_per_input_token: Option<f32>,
    #[serde(default)]
    pub cost_per_output_token: Option<f32>,
    #[serde(default)]
    pub cost_per_1k_input: f32,
    #[serde(default)]
    pub cost_per_1k_output: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct EmbeddingCandidatePolicy {
    pub label: String,
    pub model: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub selected: bool,
    #[serde(default)]
    pub recall_at_5: Option<f32>,
    #[serde(default)]
    pub recall_at_10: Option<f32>,
    #[serde(default)]
    pub ndcg_at_10: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoutingPolicyThresholds {
    #[serde(default = "default_strong_retrieval_confidence")]
    pub strong_retrieval_confidence: f32,
    #[serde(default = "default_weak_reasoning_confidence")]
    pub weak_reasoning_confidence: f32,
}

impl Default for RoutingPolicyThresholds {
    fn default() -> Self {
        Self {
            strong_retrieval_confidence: default_strong_retrieval_confidence(),
            weak_reasoning_confidence: default_weak_reasoning_confidence(),
        }
    }
}

#[derive(Debug, Clone)]
struct CachedPolicy {
    path: Option<PathBuf>,
    refresh: Duration,
    loaded_at: Instant,
    policy: Option<RoutingPolicy>,
}

fn default_policy_version() -> u32 {
    1
}

fn default_enabled() -> bool {
    true
}

fn default_strong_retrieval_confidence() -> f32 {
    0.72
}

fn default_weak_reasoning_confidence() -> f32 {
    0.68
}

#[derive(Debug, Default, Clone)]
pub struct Router;

impl Router {
    pub fn new() -> Self {
        Self
    }

    pub fn classify(&self, query: &str) -> RouteDecision {
        let trimmed = query.trim();
        let lowered = trimmed.to_lowercase();

        if is_direct_query(trimmed, &lowered) {
            return RouteDecision {
                category: RouteCategory::Direct,
                model_override: None,
                should_skip_retrieval: true,
                should_skip_reasoning: true,
            };
        }

        if is_complex_query(trimmed, &lowered) {
            return RouteDecision {
                category: RouteCategory::Complex,
                model_override: env_model("XAVIER_ROUTER_COMPLEX_MODEL"),
                should_skip_retrieval: false,
                should_skip_reasoning: false,
            };
        }

        RouteDecision {
            category: RouteCategory::Retrieved,
            model_override: env_model("XAVIER_ROUTER_RETRIEVED_MODEL"),
            should_skip_retrieval: false,
            should_skip_reasoning: false,
        }
    }

    pub fn resolve_model_override(
        &self,
        route_category: RouteCategory,
        retrieval_result: &RetrievalResult,
        reasoning_result: &ReasoningResult,
    ) -> Option<String> {
        let policy = load_routing_policy();
        let fast_model = env_model("XAVIER_ROUTER_FAST_MODEL").or_else(|| {
            policy
                .as_ref()
                .and_then(|policy| select_best_candidate(&policy.models.fast))
        });
        let quality_model = env_model("XAVIER_ROUTER_QUALITY_MODEL").or_else(|| {
            policy
                .as_ref()
                .and_then(|policy| select_best_candidate(&policy.models.quality))
        });

        match (fast_model, quality_model) {
            (None, None) => None,
            (Some(single), None) | (None, Some(single)) => Some(single),
            (Some(fast), Some(quality)) if fast == quality => Some(fast),
            (Some(fast), Some(quality)) => {
                if route_category == RouteCategory::Direct {
                    return Some(fast);
                }

                if route_category == RouteCategory::Complex {
                    return Some(quality);
                }

                let thresholds = policy
                    .as_ref()
                    .map(|policy| policy.thresholds.clone())
                    .unwrap_or_default();
                let retrieval_confidence = retrieval_confidence(retrieval_result);
                let top_priority = highest_memory_priority(retrieval_result);
                let high_priority_memory = matches!(
                    top_priority,
                    MemoryPriority::Critical | MemoryPriority::High
                );
                let weak_retrieval = retrieval_confidence < thresholds.strong_retrieval_confidence
                    || reasoning_result.confidence < thresholds.weak_reasoning_confidence;

                if high_priority_memory || weak_retrieval {
                    Some(quality)
                } else {
                    Some(fast)
                }
            }
        }
    }

    pub fn direct_response(&self, query: &str) -> Option<String> {
        let lowered = query.trim().to_lowercase();

        if [
            "hello",
            "hi",
            "hey",
            "hola",
            "good morning",
            "good afternoon",
        ]
        .iter()
        .any(|greeting| lowered == *greeting)
        {
            return Some("Hello. How can I help with Xavier?".to_string());
        }

        if lowered.contains("thank") || lowered == "thanks" || lowered == "gracias" {
            return Some("You're welcome.".to_string());
        }

        None
    }
}

fn routing_policy_cache() -> &'static Mutex<Option<CachedPolicy>> {
    static CACHE: OnceLock<Mutex<Option<CachedPolicy>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

pub fn load_routing_policy() -> Option<RoutingPolicy> {
    let path = std::env::var("XAVIER_ROUTER_POLICY_PATH")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);
    let refresh = Duration::from_secs(
        std::env::var("XAVIER_ROUTER_POLICY_REFRESH_SECS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(30),
    );

    let mut cache = routing_policy_cache()
        .lock()
        .expect("router policy cache lock poisoned");
    let should_reload = match cache.as_ref() {
        Some(existing) => {
            existing.path != path
                || existing.refresh != refresh
                || existing.loaded_at.elapsed() >= refresh
        }
        None => true,
    };

    if should_reload {
        let policy = path.as_ref().and_then(|path| match fs::read_to_string(path) {
            Ok(payload) => match serde_json::from_str::<RoutingPolicy>(&payload) {
                Ok(policy) => Some(policy),
                Err(error) => {
                    warn!(path = %path.display(), error = %error, "failed to parse router policy");
                    None
                }
            },
            Err(error) => {
                warn!(path = %path.display(), error = %error, "failed to read router policy");
                None
            }
        });

        *cache = Some(CachedPolicy {
            path,
            refresh,
            loaded_at: Instant::now(),
            policy: policy.clone(),
        });
        return policy;
    }

    cache.as_ref().and_then(|entry| entry.policy.clone())
}

fn select_best_candidate(candidates: &[ModelCandidatePolicy]) -> Option<String> {
    candidates
        .iter()
        .filter(|c| {
            let healthy = c
                .health
                .as_deref()
                .map(|value| {
                    !matches!(
                        value.trim().to_ascii_lowercase().as_str(),
                        "down" | "failed"
                    )
                })
                .unwrap_or(true);
            c.enabled && healthy && !c.name.trim().is_empty()
        })
        .max_by(|a, b| {
            let ratio_a = a.quality_score.unwrap_or(0.0)
                / (a.cost_per_1k_input + a.cost_per_1k_output + 0.000001);
            let ratio_b = b.quality_score.unwrap_or(0.0)
                / (b.cost_per_1k_input + b.cost_per_1k_output + 0.000001);

            ratio_a
                .partial_cmp(&ratio_b)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    a.quality_score
                        .unwrap_or(0.0)
                        .partial_cmp(&b.quality_score.unwrap_or(0.0))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        })
        .map(|c| c.name.trim().to_string())
}

fn retrieval_confidence(result: &RetrievalResult) -> f32 {
    if result.documents.is_empty() {
        return 0.0;
    }

    let top_relevance = result
        .documents
        .iter()
        .take(3)
        .map(|doc| doc.relevance_score)
        .sum::<f32>()
        / result.documents.iter().take(3).count() as f32;
    let evidence_bonus = result
        .documents
        .first()
        .map(document_has_specific_evidence)
        .unwrap_or(false) as u8 as f32;
    let volume_bonus = (result.total_results.min(3) as f32) / 3.0;

    (top_relevance * 0.6 + evidence_bonus * 0.25 + volume_bonus * 0.15).clamp(0.0, 1.0)
}

fn document_has_specific_evidence(document: &crate::agents::system1::RetrievedDocument) -> bool {
    document.metadata.get("resolved_date").is_some()
        || document.metadata.get("normalized_value").is_some()
        || document.metadata.get("answer_span").is_some()
        || document
            .metadata
            .get("evidence_kind")
            .and_then(|value| value.as_str())
            .is_some()
        || document
            .metadata
            .get("provenance")
            .and_then(|value| value.as_object())
            .is_some_and(|value| {
                value.contains_key("symbol")
                    || value.contains_key("file_path")
                    || value.contains_key("url")
                    || value.contains_key("message_id")
            })
}

fn highest_memory_priority(result: &RetrievalResult) -> MemoryPriority {
    result
        .documents
        .iter()
        .map(|doc| MemoryPriority::from_metadata(&doc.metadata))
        .min_by_key(|priority| *priority as u8)
        .unwrap_or(MemoryPriority::Medium)
}

fn env_model(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn is_direct_query(trimmed: &str, lowered: &str) -> bool {
    let token_count = lowered.split_whitespace().count();
    let direct_phrases = [
        "hello",
        "hi",
        "hey",
        "hola",
        "thanks",
        "thank you",
        "gracias",
        "good morning",
        "good afternoon",
        "good evening",
    ];

    token_count <= 4
        && direct_phrases
            .iter()
            .any(|phrase| lowered == *phrase || lowered.starts_with(&format!("{phrase} ")))
        && !trimmed.ends_with('?')
}

fn is_complex_query(trimmed: &str, lowered: &str) -> bool {
    let token_count = lowered.split_whitespace().count();
    let clause_markers = [",", ";", " and ", " but ", " while ", " versus ", " vs "];
    let reasoning_markers = [
        "why",
        "how",
        "compare",
        "difference",
        "summarize",
        "synthesize",
        "timeline",
        "before",
        "after",
        "impact",
        "tradeoff",
        "trade-off",
        "analyze",
        "reason",
        "multi-hop",
    ];

    token_count >= 14
        || clause_markers
            .iter()
            .filter(|marker| lowered.contains(**marker))
            .count()
            >= 2
        || reasoning_markers
            .iter()
            .any(|marker| lowered.contains(marker))
        || (trimmed.ends_with('?')
            && ["why ", "how "]
                .iter()
                .any(|prefix| lowered.starts_with(prefix)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::system1::SearchType;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn retrieved_doc(priority: &str) -> crate::agents::system1::RetrievedDocument {
        crate::agents::system1::RetrievedDocument {
            id: "doc-1".to_string(),
            path: "memory/doc-1".to_string(),
            content: "Relevant memory".to_string(),
            relevance_score: 1.0,
            token_count: 2,
            metadata: serde_json::json!({
                "memory_priority": priority,
                "evidence_kind": "fact_atom",
            }),
        }
    }

    #[test]
    fn classifies_greetings_as_direct() {
        let router = Router::new();
        let decision = router.classify("hello");
        assert_eq!(decision.category, RouteCategory::Direct);
        assert!(decision.should_skip_retrieval);
        assert!(decision.should_skip_reasoning);
    }

    #[test]
    fn classifies_simple_fact_queries_as_retrieved() {
        let router = Router::new();
        let decision = router.classify("What is Xavier?");
        assert_eq!(decision.category, RouteCategory::Retrieved);
        assert!(!decision.should_skip_retrieval);
    }

    #[test]
    fn classifies_synthesis_queries_as_complex() {
        let router = Router::new();
        let decision = router.classify(
            "Compare the impact of semantic caching before and after dynamic routing and explain the tradeoffs.",
        );
        assert_eq!(decision.category, RouteCategory::Complex);
    }

    #[test]
    fn selects_quality_model_for_high_priority_memories() {
        let _guard = env_lock().lock().expect("test assertion");
        std::env::set_var("XAVIER_ROUTER_FAST_MODEL", "fast-model");
        std::env::set_var("XAVIER_ROUTER_QUALITY_MODEL", "quality-model");
        std::env::remove_var("XAVIER_ROUTER_POLICY_PATH");

        let router = Router::new();
        let retrieval = RetrievalResult {
            query: "query".to_string(),
            documents: vec![retrieved_doc("critical")],
            search_type: SearchType::Hybrid,
            total_results: 1,
        };
        let reasoning = ReasoningResult {
            query: "query".to_string(),
            analysis: "analysis".to_string(),
            confidence: 0.9,
            supporting_evidence: vec![],
            beliefs_updated: vec![],
            reasoning_chain: vec![],
        };

        assert_eq!(
            router.resolve_model_override(RouteCategory::Retrieved, &retrieval, &reasoning),
            Some("quality-model".to_string())
        );
    }

    #[test]
    fn selects_fast_model_for_strong_medium_priority_retrieval() {
        let _guard = env_lock().lock().expect("test assertion");
        std::env::set_var("XAVIER_ROUTER_FAST_MODEL", "fast-model");
        std::env::set_var("XAVIER_ROUTER_QUALITY_MODEL", "quality-model");
        std::env::remove_var("XAVIER_ROUTER_POLICY_PATH");

        let router = Router::new();
        let retrieval = RetrievalResult {
            query: "query".to_string(),
            documents: vec![retrieved_doc("medium")],
            search_type: SearchType::Hybrid,
            total_results: 3,
        };
        let reasoning = ReasoningResult {
            query: "query".to_string(),
            analysis: "analysis".to_string(),
            confidence: 0.9,
            supporting_evidence: vec![],
            beliefs_updated: vec![],
            reasoning_chain: vec![],
        };

        assert_eq!(
            router.resolve_model_override(RouteCategory::Retrieved, &retrieval, &reasoning),
            Some("fast-model".to_string())
        );
    }

    #[test]
    fn selects_model_with_best_quality_cost_ratio() {
        let candidates = vec![
            ModelCandidatePolicy {
                name: "expensive-quality".to_string(),
                enabled: true,
                benchmark_latency_ms: None,
                quality_score: Some(0.9),
                health: None,
                cost_per_input_token: None,
                cost_per_output_token: None,
                cost_per_1k_input: 0.1,
                cost_per_1k_output: 0.2,
            },
            ModelCandidatePolicy {
                name: "cheap-efficient".to_string(),
                enabled: true,
                benchmark_latency_ms: None,
                quality_score: Some(0.7),
                health: None,
                cost_per_input_token: None,
                cost_per_output_token: None,
                cost_per_1k_input: 0.01,
                cost_per_1k_output: 0.01,
            },
        ];

        // expensive-quality ratio: 0.9 / 0.3 = 3.0
        // cheap-efficient ratio: 0.7 / 0.02 = 35.0
        assert_eq!(
            select_best_candidate(&candidates),
            Some("cheap-efficient".to_string())
        );
    }

    #[test]
    fn handles_multiple_candidates_in_policy() {
        let policy_json = r#"{
            "version": 1,
            "models": {
                "fast": [
                    { "name": "fast-1", "enabled": true, "quality_score": 0.5, "cost_per_1k_input": 0.1, "cost_per_1k_output": 0.1 },
                    { "name": "fast-2", "enabled": true, "quality_score": 0.8, "cost_per_1k_input": 0.01, "cost_per_1k_output": 0.01 }
                ],
                "quality": { "name": "quality-1", "enabled": true, "quality_score": 0.9 }
            }
        }"#;

        let policy: RoutingPolicy = serde_json::from_str(policy_json).unwrap();
        assert_eq!(policy.models.fast.len(), 2);
        assert_eq!(policy.models.quality.len(), 1);

        assert_eq!(
            select_best_candidate(&policy.models.fast),
            Some("fast-2".to_string())
        );
        assert_eq!(
            select_best_candidate(&policy.models.quality),
            Some("quality-1".to_string())
        );
    }
}
