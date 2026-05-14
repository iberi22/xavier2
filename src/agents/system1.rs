//! System 1 - Retriever Agent
//!
//! Recibe queries del usuario, busca en memoria híbrida y retorna contexto relevante.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn};

use crate::memory::belief_graph::SharedBeliefGraph;
use crate::memory::qmd_memory::{query_with_embedding_filtered, QmdMemory};
use crate::memory::schema::{resolve_metadata, EvidenceKind, MemoryKind, MemoryQueryFilters};

/// Response del System 1
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalResult {
    pub query: String,
    pub documents: Vec<RetrievedDocument>,
    pub search_type: SearchType,
    pub total_results: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievedDocument {
    pub id: String,
    pub path: String,
    pub content: String,
    pub relevance_score: f32,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SearchType {
    Hybrid,
    Semantic,
    Keyword,
}

/// Configuración del Retriever
#[derive(Debug, Clone)]
pub struct RetrieverConfig {
    pub max_results: usize,
    pub min_relevance_score: f32,
    pub default_search_type: SearchType,
    pub use_hyde: bool,
}

impl Default for RetrieverConfig {
    fn default() -> Self {
        Self {
            max_results: 10,
            min_relevance_score: 0.3,
            default_search_type: SearchType::Hybrid,
            use_hyde: hyde_enabled_from_env(),
        }
    }
}

fn hyde_enabled_from_env() -> bool {
    std::env::var("XAVIER_DISABLE_HYDE")
        .ok()
        .map(|value| {
            !matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(true)
}

/// System 1 - Retriever Agent (simplificado para compilación)
pub struct System1Retriever {
    memory: Arc<QmdMemory>,
    belief_graph: Option<SharedBeliefGraph>,
    config: RetrieverConfig,
    provider: crate::agents::provider::ModelProviderClient,
}

impl System1Retriever {
    /// Crea un nuevo retriever
    pub fn new(
        memory: Arc<QmdMemory>,
        belief_graph: Option<SharedBeliefGraph>,
        config: RetrieverConfig,
    ) -> Self {
        Self {
            memory,
            belief_graph,
            config,
            provider: crate::agents::provider::ModelProviderClient::from_env(),
        }
    }

    /// Ejecuta la retrieval de manera asíncrona
    pub async fn run(
        &self,
        query: &str,
        search_type: Option<SearchType>,
    ) -> Result<RetrievalResult> {
        self.run_with_filters(query, search_type, None).await
    }

    pub async fn run_with_filters(
        &self,
        query: &str,
        search_type: Option<SearchType>,
        filters: Option<&MemoryQueryFilters>,
    ) -> Result<RetrievalResult> {
        let start = std::time::Instant::now();

        info!("🔍 System1 starting retrieval for query: {}", query);

        let search_query = if self.config.use_hyde {
            match self.provider.generate_hypothetical_document(query).await {
                Ok(hypothetical_doc) => {
                    info!("🧠 HyDE: Generated hypothetical document correctly.");
                    format!("{}\n\n{}", query, hypothetical_doc)
                }
                Err(e) => {
                    warn!(
                        "🧠 HyDE generation failed: {}. Falling back to standard query.",
                        e
                    );
                    query.to_string()
                }
            }
        } else {
            query.to_string()
        };

        let selected_search_type = search_type.unwrap_or(self.config.default_search_type.clone());
        let keyword_search = || async {
            self.memory
                .search_with_cache_filtered(&search_query, self.config.max_results, filters)
                .await
                .map(|result| result.documents)
        };
        let raw_documents = match selected_search_type {
            SearchType::Keyword => keyword_search().await?,
            SearchType::Hybrid => {
                match query_with_embedding_filtered(
                    &self.memory,
                    &search_query,
                    self.config.max_results,
                    filters,
                )
                .await
                {
                    Ok(results) => results,
                    Err(error) => {
                        warn!(
                            "Hybrid search failed, falling back to keyword search: {}",
                            error
                        );
                        keyword_search().await?
                    }
                }
            }
            SearchType::Semantic => {
                let semantic_results = query_with_embedding_filtered(
                    &self.memory,
                    &search_query,
                    self.config.max_results,
                    filters,
                )
                .await;

                match semantic_results {
                    Ok(results) if !results.is_empty() => results,
                    Ok(_) => {
                        warn!("Semantic search returned no results, falling back to hybrid search");
                        let hybrid_results = self
                            .memory
                            .search_filtered(&search_query, self.config.max_results, filters)
                            .await?;

                        if hybrid_results.is_empty() {
                            warn!(
                                "Hybrid search returned no results, falling back to keyword search"
                            );
                            keyword_search().await?
                        } else {
                            hybrid_results
                        }
                    }
                    Err(error) => {
                        warn!(
                            "Semantic search failed, falling back to hybrid search: {}",
                            error
                        );
                        let hybrid_results = self
                            .memory
                            .search_filtered(&search_query, self.config.max_results, filters)
                            .await?;

                        if hybrid_results.is_empty() {
                            warn!(
                                "Hybrid search returned no results, falling back to keyword search"
                            );
                            keyword_search().await?
                        } else {
                            hybrid_results
                        }
                    }
                }
            }
        };

        let mut documents: Vec<RetrievedDocument> = raw_documents
            .into_iter()
            .enumerate()
            .map(|(index, doc)| RetrievedDocument {
                id: doc
                    .id
                    .clone()
                    .unwrap_or_else(|| format!("memory:{}", index)),
                path: doc.path,
                content: doc.content,
                relevance_score: 1.0,
                metadata: doc.metadata,
            })
            .collect();

        // --- HIERARCHICAL EXPANSION ---
        // Identify common domain/topic from top results
        let mut domain_counts = HashMap::new();
        let mut topic_counts = HashMap::new();

        for doc in documents.iter().take(3) {
            if let Some(domain) = doc.metadata.get("domain").and_then(|d| d.as_str()) {
                *domain_counts.entry(domain.to_string()).or_insert(0) += 1;
            }
            if let Some(topic) = doc.metadata.get("topic").and_then(|t| t.as_str()) {
                *topic_counts.entry(topic.to_string()).or_insert(0) += 1;
            }
        }

        let top_domain = domain_counts
            .into_iter()
            .max_by_key(|&(_, count)| count)
            .map(|(d, _)| d);
        let top_topic = topic_counts
            .into_iter()
            .max_by_key(|&(_, count)| count)
            .map(|(t, _)| t);

        if let Some(domain) = top_domain {
            info!(
                "🧠 Hierarchical Expansion: Searching for more in domain '{}'",
                domain
            );
            if let Ok(more_docs) = self.memory.search_filtered(&domain, 2, filters).await {
                for doc in more_docs {
                    if !documents
                        .iter()
                        .any(|d| d.id == doc.id.clone().unwrap_or_default() || d.path == doc.path)
                    {
                        documents.push(RetrievedDocument {
                            id: doc.id.unwrap_or_default(),
                            path: doc.path,
                            content: doc.content,
                            relevance_score: 0.8,
                            metadata: doc.metadata,
                        });
                    }
                }
            }
        }

        if let Some(topic) = top_topic {
            info!(
                "🧠 Hierarchical Expansion: Searching for more in topic '{}'",
                topic
            );
            if let Ok(more_docs) = self.memory.search_filtered(&topic, 2, filters).await {
                for doc in more_docs {
                    if !documents
                        .iter()
                        .any(|d| d.id == doc.id.clone().unwrap_or_default() || d.path == doc.path)
                    {
                        documents.push(RetrievedDocument {
                            id: doc.id.unwrap_or_default(),
                            path: doc.path,
                            content: doc.content,
                            relevance_score: 0.8,
                            metadata: doc.metadata,
                        });
                    }
                }
            }
        }

        // --- BELIEF GRAPH AUGMENTATION ---
        if let Some(graph_lock) = &self.belief_graph {
            let graph = graph_lock.read().await;
            let beliefs = graph.search(query).await;
            if !beliefs.is_empty() {
                info!(
                    "🧠 Graph Augmentation: Found {} related beliefs",
                    beliefs.len()
                );
                let belief_text = beliefs
                    .iter()
                    .take(5)
                    .map(|b| format!("FACT: {} {} {}", b.subject, b.predicate, b.object))
                    .collect::<Vec<_>>()
                    .join("\n");

                documents.push(RetrievedDocument {
                    id: "belief_graph_context".to_string(),
                    path: "belief_graph".to_string(),
                    content: belief_text,
                    relevance_score: 0.9,
                    metadata: serde_json::json!({"type": "graph_context"}),
                });
            }
        }

        rank_documents_for_query(query, &mut documents);

        let total = documents.len();

        info!(
            "✅ System1 retrieved {} documents in {:?}",
            total,
            start.elapsed()
        );

        Ok(RetrievalResult {
            query: query.to_string(),
            documents,
            search_type: selected_search_type,
            total_results: total,
        })
    }
}

fn query_terms(query: &str) -> Vec<String> {
    query
        .to_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .filter(|term| {
            let term = *term;
            term.len() > 2
                && !matches!(
                    term,
                    "when"
                        | "what"
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
                        | "the"
                        | "and"
                        | "for"
                        | "how"
                        | "why"
                        | "was"
                        | "were"
                )
        })
        .map(|term| term.to_string())
        .collect()
}

fn detect_query_kind(query: &str) -> &'static str {
    let lowered = query.to_lowercase();
    if lowered.contains("when") || lowered.contains("date") || lowered.contains("year") {
        "temporal"
    } else {
        "factual"
    }
}

fn metadata_text(doc: &RetrievedDocument, key: &str) -> String {
    doc.metadata
        .get(key)
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_lowercase()
}

fn rank_documents_for_query(query: &str, documents: &mut [RetrievedDocument]) {
    let terms = query_terms(query);
    let kind = detect_query_kind(query);
    let query_lower = query.to_lowercase();

    documents.sort_by(|left, right| {
        let left_score = document_rank_score(left, &terms, &query_lower, kind);
        let right_score = document_rank_score(right, &terms, &query_lower, kind);
        right_score
            .cmp(&left_score)
            .then_with(|| left.path.cmp(&right.path))
    });
}

fn document_rank_score(
    doc: &RetrievedDocument,
    terms: &[String],
    query_lower: &str,
    kind: &str,
) -> usize {
    let resolved = resolve_metadata(
        &doc.path,
        &doc.metadata,
        doc.metadata
            .get("namespace")
            .and_then(|value| value.get("workspace_id"))
            .and_then(|value| value.as_str())
            .unwrap_or("default"),
        None,
    )
    .ok();
    let content = doc.content.to_lowercase();
    let speaker = metadata_text(doc, "speaker");
    let memory_kind = metadata_text(doc, "memory_kind");
    let normalized_value = metadata_text(doc, "normalized_value");
    let answer_span = metadata_text(doc, "answer_span");
    let resolved_date = metadata_text(doc, "resolved_date");
    let event_action = metadata_text(doc, "event_action");
    let doc_category = doc
        .metadata
        .get("category")
        .and_then(|value| value.as_str())
        .unwrap_or_default();

    let mut score = 0usize;
    for term in terms {
        if content.contains(term) {
            score += 3;
        }
        if speaker == *term {
            score += 4;
        }
        if event_action.contains(term) {
            score += 5;
        }
        if normalized_value.contains(term) || answer_span.contains(term) {
            score += 4;
        }
    }

    if !speaker.is_empty() && query_lower.contains(&speaker) {
        score += 6;
    }

    if let Some(resolved) = &resolved {
        match resolved.kind {
            MemoryKind::Repo | MemoryKind::File | MemoryKind::Symbol | MemoryKind::Url => {
                score += 8;
            }
            MemoryKind::Decision | MemoryKind::Task => score += 4,
            _ => {}
        }

        match resolved.evidence_kind {
            Some(EvidenceKind::TemporalEvent) if kind == "temporal" => score += 10,
            Some(EvidenceKind::FactAtom | EvidenceKind::EntityState) if kind != "temporal" => {
                score += 8
            }
            Some(EvidenceKind::SourceTurn) => score += 4,
            Some(EvidenceKind::SessionSummary) => score = score.saturating_sub(8),
            _ => {}
        }

        for exact in [
            resolved.provenance.symbol.as_ref(),
            resolved.provenance.file_path.as_ref(),
            resolved.provenance.repo_url.as_ref(),
            resolved.provenance.url.as_ref(),
            resolved.namespace.session_id.as_ref(),
            resolved.namespace.agent_id.as_ref(),
            resolved.namespace.user_id.as_ref(),
            resolved.namespace.project.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            if query_lower.contains(&exact.to_ascii_lowercase()) {
                score += 10;
            }
        }
    }

    match kind {
        "temporal" => {
            if !resolved_date.is_empty() {
                score += 8;
            }
            if !event_action.is_empty() {
                score += 4;
            }
        }
        _ => {
            if !normalized_value.is_empty() || !answer_span.is_empty() {
                score += 8;
            }
            if matches!(
                memory_kind.as_str(),
                "fact_atom" | "entity_state" | "summary_fact"
            ) {
                score += 6;
            }
        }
    }

    if doc_category == "session_summary" {
        score = score.saturating_sub(6);
    }

    score
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retriever_config_defaults() {
        let config = RetrieverConfig::default();

        assert_eq!(config.max_results, 10);
        assert_eq!(config.min_relevance_score, 0.3);
        assert!(matches!(config.default_search_type, SearchType::Hybrid));
        assert!(config.use_hyde);
    }

    #[test]
    fn disable_hyde_env_values_are_respected() {
        assert!(!matches_hyde_disable_value("1"));
        assert!(!matches_hyde_disable_value("true"));
        assert!(!matches_hyde_disable_value("yes"));
        assert!(!matches_hyde_disable_value("on"));
        assert!(matches_hyde_disable_value("0"));
        assert!(matches_hyde_disable_value("false"));
    }

    fn matches_hyde_disable_value(value: &str) -> bool {
        !matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    }
}
