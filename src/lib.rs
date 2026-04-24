//! Xavier2 - Cognitive Memory System
#![cfg_attr(feature = "telegram", allow(dead_code))]
//!
//! A cognitive memory system with agent runtime, task management, and native UI.

pub mod a2a;
pub mod agents;
pub mod api;
pub mod checkpoint;
pub mod consistency;
pub mod consolidation;
pub mod coordination;
pub mod crypto;
pub mod embedding;
pub mod memory;
pub mod retrieval;
pub mod scheduler;
pub mod search;
pub mod secrets;
pub mod security;
pub mod server;
pub mod sync;
pub mod tasks;
#[cfg(feature = "telegram")]
pub mod telegram;
pub mod tools;
pub mod ui;
pub mod utils;
pub mod verification;
pub mod workspace;

// Hexagonal architecture modules
pub mod adapters;
pub mod app;
pub mod domain;
pub mod ports;

use std::sync::Arc;

use memory::file_indexer::FileIndexer;
use workspace::WorkspaceRegistry;

use crate::adapters::outbound::vec::pattern_adapter::PatternAdapter;
use crate::app::security_service::SecurityService;

/// Application state for HTTP server
#[derive(Clone)]
pub struct AppState {
    pub workspace_registry: Arc<WorkspaceRegistry>,
    pub code_indexer: Arc<code_graph::indexer::Indexer>,
    pub code_query: Arc<code_graph::query::QueryEngine>,
    pub code_db: Arc<code_graph::db::CodeGraphDB>,
    pub indexer: FileIndexer,
    pub pattern_adapter: Arc<PatternAdapter>,
    pub security_service: Arc<SecurityService>,
}
