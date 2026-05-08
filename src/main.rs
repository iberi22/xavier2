// Xavier2 - Cognitive Memory System
// Public open-core release

mod cli;
mod chronicle;
mod settings;
extern crate xavier2 as xavier2_lib;

// Re-export memory types for binary crate access
pub use xavier2_lib::memory;
pub use xavier2_lib::workspace;

use crate::settings::Xavier2Settings;
use anyhow::Result;
use clap::Parser;
use cli::Cli;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    if let Some(settings) = Xavier2Settings::load()? {
        settings.apply_to_env();
    }

    // Setup logging
    let log_filter = std::env::var("RUST_LOG")
        .ok()
        .or_else(|| std::env::var("XAVIER2_LOG_LEVEL").ok())
        .unwrap_or_else(|| "info".to_string());

    tracing_subscriber::registry()
        .with(EnvFilter::new(&log_filter))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Parse and run CLI
    let cli = Cli::parse();
    cli.run().await
}
