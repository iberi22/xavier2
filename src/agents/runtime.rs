//! Runtime / Orchestrator
//!
//! Coordina System 1 → 2 → 3, maneja timeouts y errores.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info};

use crate::context::orchestrator::Orchestrator;

use crate::utils::crypto::sha256_hex;

use crate::agents::router::{RouteCategory, Router};
use crate::agents::system1::{RetrieverConfig, System1Retriever};
use crate::agents::system2::{ReasonerConfig, System2Reasoner};
use crate::agents::system3::{ActorConfig, System3Actor};
use crate::checkpoint::{Checkpoint, CheckpointManager};
use crate::memory::belief_graph::SharedBeliefGraph;
use crate::memory::qmd_memory::QmdMemory;
use crate::memory::schema::MemoryQueryFilters;
use crate::scheduler::JobScheduler;

/// Estado de la sesión
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub messages: Vec<ConversationMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub id: String,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

/// Response final del agente
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    pub session_id: String,
    pub query: String,
    pub response: String,
    pub confidence: f32,
    pub system_timings: SystemTimings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRunTrace {
    pub agent: AgentResponse,
    pub retrieval: crate::agents::system1::RetrievalResult,
    pub reasoning: crate::agents::system2::ReasoningResult,
    pub action: crate::agents::system3::ActionResult,
    pub optimization: RunOptimizationTrace,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunOptimizationTrace {
    pub route_category: RouteCategory,
    pub semantic_cache_hit: bool,
    pub llm_used: bool,
    pub model: Option<String>,
    pub query_fingerprint: String,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum System3Mode {
    #[default]
    Auto,
    Disabled,
    Required,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemTimings {
    pub system1_ms: u64,
    pub system2_ms: u64,
    pub system3_ms: u64,
    pub total_ms: u64,
}

/// Configuración del Runtime
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub timeout_seconds: u64,
    pub max_retries: usize,
    pub model_provider: Option<String>,
    pub model_api_key: Option<String>,
    pub model_url: Option<String>,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: 30,
            max_retries: 2,
            model_provider: None,
            model_api_key: None,
            model_url: None,
        }
    }
}

impl RuntimeConfig {
    pub fn from_env() -> Self {
        Self::from_env_with(|name| std::env::var(name).ok())
    }

    fn from_env_with<F>(mut lookup: F) -> Self
    where
        F: FnMut(&str) -> Option<String>,
    {
        let mut config = Self::default();

        if let Some(provider) =
            lookup("XAVIER2_MODEL_PROVIDER").map(|value| value.trim().to_ascii_lowercase())
        {
            if provider == "local" {
                if let Some(url) = lookup("XAVIER2_LOCAL_LLM_URL").filter(|v| !v.trim().is_empty())
                {
                    config.model_provider = Some(provider);
                    config.model_url = Some(url);
                    return config;
                }
            }

            let env_var = match provider.as_str() {
                "gemini" => Some("GEMINI_API_KEY"),
                "openai" => Some("OPENAI_API_KEY"),
                "minimax" => Some("MINIMAX_API_KEY"),
                "deepseek" => Some("DEEPSEEK_API_KEY"),
                "anthropic" => Some("ANTHROPIC_API_KEY"),
                _ => None,
            };

            if let Some(env_var) = env_var {
                config.model_provider = Some(provider);
                config.model_api_key = lookup(env_var).filter(|value| !value.trim().is_empty());
                return config;
            }
        }

        if let Some(url) = lookup("XAVIER2_LOCAL_LLM_URL").filter(|v| !v.trim().is_empty()) {
            config.model_provider = Some("local".to_string());
            config.model_url = Some(url);
            return config;
        }

        for (provider, env_var) in [
            ("gemini", "GEMINI_API_KEY"),
            ("openai", "OPENAI_API_KEY"),
            ("minimax", "MINIMAX_API_KEY"),
            ("deepseek", "DEEPSEEK_API_KEY"),
            ("anthropic", "ANTHROPIC_API_KEY"),
        ] {
            if let Some(api_key) = lookup(env_var).filter(|value| !value.trim().is_empty()) {
                config.model_provider = Some(provider.to_string());
                config.model_api_key = Some(api_key);
                break;
            }
        }

        config
    }
}

use crate::memory::semantic_cache::SemanticCache;

/// Runtime que orquesta los tres sistemas
pub struct AgentRuntime {
    memory: Arc<QmdMemory>,
    semantic_cache: Arc<SemanticCache>,
    router: Router,
    system1: System1Retriever,
    system2: System2Reasoner,
    config: RuntimeConfig,
    checkpoint_manager: Option<Arc<CheckpointManager>>,
    scheduler: Option<Arc<tokio::sync::Mutex<JobScheduler>>>,
    orchestrator: Option<Orchestrator>,
}

impl AgentRuntime {
    pub fn new(
        memory: Arc<QmdMemory>,
        belief_graph: Option<SharedBeliefGraph>,
        config: RuntimeConfig,
    ) -> Result<Self> {
        let semantic_cache = Arc::new(SemanticCache::new(0.95)?);
        Ok(Self {
            system1: System1Retriever::new(
                Arc::clone(&memory),
                belief_graph,
                RetrieverConfig::default(),
            ),
            memory,
            semantic_cache,
            router: Router::new(),
            system2: System2Reasoner::new(ReasonerConfig::default()),
            config,
            checkpoint_manager: None,
            scheduler: None,
            orchestrator: None,
        })
    }

    pub fn with_checkpoint_manager(mut self, manager: Arc<CheckpointManager>) -> Self {
        self.checkpoint_manager = Some(manager);
        self
    }

    pub fn with_scheduler(mut self, scheduler: JobScheduler) -> Self {
        self.scheduler = Some(Arc::new(tokio::sync::Mutex::new(scheduler)));
        self
    }

    pub fn with_orchestrator(mut self, orchestrator: Orchestrator) -> Self {
        self.orchestrator = Some(orchestrator);
        self
    }

    pub fn checkpoint_manager(&self) -> Option<&Arc<CheckpointManager>> {
        self.checkpoint_manager.as_ref()
    }

    pub async fn scheduler(&self) -> Option<tokio::sync::MutexGuard<'_, JobScheduler>> {
        if let Some(scheduler) = &self.scheduler {
            Some(scheduler.lock().await)
        } else {
            None
        }
    }

    pub fn memory(&self) -> Arc<QmdMemory> {
        Arc::clone(&self.memory)
    }

    pub fn config(&self) -> &RuntimeConfig {
        &self.config
    }

    /// Ejecuta el ciclo completo: System 1 → System 2 → System 3
    pub async fn run(
        &self,
        query: &str,
        session_id: Option<String>,
        category: Option<String>,
    ) -> Result<AgentResponse> {
        Ok(self
            .run_with_trace_filtered(query, session_id, category, None, System3Mode::Auto)
            .await?
            .agent)
    }

    pub async fn run_with_trace(
        &self,
        query: &str,
        session_id: Option<String>,
        category: Option<String>,
    ) -> Result<AgentRunTrace> {
        self.run_with_trace_filtered(query, session_id, category, None, System3Mode::Auto)
            .await
    }

    pub async fn run_with_trace_filtered(
        &self,
        query: &str,
        session_id: Option<String>,
        category: Option<String>,
        filters: Option<MemoryQueryFilters>,
        system3_mode: System3Mode,
    ) -> Result<AgentRunTrace> {
        let start = std::time::Instant::now();
        let session_id = session_id.unwrap_or_else(|| ulid::Ulid::new().to_string());
        let query_fingerprint = query_fingerprint(query);

        // Fire session_start hook into context orchestrator (fire-and-forget)
        if let Some(ref orch) = self.orchestrator {
            orch.session_start(&session_id, query, &[]);
            debug!(session_id = %session_id, "context_orchestrator: session_start");
        }

        info!("🚀 Starting agent runtime for session: {}", session_id);
        if let Some(provider) = &self.config.model_provider {
            debug!("Using configured model provider: {}", provider);
        }

        let route = self.router.classify(query);
        debug!(
            query_fingerprint = %query_fingerprint,
            route_category = ?route.category,
            "router_classification"
        );

        let mut current_query = query.to_string();
        let mut retries = 0;
        let mut s1_total_ms = 0;
        let mut s2_total_ms = 0;

        // 1. Check Semantic Cache
        if let Ok(Some(cached_response)) = self.semantic_cache.get(query).await {
            info!("⚡ Semantic Cache Hit! Returning zero-token cost response.");
            let agent = AgentResponse {
                session_id,
                query: query.to_string(),
                response: cached_response.clone(),
                confidence: 1.0,
                system_timings: SystemTimings {
                    system1_ms: 0,
                    system2_ms: 0,
                    system3_ms: 0,
                    total_ms: start.elapsed().as_millis() as u64,
                },
            };
            return Ok(AgentRunTrace {
                agent,
                // Provide empty/default results for the bypassed systems
                retrieval: crate::agents::system1::RetrievalResult {
                    query: query.to_string(),
                    documents: vec![],
                    search_type: crate::agents::system1::SearchType::Semantic,
                    total_results: 1,
                },
                reasoning: crate::agents::system2::ReasoningResult {
                    query: query.to_string(),
                    analysis: "Cached Response".to_string(),
                    confidence: 1.0,
                    supporting_evidence: vec![],
                    beliefs_updated: vec![],
                    reasoning_chain: vec![],
                },
                action: crate::agents::system3::ActionResult {
                    query: query.to_string(),
                    response: cached_response,
                    actions_taken: vec![],
                    memory_updates: vec![],
                    tool_calls: vec![],
                    success: true,
                    semantic_cache_hit: true,
                    llm_used: false,
                    model: None,
                },
                optimization: RunOptimizationTrace {
                    route_category: route.category,
                    semantic_cache_hit: true,
                    llm_used: false,
                    model: None,
                    query_fingerprint,
                },
            });
        }

        if route.category == RouteCategory::Direct {
            let response = self
                .router
                .direct_response(query)
                .unwrap_or_else(|| System3Actor::simple_response(query, &[], category.as_deref()));
            let total_ms = start.elapsed().as_millis() as u64;
            let agent = AgentResponse {
                session_id,
                query: query.to_string(),
                response: response.clone(),
                confidence: 1.0,
                system_timings: SystemTimings {
                    system1_ms: 0,
                    system2_ms: 0,
                    system3_ms: 0,
                    total_ms,
                },
            };
            return Ok(AgentRunTrace {
                agent,
                retrieval: crate::agents::system1::RetrievalResult {
                    query: query.to_string(),
                    documents: vec![],
                    search_type: crate::agents::system1::SearchType::Semantic,
                    total_results: 0,
                },
                reasoning: crate::agents::system2::ReasoningResult {
                    query: query.to_string(),
                    analysis: "Direct route".to_string(),
                    confidence: 1.0,
                    supporting_evidence: vec![],
                    beliefs_updated: vec![],
                    reasoning_chain: vec![],
                },
                action: crate::agents::system3::ActionResult {
                    query: query.to_string(),
                    response,
                    actions_taken: vec![],
                    memory_updates: vec![],
                    tool_calls: vec![],
                    success: true,
                    semantic_cache_hit: false,
                    llm_used: false,
                    model: None,
                },
                optimization: RunOptimizationTrace {
                    route_category: route.category,
                    semantic_cache_hit: false,
                    llm_used: false,
                    model: None,
                    query_fingerprint,
                },
            });
        }

        let (retrieval_result, reasoning_result) = loop {
            // System 1: Retrieval
            let s1_start = std::time::Instant::now();
            let retrieval_result = self
                .system1
                .run_with_filters(&current_query, None, filters.as_ref())
                .await?;
            s1_total_ms += s1_start.elapsed().as_millis() as u64;

            debug!(
                "✅ System 1 completed in {}ms (retry {})",
                s1_start.elapsed().as_millis(),
                retries
            );

            // System 2: Reasoning
            let s2_start = std::time::Instant::now();
            let reasoning_result = self.system2.run(&current_query, &retrieval_result).await?;
            s2_total_ms += s2_start.elapsed().as_millis() as u64;

            debug!(
                "✅ System 2 completed in {}ms (retry {})",
                s2_start.elapsed().as_millis(),
                retries
            );

            // Reflection / Context Paging Loop
            if reasoning_result.confidence >= 0.7 || retries >= self.config.max_retries {
                break (retrieval_result, reasoning_result);
            }

            info!(
                "⚠️ System 2 confidence too low ({:.2}). Auto-reflecting and expanding query...",
                reasoning_result.confidence
            );

            // Fire precompact hook before expanding context
            if let Some(ref orch) = self.orchestrator {
                orch.precompact(&session_id, &current_query, &[]);
                debug!(session_id = %session_id, "context_orchestrator: precompact");
            }

            current_query = format!("{} (expanded context needed)", current_query);
            retries += 1;
        };

        // System 3: Action
        let should_skip_system3 = match system3_mode {
            System3Mode::Disabled => true,
            System3Mode::Required => false,
            System3Mode::Auto => should_answer_from_evidence(
                query,
                category.as_deref(),
                route.category,
                &retrieval_result,
                &reasoning_result,
            ),
        };

        let (action_result, s3_ms) = if should_skip_system3 {
            (
                crate::agents::system3::ActionResult {
                    query: query.to_string(),
                    response: System3Actor::simple_response(
                        query,
                        &retrieval_result.documents,
                        category.as_deref(),
                    ),
                    actions_taken: vec![],
                    memory_updates: vec![],
                    tool_calls: vec![],
                    success: true,
                    semantic_cache_hit: false,
                    llm_used: false,
                    model: None,
                },
                0,
            )
        } else {
            let selected_model_override = self
                .router
                .resolve_model_override(route.category, &retrieval_result, &reasoning_result)
                .or(route.model_override.clone());
            let system3 = System3Actor::new(ActorConfig {
                semantic_cache: Some(Arc::clone(&self.semantic_cache)),
                model_override: selected_model_override,
                provider_override: self.config.model_provider.clone(),
                ..ActorConfig::default()
            });
            let s3_start = std::time::Instant::now();
            let action_result = system3
                .run(
                    query,
                    &retrieval_result,
                    &reasoning_result,
                    category.as_deref(),
                )
                .await?;
            (action_result, s3_start.elapsed().as_millis() as u64)
        };

        debug!("✅ System 3 completed in {}ms", s3_ms);

        let total_ms = start.elapsed().as_millis() as u64;

        info!("✅ Agent runtime completed: {}ms total", total_ms);
        info!(
            query_fingerprint = %query_fingerprint,
            route_category = ?route.category,
            semantic_cache_hit = action_result.semantic_cache_hit,
            llm_used = action_result.llm_used,
            model = action_result.model.as_deref().unwrap_or("none"),
            retrieval_results = retrieval_result.total_results,
            total_ms,
            "runtime_optimization"
        );

        let agent = AgentResponse {
            session_id,
            query: query.to_string(),
            response: action_result.response.clone(),
            confidence: reasoning_result.confidence,
            system_timings: SystemTimings {
                system1_ms: s1_total_ms,
                system2_ms: s2_total_ms,
                system3_ms: s3_ms,
                total_ms,
            },
        };
        let optimization = RunOptimizationTrace {
            route_category: route.category,
            semantic_cache_hit: action_result.semantic_cache_hit,
            llm_used: action_result.llm_used,
            model: action_result.model.clone(),
            query_fingerprint,
        };

        Ok(AgentRunTrace {
            agent,
            retrieval: retrieval_result,
            reasoning: reasoning_result,
            action: action_result,
            optimization,
        })
    }
}

fn should_answer_from_evidence(
    query: &str,
    category: Option<&str>,
    route_category: RouteCategory,
    retrieval_result: &crate::agents::system1::RetrievalResult,
    reasoning_result: &crate::agents::system2::ReasoningResult,
) -> bool {
    if route_category == RouteCategory::Complex || retrieval_result.documents.is_empty() {
        return false;
    }

    let lowered = query.to_ascii_lowercase();
    let explicit_category = category.unwrap_or_default();
    let deterministic_shape = lowered.starts_with("when ")
        || lowered.starts_with("where ")
        || lowered.starts_with("who ")
        || lowered.starts_with("what ")
        || lowered.starts_with("which ")
        || explicit_category == "1"
        || explicit_category == "2";
    let top_doc = retrieval_result.documents.first();
    let has_specific_evidence = top_doc.is_some_and(|doc| {
        doc.metadata.get("resolved_date").is_some()
            || doc.metadata.get("normalized_value").is_some()
            || doc.metadata.get("answer_span").is_some()
            || doc
                .metadata
                .get("evidence_kind")
                .and_then(|value| value.as_str())
                .is_some()
            || doc
                .metadata
                .get("provenance")
                .and_then(|value| value.as_object())
                .is_some_and(|value| {
                    value.contains_key("symbol")
                        || value.contains_key("file_path")
                        || value.contains_key("url")
                        || value.contains_key("message_id")
                })
    });

    deterministic_shape && has_specific_evidence && reasoning_result.confidence >= 0.7
}

impl AgentRuntime {
    pub async fn save_checkpoint(
        &self,
        task_id: &str,
        name: &str,
        data: serde_json::Value,
    ) -> Result<()> {
        if let Some(manager) = &self.checkpoint_manager {
            let checkpoint = Checkpoint::new(task_id.to_string(), name.to_string(), data);
            manager.save(checkpoint).await?;
        }
        Ok(())
    }

    pub async fn load_checkpoint(
        &self,
        task_id: &str,
        name: &str,
    ) -> Result<Option<serde_json::Value>> {
        if let Some(manager) = &self.checkpoint_manager {
            let checkpoint = manager.load(task_id.to_string(), name.to_string()).await?;
            return Ok(checkpoint.map(|c| c.data));
        }
        Ok(None)
    }

    pub async fn list_checkpoints(&self, task_id: &str) -> Result<Vec<String>> {
        if let Some(manager) = &self.checkpoint_manager {
            let checkpoints = manager.list(task_id.to_string()).await?;
            return Ok(checkpoints.into_iter().map(|c| c.name).collect());
        }
        Ok(Vec::new())
    }
}

fn query_fingerprint(query: &str) -> String {
    sha256_hex(query.as_bytes())[..12].to_string()
}

/// Builder para crear el runtime
pub struct RuntimeBuilder {
    config: RuntimeConfig,
    memory: Option<Arc<QmdMemory>>,
    belief_graph: Option<SharedBeliefGraph>,
    checkpoint_manager: Option<Arc<CheckpointManager>>,
    scheduler: Option<JobScheduler>,
}

impl RuntimeBuilder {
    pub fn new() -> Self {
        Self {
            config: RuntimeConfig::default(),
            memory: None,
            belief_graph: None,
            checkpoint_manager: None,
            scheduler: None,
        }
    }

    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.config.timeout_seconds = seconds;
        self
    }

    pub fn with_memory(mut self, memory: Arc<QmdMemory>) -> Self {
        self.memory = Some(memory);
        self
    }

    pub fn with_belief_graph(mut self, belief_graph: SharedBeliefGraph) -> Self {
        self.belief_graph = Some(belief_graph);
        self
    }

    pub fn with_checkpoint_manager(mut self, manager: Arc<CheckpointManager>) -> Self {
        self.checkpoint_manager = Some(manager);
        self
    }

    pub fn with_scheduler(mut self, scheduler: JobScheduler) -> Self {
        self.scheduler = Some(scheduler);
        self
    }

    pub fn build(self) -> Result<AgentRuntime> {
        let memory = self
            .memory
            .ok_or_else(|| anyhow::anyhow!("RuntimeBuilder requires a memory backend"))?;
        let runtime = AgentRuntime::new(memory, self.belief_graph, self.config)?;
        let runtime = if let Some(manager) = self.checkpoint_manager {
            runtime.with_checkpoint_manager(manager)
        } else {
            runtime
        };
        Ok(if let Some(scheduler) = self.scheduler {
            runtime.with_scheduler(scheduler)
        } else {
            runtime
        })
    }
}

impl Default for RuntimeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::router::RouteCategory;

    #[test]
    fn test_runtime_config() {
        let config = RuntimeConfig::default();
        assert_eq!(config.timeout_seconds, 30);
        assert!(config.model_api_key.is_none());
    }

    #[test]
    fn test_runtime_config_from_env_prefers_minimax() {
        let config = RuntimeConfig::from_env_with(|name| match name {
            "GEMINI_API_KEY" => Some("gemini-key".to_string()),
            "OPENAI_API_KEY" => Some("openai-key".to_string()),
            _ => None,
        });

        assert_eq!(config.model_provider.as_deref(), Some("gemini"));
        assert_eq!(config.model_api_key.as_deref(), Some("gemini-key"));
    }

    #[test]
    fn direct_route_is_detected_for_short_greeting() {
        let router = Router::new();
        let decision = router.classify("hello");

        assert_eq!(decision.category, RouteCategory::Direct);
        assert!(decision.should_skip_retrieval);
        assert!(decision.should_skip_reasoning);
    }
}
