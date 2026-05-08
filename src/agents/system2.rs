//! System 2 - Reasoner Agent
//!
//! Recibe contexto de System 1, analiza y razona sobre la información.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::agents::system1::RetrievalResult;

/// Response del System 2
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningResult {
    pub query: String,
    pub analysis: String,
    pub confidence: f32,
    pub supporting_evidence: Vec<Evidence>,
    pub beliefs_updated: Vec<BeliefUpdate>,
    pub reasoning_chain: Vec<ReasoningStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub source_id: String,
    pub content: String,
    pub relevance: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeliefUpdate {
    pub concept: String,
    pub relation: String,
    pub target: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningStep {
    pub step: usize,
    pub thought: String,
    pub conclusion: String,
}

/// Configuración del Reasoner
#[derive(Debug, Clone)]
pub struct ReasonerConfig {
    pub max_evidence: usize,
    pub min_confidence: f32,
}

impl Default for ReasonerConfig {
    fn default() -> Self {
        Self {
            max_evidence: 5,
            min_confidence: 0.5,
        }
    }
}

/// System 2 - Reasoner Agent (simplificado)
/// System 2 - Reasoner Agent (simplificado)
pub struct System2Reasoner {
    config: ReasonerConfig,
    provider: crate::agents::provider::ModelProviderClient,
}

impl System2Reasoner {
    pub fn new(config: ReasonerConfig) -> Self {
        Self {
            config,
            provider: crate::agents::provider::ModelProviderClient::from_env(),
        }
    }

    pub fn with_provider(config: ReasonerConfig, provider: crate::agents::provider::ModelProviderClient) -> Self {
        Self { config, provider }
    }

    pub async fn run(&self, query: &str, context: &RetrievalResult) -> Result<ReasoningResult> {
        let start = std::time::Instant::now();

        info!("🧠 System2 reasoning for query: {}", query);

        // Simple reasoning - just extract evidence from retrieval
        let evidence: Vec<Evidence> = context
            .documents
            .iter()
            .take(self.config.max_evidence)
            .map(|doc| Evidence {
                source_id: doc.id.clone(),
                content: doc.content.clone(),
                relevance: doc.relevance_score,
            })
            .collect();

        // Self-RAG: Use LLM to evaluate confidence if evidence exists
        let confidence = if evidence.is_empty() {
            0.0
        } else {
            match self
                .provider
                .evaluate_context(query, &context.documents)
                .await
            {
                Ok(conf) => {
                    info!(
                        "🔍 Self-RAG evaluation scored context confidence: {:.2}",
                        conf
                    );
                    conf
                }
                Err(e) => {
                    tracing::warn!(
                        "Self-RAG evaluation failed: {}. Using fallback heuristic.",
                        e
                    );
                    evidence.iter().map(|e| e.relevance).sum::<f32>() / evidence.len() as f32
                }
            }
        };

        let analysis = format!(
            "Found {} relevant documents for query '{}'. Self-RAG Confidence: {:.2}",
            evidence.len(),
            query,
            confidence
        );

        info!("✅ System2 completed in {:?}", start.elapsed());

        Ok(ReasoningResult {
            query: query.to_string(),
            analysis,
            confidence,
            supporting_evidence: evidence,
            beliefs_updated: vec![],
            reasoning_chain: vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reasoner_config() {
        let config = ReasonerConfig::default();
        assert_eq!(config.max_evidence, 5);
    }
}
