//! DevLog generation and management module.
//!
//! Provides tools to transform Markdown documentation into a static blog site.

pub mod generator;
pub mod models;

/// Entry point for devlog CLI commands.
pub async fn handle_command(args: &[String]) -> anyhow::Result<()> {
    if args.is_empty() {
        println!("Usage: xavier devlog <command>");
        println!("Commands: build, serve");
        return Ok(());
    }

    match args[0].as_str() {
        "build" => {
            println!("Building DevLog static site...");
            // TODO: Jules will implement the logic here
            Ok(())
        }
        "serve" => {
            println!("Previewing DevLog on http://localhost:8080...");
            // TODO: Jules will implement the logic here
            Ok(())
        }
        _ => {
            anyhow::bail!("Unknown devlog command: {}", args[0])
        }
    }
}
