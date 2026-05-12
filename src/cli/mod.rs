//! Xavier CLI - Command-line interface
//!
//! Split from monolithic src/cli.rs into submodules:
//! - `state`: CliState and Cli structs
//! - `commands`: Command enum and subcommand dispatch
//! - `config`: Environment and configuration resolution
//! - `security`: CLI input validation and sanitization
//! - `code_graph`: Code graph query helpers
//! - `utils`: Utility functions
//! - `server`: HTTP server and route handlers
//! - `mcp`: MCP stdio server
//! - `tests`: Integration tests

pub mod state;
pub mod commands;
pub(crate) mod config;
pub(crate) mod security;
pub(crate) mod code_graph;
pub(crate) mod utils;
pub(crate) mod server;
pub(crate) mod mcp;
#[cfg(test)]
mod tests;

// Re-exports for backward compatibility (main.rs uses `use cli::Cli`)
pub use state::Cli;
pub use commands::Command;
