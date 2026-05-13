use anyhow::Result;
use tracing::{info, warn};
use crate::agents::system1::{RetrievalResult, RetrievedDocument};
use crate::agents::system2::ReasoningResult;
use super::types::{ActionResult, ActorConfig};
use super::client::LlmClient;
use super::helpers::*;

impl System3Actor {
    pub(crate) fn heuristic_answer(query: &str, docs: &[RetrievedDocument], category: Option<&str>) -> String {
        if docs.is_empty() {
            return "Not discussed in the available memories.".to_string();
        }

        let category = category.unwrap_or_else(|| detect_question_category(query));

        if let Some(answer) = best_shared_fact_answer(query, docs) {
            return answer;
        }

        if let Some(answer) = best_description_answer(query, docs) {
            return snippet(&answer, 220);
        }

        if let Some(answer) = best_structured_fact_answer(query, docs) {
            return answer;
        }

        if let Some(answer) = best_reason_answer(query, docs) {
            return snippet(&answer, 220);
        }

        // Cat 2: Dates - Use best_date_answer with expanded date patterns
        if category == "2" {
            if let Some(answer) = best_date_answer(query, docs) {
                return answer;
            }
            // Fallback: try extracting dates from all docs
            if let Some(answer) = best_category_answer(query, docs, category) {
                return clean_date(&answer);
            }
        }

        // Cat 3: Opinions - Extract sentences with opinion keywords
        if category == "3" {
            if let Some(answer) = best_category_answer(query, docs, category) {
                return snippet(&answer, 220);
            }
            let mut all_opinions = Vec::new();
            for doc in docs {
                let opinions = extract_opinion_sentences(&doc.content);
                if !opinions.is_empty() {
                    all_opinions.push(opinions);
                }
            }
            if !all_opinions.is_empty() {
                return snippet(&all_opinions.join(" "), 300);
            }
        }

        // Cat 4: Actions - Extract sentences with action verbs
        if category == "4" {
            if let Some(answer) = best_category_answer(query, docs, category) {
                return snippet(&answer, 220);
            }
            let mut all_actions = Vec::new();
            for doc in docs {
                let actions = extract_action_sentences(&doc.content);
                if !actions.is_empty() {
                    all_actions.push(actions);
                }
            }
            if !all_actions.is_empty() {
                return snippet(&all_actions.join(" "), 300);
            }
        }

        // Cat 1: Facts - Return full document content (multi-hop support)
        // Multi-hop: "what do X and Y both/both have in common"
        let lowered = query_lower(query);
        if lowered.contains("what do") && lowered.contains("both")
            || lowered.contains("what do") && lowered.contains("have in common")
            || lowered.contains("how do") && lowered.contains("both")
        {
            let joined = top_non_empty_contents(docs, 2).join(" ");
            if !joined.is_empty() {
                return snippet(&joined, 220);
            }
        }

        if let Some(answer) = best_category_answer(query, docs, "1") {
            return snippet(&answer, 160);
        }

        if let Some(sentence) = best_relevant_sentence(query, docs, Some("conversation")) {
            return snippet(&sentence, 220);
        }

        // Cat 1 fallback: Return full content from top docs for multi-hop reasoning
        if docs.len() > 1 {
            let joined = top_non_empty_contents(docs, 3).join(" ");
            if !joined.is_empty() {
                return snippet(&joined, 300);
            }
        }

        if let Some(first) = docs.first() {
            let text = first.content.trim();
            if !text.is_empty() {
                return snippet(text, 220);
            }
        }

        "I found relevant memory, but the best answer could not be synthesized yet.".to_string()
    }
}
/// System 3 - Actor Agent
pub struct System3Actor {
    config: ActorConfig,
    llm_client: LlmClient,
}

impl System3Actor {
    pub fn new(config: ActorConfig) -> Self {
        let llm_client = LlmClient::new(
            config.model_override.clone(),
            config.provider_override.clone(),
        );
        Self { config, llm_client }
    }

    pub fn with_config(
        config: ActorConfig,
        provider_config: crate::agents::provider::ModelProviderConfig,
    ) -> Self {
        let llm_client = LlmClient::with_config(provider_config);
        Self { config, llm_client }
    }

    #[cfg(test)]
    pub(crate) fn with_llm_client(config: ActorConfig, llm_client: LlmClient) -> Self {
        Self { config, llm_client }
    }

    pub async fn run(
        &self,
        query: &str,
        retrieval_result: &RetrievalResult,
        _reasoning_result: &ReasoningResult,
        category: Option<&str>,
    ) -> Result<ActionResult> {
        let query_fingerprint = query_fingerprint(query);
        info!(query_fingerprint = %query_fingerprint, "system3_execute");

        let heuristic_response =
            Self::simple_response(query, &retrieval_result.documents, category);
        let should_prefer_heuristic = query.trim_end().ends_with('?')
            && !retrieval_result.documents.is_empty()
            && heuristic_response != "Not discussed in the available memories.";

        if should_prefer_heuristic {
            return Ok(ActionResult {
                query: query.to_string(),
                response: heuristic_response,
                actions_taken: vec![],
                memory_updates: vec![],
                tool_calls: vec![],
                success: true,
                semantic_cache_hit: false,
                llm_used: false,
                model: self.llm_client.model_label(),
            });
        }

        // Generate response using LLM with context
        let mut llm_used = false;
        let response = if self.config.use_llm {
            if let Some(cache) = &self.config.semantic_cache {
                match cache.get(query).await {
                    Ok(Some(cached_response)) => {
                        info!(query_fingerprint = %query_fingerprint, "[CACHE HIT][SYSTEM3]");
                        return Ok(ActionResult {
                            query: query.to_string(),
                            response: cached_response,
                            actions_taken: vec![],
                            memory_updates: vec![],
                            tool_calls: vec![],
                            success: true,
                            semantic_cache_hit: true,
                            llm_used: false,
                            model: self.llm_client.model_label(),
                        });
                    }
                    Ok(None) => {
                        info!(query_fingerprint = %query_fingerprint, "[CACHE MISS][SYSTEM3]");
                        match self
                            .llm_client
                            .generate_response(query, &retrieval_result.documents)
                            .await
                        {
                            Ok(response) => {
                                llm_used = true;
                                if let Err(error) = cache.put(query, &response).await {
                                    warn!("System3 cache store failed: {}", error);
                                }
                                response
                            }
                            Err(error) => {
                                warn!("LLM generation failed: {}", error);
                                Self::simple_response(query, &retrieval_result.documents, category)
                            }
                        }
                    }
                    Err(error) => {
                        warn!("System3 cache lookup failed: {}", error);
                        match self
                            .llm_client
                            .generate_response(query, &retrieval_result.documents)
                            .await
                        {
                            Ok(response) => {
                                llm_used = true;
                                response
                            }
                            Err(e) => {
                                warn!("LLM generation failed: {}", e);
                                Self::simple_response(query, &retrieval_result.documents, category)
                            }
                        }
                    }
                }
            } else {
                match self
                    .llm_client
                    .generate_response(query, &retrieval_result.documents)
                    .await
                {
                    Ok(response) => {
                        llm_used = true;
                        response
                    }
                    Err(e) => {
                        warn!("LLM generation failed: {}", e);
                        Self::simple_response(query, &retrieval_result.documents, category)
                    }
                }
            }
        } else {
            Self::simple_response(query, &retrieval_result.documents, category)
        };

        Ok(ActionResult {
            query: query.to_string(),
            response,
            actions_taken: vec![],
            memory_updates: vec![],
            tool_calls: vec![],
            success: true,
            semantic_cache_hit: false,
            llm_used,
            model: self.llm_client.model_label(),
        })
    }

    pub fn simple_response(
        query: &str,
        docs: &[RetrievedDocument],
        category: Option<&str>,
    ) -> String {
        Self::heuristic_answer(query, docs, category)
    }
}
