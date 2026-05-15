//! CLI application state

use super::Command;
use clap::Parser;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use xavier::agents::rate_limit::RateLimitManager;
use xavier::app::proxy_use_case::ProxyUseCase;
use xavier::coordination::{KeyLendingEngine, XavierEventBus};
use xavier::memory::session_store::SessionStore;
use xavier::memory::store::MemoryStore;
use xavier::ports::inbound::{AgentLifecyclePort, MemoryQueryPort};
use xavier::app::security_service::SecurityService;
use xavier::tasks::store::{InMemoryTaskStore, TaskService};
use xavier::time::TimeMetricsStore;


#[derive(Clone)]
pub struct CliState {
    pub memory: Arc<dyn MemoryQueryPort>,
    pub store: Arc<dyn MemoryStore>,
    pub workspace_id: String,
    pub workspace_dir: PathBuf,
    pub code_db: Arc<::code_graph::db::CodeGraphDB>,
    pub code_indexer: Arc<::code_graph::indexer::Indexer>,
    pub code_query: Arc<::code_graph::query::QueryEngine>,
    pub security: Arc<SecurityService>,
    pub _time_store: Option<Arc<TimeMetricsStore>>,
    pub agent_registry: Arc<dyn AgentLifecyclePort>,
    pub panel_store: Arc<SessionStore>,
    pub secrets_engine: Arc<KeyLendingEngine>,
    #[allow(dead_code)]
    // TODO: integrate with future event-driven architecture
    pub event_bus: XavierEventBus,
    #[allow(dead_code)]
    // TODO: integrate with persistent task management
    pub tasks: Arc<TaskService<InMemoryTaskStore>>,
    pub rate_manager: Arc<RateLimitManager>,
    #[allow(dead_code)]
    // TODO: enable structured prompt caching across sessions
    pub prompt_cache: Arc<Mutex<HashMap<String, Vec<String>>>>,
    #[allow(dead_code)]
    // TODO: use for background provider health checks
    pub http_client: reqwest::Client,
    pub proxy_use_case: Arc<ProxyUseCase>,

}

#[derive(Parser)]
#[command(name = "xavier", version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Xavier - Fast Vector Memory for AI Agents", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Option<Command>,
}
