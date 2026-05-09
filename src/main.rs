// Xavier - Cognitive Memory System
// Public open-core release

mod cli;
mod settings;
extern crate xavier as xavier_lib;

// Re-export memory types for binary crate access
pub use xavier_lib::memory;
pub use xavier_lib::workspace;

use crate::settings::XavierSettings;
use anyhow::Result;
use clap::Parser;
use cli::Cli;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    if let Some(settings) = XavierSettings::load()? {
        settings.apply_to_env();
    }

    // Setup logging
    let log_filter = std::env::var("RUST_LOG")
        .ok()
        .or_else(|| std::env::var("XAVIER_LOG_LEVEL").ok())
        .unwrap_or_else(|| "info".to_string());

    tracing_subscriber::registry()
        .with(EnvFilter::new(&log_filter))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Parse and run CLI
    let cli = Cli::parse();
    cli.run().await
}
