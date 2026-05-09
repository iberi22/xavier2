//! code-graph - Codebase Understanding without RAG
//!
//! A fast, lightweight codebase indexer using tree-sitter AST parsing.
//! Inspired by Aider's tree-sitter strategy and Claude Code's agentic search.
//!
//! Note: Storage - use SQLite for standalone CLI, or SurrealDB via Xavier integration

pub mod db;
pub mod debug;
pub mod indexer;
pub mod parser;
pub mod query;

pub use error::GraphError;
pub use types::*;

mod error;
pub mod types;
