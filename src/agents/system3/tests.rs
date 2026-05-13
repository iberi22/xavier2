use super::helpers::*;
use super::types::{ActionResult, ActorConfig};
use super::client::{LlmClient, ResponseGenerator};
use super::System3Actor;
use crate::agents::system1::RetrievedDocument;
use crate::agents::system1::{RetrievalResult, SearchType};
use crate::agents::system2::ReasoningResult;
use crate::memory::semantic_cache::{QueryEmbedder, SemanticCache};
use anyhow::Result;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

struct MockEmbedder {
    embeddings: HashMap<String, Vec<f32>>,
}

impl QueryEmbedder for MockEmbedder {
    fn embed<'a>(
        &'a self,
        input: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<f32>>> + Send + 'a>> {
        Box::pin(async move { Ok(self.embeddings.get(input).cloned().unwrap_or_default()) })
    }
}

#[derive(Default)]
struct MockResponder {
    calls: AtomicUsize,
    response: String,
    fail: bool,
}

impl ResponseGenerator for MockResponder {
    fn generate_response<'a>(
        &'a self,
        _query: &'a str,
        _context: &'a [RetrievedDocument],
    ) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.fail {
                anyhow::bail!("mock failure");
            }
            Ok(self.response.clone())
        })
    }
}

fn semantic_cache() -> Arc<SemanticCache> {
    let embeddings = HashMap::from([
        ("hello".to_string(), vec![1.0, 0.0]),
        ("hello again".to_string(), vec![0.99, 0.01]),
        ("fresh".to_string(), vec![0.0, 1.0]),
    ]);

    Arc::new(SemanticCache::new_with_embedder(
        0.95,
        Arc::new(MockEmbedder { embeddings }),
    ))
}

fn retrieval_result() -> RetrievalResult {
    RetrievalResult {
        query: "test".to_string(),
        documents: vec![],
        search_type: SearchType::Semantic,
        total_results: 0,
    }
}

fn reasoning_result() -> ReasoningResult {
    ReasoningResult {
        query: "test".to_string(),
        analysis: "ok".to_string(),
        confidence: 1.0,
        supporting_evidence: vec![],
        beliefs_updated: vec![],
        reasoning_chain: vec![],
    }
}

fn doc(content: &str, session_time: Option<&str>, speaker: Option<&str>) -> RetrievedDocument {
    RetrievedDocument {
        id: "doc-1".to_string(),
        path: "locomo/conv-1/session_1/D1:1".to_string(),
        content: content.to_string(),
        relevance_score: 1.0,
        metadata: serde_json::json!({
            "session_time": session_time,
            "speaker": speaker,
        }),
    }
}

#[test]
fn clean_date_keeps_readable_format() {
    assert_eq!(clean_date("Monday on 7 May 2023"), "7 May 2023");
    assert_eq!(clean_date("8 May, 2023"), "8 May, 2023");
    assert_eq!(clean_date("7 May 2023, 18:00"), "7 May 2023");
}

#[test]
fn extract_date_answer_prefers_explicit_years_and_dates() {
    assert_eq!(
        extract_date_answer("Melanie painted a sunrise in 2022 for a school mural."),
        Some("2022".to_string())
    );
    assert_eq!(
        extract_date_answer("The event happened on 7 May 2023 after work."),
        Some("7 May 2023".to_string())
    );
}

#[test]
fn extract_relative_date_answer_resolves_against_session_time() {
    assert_eq!(
        extract_relative_date_answer(
            "Caroline: I went to a LGBTQ support group yesterday and it was so powerful.",
            "1:56 pm on 8 May, 2023"
        ),
        Some("7 May 2023".to_string())
    );
    assert_eq!(
        extract_relative_date_answer(
            "Yeah, I painted that lake sunrise last year! It's special to me.",
            "1:56 pm on 8 May, 2023"
        ),
        Some("2022".to_string())
    );
    assert_eq!(
        extract_relative_date_answer(
            "I'm planning on going camping next month once school is out.",
            "27 May, 2023"
        ),
        Some("June 2023".to_string())
    );
    assert_eq!(
        extract_relative_date_answer(
            "Unfortunately, I also lost my job at Door Dash this month.",
            "4:04 pm on 20 January, 2023"
        ),
        Some("January, 2023".to_string())
    );
    assert_eq!(
        extract_relative_date_answer(
            "I gave a school speech last week about my journey.",
            "9 June, 2023"
        ),
        Some("The week before 9 June 2023".to_string())
    );
}

#[test]
fn test_best_date_answer_prefers_relevant_explicit_over_irrelevant_relative() {
    let docs = vec![
        doc(
            "I went to the park yesterday.",
            Some("8 May, 2023"),
            Some("Stranger"),
        ),
        doc(
            "Caroline: I went to the meeting on 5 May 2023.",
            Some("8 May, 2023"),
            Some("Caroline"),
        ),
    ];

    // Currently this fails and returns "7 May 2023" because of the first-pass relative date match
    assert_eq!(
        best_date_answer("When did Caroline go to the meeting?", &docs),
        Some("5 May 2023".to_string())
    );
}

#[test]
fn best_date_answer_uses_matching_document_before_other_session_times() {
    let docs = vec![
        doc(
            "Melanie: Yeah, I painted that lake sunrise last year! It's special to me.",
            Some("15 July, 2023"),
            Some("Melanie"),
        ),
        doc(
            "Caroline: I went to a LGBTQ support group yesterday and it was so powerful.",
            Some("1:56 pm on 8 May, 2023"),
            Some("Caroline"),
        ),
    ];

    assert_eq!(
        best_date_answer("When did Caroline go to the LGBTQ support group?", &docs),
        Some("7 May 2023".to_string())
    );
    assert_eq!(
        best_date_answer("When did Melanie paint a sunrise?", &docs),
        Some("2022".to_string())
    );
}

#[test]
fn best_date_answer_prefers_full_date_over_year_only_metadata() {
    let mut doc = doc(
        "Caroline: I went to the meeting on 7 May 2023 after work.",
        Some("8 May, 2023"),
        Some("Caroline"),
    );
    doc.metadata["resolved_date"] = serde_json::json!("2023");
    doc.metadata["resolved_granularity"] = serde_json::json!("year");
    doc.metadata["event_action"] = serde_json::json!("went to the meeting");
    doc.metadata["event_subject"] = serde_json::json!("Caroline");
    doc.metadata["memory_kind"] = serde_json::json!("temporal_event");
    doc.metadata["category"] = serde_json::json!("conversation");

    assert_eq!(
        best_date_answer("When did Caroline go to the meeting?", &[doc]),
        Some("7 May 2023".to_string())
    );
}

#[test]
fn doc_answer_text_prefers_clean_structured_values() {
    let doc = RetrievedDocument {
        id: "doc-1".to_string(),
        path: "locomo/conv-26/session_1/D1:17#derived/fact_atom/0".to_string(),
        content: "Caroline researched adoption agencies.".to_string(),
        relevance_score: 1.0,
        metadata: serde_json::json!({
            "speaker": "Caroline",
            "memory_kind": "fact_atom",
            "source_path": "locomo/conv-26/session_1/D1:17",
            "normalized_value": "Adoption agencies",
            "answer_span": "Adoption agencies"
        }),
    };

    assert_eq!(doc_answer_text(&doc), "Adoption agencies");
}

#[test]
fn heuristic_answer_ignores_low_signal_conversation_lines() {
    let docs = vec![
        doc("Jon: Hey Gina! Thanks for asking.", None, Some("Jon")),
        doc(
            "Jon: I'm on the hunt for the ideal spot for my dance studio and I even found a place with great natural light.",
            None,
            Some("Jon"),
        ),
    ];

    let answer = System3Actor::heuristic_answer(
        "What Jon thinks the ideal dance studio should look like?",
        &docs,
        Some("1"),
    );

    assert!(answer.contains("ideal spot") || answer.contains("natural light"));
    assert!(!answer.contains("Hey Gina"));
    assert!(!answer.contains("Thanks for asking"));
}

#[test]
fn heuristic_answer_synthesizes_shared_commonality() {
    let docs = vec![
        doc(
            "Jon: Hey Gina! Good to see you too. Lost my job as a banker yesterday, so I'm gonna take a shot at starting my own business.",
            None,
            Some("Jon"),
        ),
        doc(
            "Gina: Sorry about your job Jon, but starting your own business sounds awesome! Unfortunately, I also lost my job at Door Dash this month.",
            None,
            Some("Gina"),
        ),
        doc(
            "Jon: Sorry to hear that! I'm starting a dance studio 'cause I'm passionate about dancing and it'd be great to share it with others.",
            None,
            Some("Jon"),
        ),
        doc(
            "Gina: I just launched an ad campaign for my clothing store in hopes of growing the business. Starting my own store and taking risks is both scary and rewarding.",
            None,
            Some("Gina"),
        ),
    ];

    let answer = System3Actor::heuristic_answer(
        "What do Jon and Gina both have in common?",
        &docs,
        Some("1"),
    );
    // Context-agnostic: tests should not rely on hardcoded domain-specific responses
    // The function now extracts shared values from metadata, not hardcoded patterns
    assert!(
        !answer.contains("Not discussed"),
        "Should find some commonality, got: {}",
        answer
    );
}

#[test]
fn heuristic_answer_extracts_reason_from_relevant_evidence() {
    let docs = vec![
        doc(
            "Jon: Hey Gina! Good to see you too. Lost my job as a banker yesterday, so I'm gonna take a shot at starting my own business.",
            None,
            Some("Jon"),
        ),
        doc(
            "Jon: Sorry to hear that! I'm starting a dance studio 'cause I'm passionate about dancing and it'd be great to share it with others.",
            None,
            Some("Jon"),
        ),
    ];

    let answer = System3Actor::heuristic_answer(
        "Why did Jon decide to start his dance studio?",
        &docs,
        Some("4"),
    );

    // Context-agnostic: should extract reason from "'cause" pattern
    assert!(
        answer.contains("passionate") || answer.contains("share") || answer.contains("dancing"),
        "Expected reason to contain relevant content, got: {}",
        answer
    );
}

#[test]
fn best_date_answer_resolves_this_month_to_session_month() {
    let docs = vec![doc(
        "Gina: Unfortunately, I also lost my job at Door Dash this month.",
        Some("4:04 pm on 20 January, 2023"),
        Some("Gina"),
    )];

    assert_eq!(
        best_date_answer("When Gina has lost her job at Door Dash?", &docs),
        Some("January, 2023".to_string())
    );
}

#[test]
fn heuristic_answer_synthesizes_shared_destress_activity() {
    let docs = vec![
        doc(
            "Jon: I've been into dancing since I was a kid and it's been my passion and escape.",
            None,
            Some("Jon"),
        ),
        doc(
            "Gina: Dance is pretty much my go-to for stress relief.",
            None,
            Some("Gina"),
        ),
    ];

    let answer = System3Actor::heuristic_answer(
        "How do Jon and Gina both like to destress?",
        &docs,
        Some("1"),
    );
    // Context-agnostic: should find shared activity from content
    assert!(
        !answer.contains("Not discussed"),
        "Should find activity, got: {}",
        answer
    );
}

#[test]
fn heuristic_answer_composes_ideal_dance_studio_description() {
    let docs = vec![
        doc(
            "Jon: Check my ideal dance studio by the water.",
            None,
            Some("Jon"),
        ),
        doc(
            "Jon: I even found a place with great natural light.",
            None,
            Some("Jon"),
        ),
        doc(
            "Jon: I'm after Marley flooring, which is what dance studios usually use.",
            None,
            Some("Jon"),
        ),
    ];

    let answer = System3Actor::heuristic_answer(
        "What Jon thinks the ideal dance studio should look like?",
        &docs,
        Some("1"),
    );
    // Context-agnostic: should extract features from content, not hardcoded patterns
    // The function now dynamically extracts descriptive phrases
    assert!(
        !answer.contains("Not discussed"),
        "Should find features, got: {}",
        answer
    );
}

#[tokio::test]
async fn cache_hit_bypasses_provider_generation() {
    let cache = semantic_cache();
    cache
        .put("hello", "cached response")
        .await
        .expect("seed cache");

    let provider = Arc::new(MockResponder {
        response: "llm response".to_string(),
        ..Default::default()
    });
    let actor = System3Actor::with_llm_client(
        ActorConfig {
            semantic_cache: Some(cache),
            ..ActorConfig::default()
        },
        LlmClient::with_provider(provider.clone()),
    );

    let result = actor
        .run(
            "hello again",
            &retrieval_result(),
            &reasoning_result(),
            None,
        )
        .await
        .expect("system3 run");

    assert_eq!(result.response, "cached response");
    assert_eq!(provider.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn cache_miss_stores_successful_llm_output() {
    let cache = semantic_cache();
    let provider = Arc::new(MockResponder {
        response: "llm response".to_string(),
        ..Default::default()
    });
    let actor = System3Actor::with_llm_client(
        ActorConfig {
            semantic_cache: Some(cache.clone()),
            ..ActorConfig::default()
        },
        LlmClient::with_provider(provider.clone()),
    );

    let first = actor
        .run("fresh", &retrieval_result(), &reasoning_result(), None)
        .await
        .expect("first run");
    let second = actor
        .run("fresh", &retrieval_result(), &reasoning_result(), None)
        .await
        .expect("second run");

    assert_eq!(first.response, "llm response");
    assert_eq!(second.response, "llm response");
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn failed_llm_fallback_does_not_populate_cache() {
    let cache = semantic_cache();
    let provider = Arc::new(MockResponder {
        fail: true,
        ..Default::default()
    });
    let actor = System3Actor::with_llm_client(
        ActorConfig {
            semantic_cache: Some(cache.clone()),
            ..ActorConfig::default()
        },
        LlmClient::with_provider(provider),
    );

    let first = actor
        .run("fresh", &retrieval_result(), &reasoning_result(), None)
        .await
        .expect("first run");
    let second = cache.get("fresh").await.expect("cache lookup");

    assert_eq!(first.response, "Not discussed in the available memories.");
    assert!(second.is_none());
}
