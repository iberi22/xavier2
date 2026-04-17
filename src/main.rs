// Xavier2 - Cognitive Memory System
// Public open-core release

mod cli;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
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
