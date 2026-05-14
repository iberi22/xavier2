use std::collections::HashMap;

use serde_json::Value;

use super::{
    bm25::{tokenize, Bm25Hit, Bm25Index},
    ContextDocument,
};
use crate::retrieval::config;

#[derive(Debug, Clone, PartialEq)]
pub struct ContextSearchHit {
    pub document: ContextDocument,
    pub score: f32,
    pub sources: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct HybridContextSearch {
    rrf_k: u32,
}

impl Default for HybridContextSearch {
    fn default() -> Self {
        Self::new(config::DEFAULT_RRF_K)
    }
}

impl HybridContextSearch {
    pub fn new(rrf_k: u32) -> Self {
        Self { rrf_k }
    }

    pub fn search(
        &self,
        documents: &[ContextDocument],
        query: &str,
        limit: usize,
    ) -> Vec<ContextSearchHit> {
        if documents.is_empty() || query.trim().is_empty() || limit == 0 {
            return Vec::new();
        }

        let lexical_hits =
            Bm25Index::new(documents.to_vec()).search(query, limit.saturating_mul(2));
        let metadata_hits = metadata_keyword_search(documents, query, limit.saturating_mul(2));
        let fused = reciprocal_rank_fusion(
            vec![
                ("bm25".to_string(), lexical_hits),
                ("metadata".to_string(), metadata_hits),
            ],
            self.rrf_k,
        );

        fused.into_iter().take(limit).collect()
    }
}

fn metadata_keyword_search(
    documents: &[ContextDocument],
    query: &str,
    limit: usize,
) -> Vec<Bm25Hit> {
    let query_terms = tokenize(query);
    if query_terms.is_empty() {
        return Vec::new();
    }

    let mut hits = Vec::new();
    for document in documents {
        let mut haystack = String::new();
        haystack.push_str(&document.role);
        haystack.push(' ');
        haystack.push_str(&document.tool_calls.join(" "));
        haystack.push(' ');
        haystack.push_str(&flatten_metadata(&document.metadata));

        let tokens = tokenize(&haystack);
        if tokens.is_empty() {
            continue;
        }

        let mut score = 0.0;
        for term in &query_terms {
            score += tokens.iter().filter(|token| *token == term).count() as f32;
        }

        if score > 0.0 {
            hits.push(Bm25Hit {
                document: document.clone(),
                score,
            });
        }
    }

    hits.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| right.document.created_at.cmp(&left.document.created_at))
            .then_with(|| left.document.id.cmp(&right.document.id))
    });
    hits.truncate(limit);
    hits
}

fn reciprocal_rank_fusion(
    ranked_lists: Vec<(String, Vec<Bm25Hit>)>,
    rrf_k: u32,
) -> Vec<ContextSearchHit> {
    let mut fused: HashMap<String, ContextSearchHit> = HashMap::new();

    for (source, hits) in ranked_lists {
        for (index, hit) in hits.into_iter().enumerate() {
            let rank = (index as u32) + 1;
            let contribution = 1.0 / ((rrf_k + rank) as f32);

            fused
                .entry(hit.document.id.clone())
                .and_modify(|existing| {
                    existing.score += contribution;
                    if !existing.sources.iter().any(|item| item == &source) {
                        existing.sources.push(source.clone());
                    }
                })
                .or_insert_with(|| ContextSearchHit {
                    document: hit.document,
                    score: contribution,
                    sources: vec![source.clone()],
                });
        }
    }

    let mut hits: Vec<_> = fused.into_values().collect();
    hits.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| right.document.created_at.cmp(&left.document.created_at))
            .then_with(|| left.document.id.cmp(&right.document.id))
    });
    hits
}

fn flatten_metadata(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(boolean) => boolean.to_string(),
        Value::Number(number) => number.to_string(),
        Value::String(string) => string.clone(),
        Value::Array(items) => items
            .iter()
            .map(flatten_metadata)
            .filter(|item| !item.is_empty())
            .collect::<Vec<_>>()
            .join(" "),
        Value::Object(map) => map
            .iter()
            .flat_map(|(key, value)| [key.clone(), flatten_metadata(value)])
            .filter(|item| !item.is_empty())
            .collect::<Vec<_>>()
            .join(" "),
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use serde_json::json;

    use super::*;

    fn doc(
        id: &str,
        content: &str,
        tools: &[&str],
        metadata: Value,
        seconds: i64,
    ) -> ContextDocument {
        ContextDocument::new(id, "session-1", "assistant", content)
            .with_tool_calls(tools.iter().map(|tool| tool.to_string()).collect())
            .with_metadata(metadata)
            .with_created_at(
                Utc.timestamp_opt(seconds, 0)
                    .single()
                    .expect("test assertion"),
            )
    }

    #[test]
    fn fuses_lexical_and_metadata_hits_with_rrf() {
        let documents = vec![
            doc(
                "1",
                "investigate build failure in rust workspace",
                &[],
                json!({"surface": "cli"}),
                1,
            ),
            doc(
                "2",
                "quick status update",
                &["cargo_test", "git_status"],
                json!({"surface": "terminal", "topic": "build"}),
                2,
            ),
            doc(
                "3",
                "build playbook for cargo_test failures",
                &["cargo_test"],
                json!({"surface": "terminal", "topic": "build"}),
                3,
            ),
        ];

        let hits = HybridContextSearch::default().search(&documents, "build cargo_test", 10);

        assert_eq!(hits.len(), 3);
        assert_eq!(hits[0].document.id, "3");
        assert!(hits[0].sources.iter().any(|source| source == "metadata"));
        assert!(hits[0].sources.iter().any(|source| source == "bm25"));
        assert!(hits[0].score > hits[1].score);
    }

    #[test]
    fn returns_empty_for_blank_query() {
        let documents = vec![doc("1", "hello", &[], json!({}), 1)];
        assert!(HybridContextSearch::default()
            .search(&documents, "   ", 5)
            .is_empty());
    }
}
