use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::agents::{runtime::AgentRunTrace, system1::RetrievedDocument, system2::Evidence};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiRenderResult {
    pub plain_text: String,
    pub openui_lang: String,
    pub components: Vec<String>,
    pub rules_applied: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct UiRenderAgent;

impl UiRenderAgent {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, trace: &AgentRunTrace) -> UiRenderResult {
        let mut components = vec!["Card".to_string(), "TextCallout".to_string()];
        let mut rules = vec!["always_show_summary".to_string()];
        let plain_text = trace.agent.response.clone();

        let mut blocks = vec![
            format!(
                "<Card title=\"Xavier Response\" description=\"Rendered by the internal UI agent\"><TextContent content=\"{}\" /></Card>",
                escape_attr(&plain_text)
            ),
            format!(
                "<Card title=\"System Signals\" description=\"Reasoning and retrieval quality\">\
                    <ListBlock items='{}' />\
                </Card>",
                escape_attr(&metric_items(trace))
            ),
        ];

        components.push("Table".to_string());
        rules.push("show_metric_table_when_trace_exists".to_string());

        if !trace.reasoning.supporting_evidence.is_empty() {
            blocks.push(render_evidence_block(&trace.reasoning.supporting_evidence));
            components.push("ListBlock".to_string());
            rules.push("show_evidence_when_available".to_string());
        }

        if !trace.retrieval.documents.is_empty() {
            for doc in &trace.retrieval.documents {
                let doc_type = doc
                    .metadata
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                match doc_type {
                    "question" => {
                        blocks.push(format!(
                            "<QuestionCard question='{}' />",
                            escape_attr(&doc.content)
                        ));
                        components.push("QuestionCard".to_string());
                    }
                    "decision" | "adr" => {
                        blocks.push(format!(
                            "<DecisionCard adr='{}' />",
                            escape_attr(&doc.content)
                        ));
                        components.push("DecisionCard".to_string());
                    }
                    "project" => {
                        blocks.push(format!(
                            "<ProjectCard project='{}' />",
                            escape_attr(&doc.content)
                        ));
                        components.push("ProjectCard".to_string());
                    }
                    _ => {}
                }
            }

            blocks.push(render_documents_block(&trace.retrieval.documents));
            components.push("Accordion".to_string());
            rules.push("show_memory_hits_when_documents_available".to_string());
        }

        if trace.agent.confidence < 0.55 {
            blocks.push(
                "<TextCallout variant=\"warning\" title=\"Low confidence answer\" content=\"Xavier found weak or partial evidence. Review the supporting memory before acting on this output.\" />".to_string(),
            );
            rules.push("warn_when_confidence_low".to_string());
        } else {
            blocks.push(format!(
                "<TextCallout variant=\"success\" title=\"Backed by memory\" content=\"{}\" />",
                escape_attr(&format!("Rendered at {}", Utc::now().to_rfc3339()))
            ));
            rules.push("confirm_when_confidence_high".to_string());
        }

        let openui_lang = format!(
            "<SectionBlock title=\"Xavier Internal Panel\" description=\"Reasoning output rendered through the dedicated UI agent\">{}</SectionBlock>",
            blocks.join("")
        );

        UiRenderResult {
            plain_text,
            openui_lang,
            components,
            rules_applied: rules,
        }
    }
}

fn render_evidence_block(evidence: &[Evidence]) -> String {
    format!(
        "<Card title=\"Supporting Evidence\" description=\"Highest-signal excerpts used by the render agent\">\
            <ListBlock items='{}' />\
        </Card>",
        escape_attr(
            &serde_json::to_string(
                &evidence
                    .iter()
                    .map(|item| {
                        serde_json::json!({
                            "title": item.source_id,
                            "description": item.content.chars().take(180).collect::<String>(),
                            "meta": format!("relevance {:.2}", item.relevance)
                        })
                    })
                    .collect::<Vec<_>>()
            )
            .unwrap_or_else(|_| "[]".to_string())
        )
    )
}

fn render_documents_block(documents: &[RetrievedDocument]) -> String {
    format!(
        "<Card title=\"Memory Hits\" description=\"Retrieved documents the UI agent chose to surface\">\
            <Accordion items='{}' />\
        </Card>",
        escape_attr(
            &serde_json::to_string(
                &documents
                    .iter()
                    .take(5)
                    .map(|doc| {
                        serde_json::json!({
                            "title": doc.path,
                            "content": doc.content.chars().take(260).collect::<String>()
                        })
                    })
                    .collect::<Vec<_>>()
            )
            .unwrap_or_else(|_| "[]".to_string())
        )
    )
}

fn metric_items(trace: &AgentRunTrace) -> String {
    serde_json::to_string(&vec![
        serde_json::json!({
            "title": "Confidence",
            "description": format!("{:.0}% estimated answer confidence", trace.agent.confidence * 100.0),
        }),
        serde_json::json!({
            "title": "Documents",
            "description": format!("{} retrieved memory candidates", trace.retrieval.total_results),
        }),
        serde_json::json!({
            "title": "Latency",
            "description": format!("{} ms total runtime", trace.agent.system_timings.total_ms),
        }),
    ])
    .unwrap_or_else(|_| "[]".to_string())
}

fn escape_attr(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::{
        router::RouteCategory,
        runtime::{AgentResponse, AgentRunTrace, RunOptimizationTrace, SystemTimings},
        system1::{RetrievalResult, SearchType},
        system2::ReasoningResult,
        system3::ActionResult,
    };

    #[test]
    fn render_includes_rich_components() {
        let trace = AgentRunTrace {
            agent: AgentResponse {
                session_id: "session".to_string(),
                query: "What happened?".to_string(),
                response: "Here is the answer".to_string(),
                confidence: 0.82,
                system_timings: SystemTimings {
                    system1_ms: 10,
                    system2_ms: 11,
                    system3_ms: 12,
                    total_ms: 33,
                },
            },
            retrieval: RetrievalResult {
                query: "What happened?".to_string(),
                documents: vec![RetrievedDocument {
                    id: "doc-1".to_string(),
                    path: "memory/doc-1".to_string(),
                    content: "Relevant memory".to_string(),
                    relevance_score: 1.0,
                    metadata: serde_json::json!({}),
                }],
                search_type: SearchType::Hybrid,
                total_results: 1,
            },
            reasoning: ReasoningResult {
                query: "What happened?".to_string(),
                analysis: "analysis".to_string(),
                confidence: 0.82,
                supporting_evidence: vec![Evidence {
                    source_id: "doc-1".to_string(),
                    content: "Relevant memory".to_string(),
                    relevance: 1.0,
                }],
                beliefs_updated: vec![],
                reasoning_chain: vec![],
            },
            action: ActionResult {
                query: "What happened?".to_string(),
                response: "Here is the answer".to_string(),
                actions_taken: vec![],
                memory_updates: vec![],
                tool_calls: vec![],
                success: true,
                semantic_cache_hit: false,
                llm_used: false,
                model: None,
            },
            optimization: RunOptimizationTrace {
                route_category: RouteCategory::Retrieved,
                semantic_cache_hit: false,
                llm_used: false,
                model: None,
                query_fingerprint: "tracefinger".to_string(),
            },
        };

        let result = UiRenderAgent::new().render(&trace);
        assert!(result.openui_lang.contains("Supporting Evidence"));
        assert!(result.components.iter().any(|value| value == "Table"));
    }
}
