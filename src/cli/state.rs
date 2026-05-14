//! CLI application state

use super::Command;
use clap::Parser;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use xavier::agents::rate_limit::RateLimitManager;
use xavier::coordination::{KeyLendingEngine, XavierEventBus};
use xavier::memory::session_store::SessionStore;
use xavier::memory::store::MemoryStore;
use xavier::ports::inbound::{AgentLifecyclePort, MemoryQueryPort};
use xavier::security::SecurityService;
use xavier::tasks::store::{InMemoryTaskStore, TaskService};
use xavier::time::TimeMetricsStore;
use xavier::AppState;

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
    pub event_bus: XavierEventBus,
    pub tasks: Arc<TaskService<InMemoryTaskStore>>,
    pub rate_manager: Arc<RateLimitManager>,
    pub prompt_cache: Arc<Mutex<HashMap<String, Vec<String>>>>,
}

#[derive(Parser)]
#[command(name = "xavier", version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Xavier - Fast Vector Memory for AI Agents", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Option<Command>,
}
