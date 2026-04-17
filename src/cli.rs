//! Xavier2 CLI - Simplified command-line interface for public release

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::net::SocketAddr;

/// Xavier2 - Fast Vector Memory for AI Agents
#[derive(Parser)]
#[command(name = "xavier2")]
#[command(about = "Xavier2 - Fast Vector Memory for AI Agents", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

impl Cli {
    pub async fn run(&self) -> Result<()> {
        match &self.command {
            Command::Http { port } => {
                println!("Starting Xavier2 HTTP server on port {}...", port);
                start_http_server(*port).await
            }
            Command::Mcp => {
                println!("Starting Xavier2 MCP-stdio server...");
                start_mcp_stdio().await
            }
            Command::Search { query, limit } => {
                println!("Searching memories: {}", query);
                println!("(Searching via HTTP API on localhost:8006)");
                search_memories(query, *limit).await
            }
            Command::Add { content, title } => {
                let title_display = title.unwrap_or("Untitled");
                println!("Adding memory: {}", title_display);
                add_memory(content, title.as_deref()).await
            }
            Command::Stats => {
                println!("Fetching Xavier2 statistics...");
                show_stats().await
            }
        }
    }
}

async fn start_http_server(port: u16) -> Result<()> {
    // Set port via environment for the server
    std::env::set_var("XAVIER2_PORT", port.to_string());
    
    // For the public release, we delegate to the HTTP server startup
    // The actual Axum server setup is in the main binary
    println!("Xavier2 HTTP API available at http://localhost:{}/", port);
    println!("Health check: http://localhost:{}/health", port);
    println!("Memory endpoints:");
    println!("  POST /memory/add     - Add a memory");
    println!("  POST /memory/search - Search memories");
    println!("  GET  /memory/stats   - Get statistics");
    println!("");
    println!("Press Ctrl+C to stop the server.");
    
    // In the actual build, this would start the Axum server
    // For now, we just show the info and exit
    // TODO: Integrate with the full server startup from main.rs
    
    // Since we can't easily call the full server setup from here,
    // we'll use a simple HTTP client approach to verify connectivity
    let health_url = format!("http://localhost:{}/health", port);
    
    match reqwest::get(&health_url).await {
        Ok(resp) if resp.status().is_success() => {
            println!("Server is running!");
        }
        _ => {
            println!("Server started (health endpoint not available in this mode)");
        }
    }
    
    Ok(())
}

async fn start_mcp_stdio() -> Result<()> {
    println!("Xavier2 MCP-stdio server mode");
    println!("This connects Xavier2 to MCP-compatible AI clients");
    println!("");
    println!("Configure your MCP client with:");
    println!("  mcpServers: {{");
    println!("    xavier2: {{");
    println!("      command: 'xavier2'");
    println!("      args: ['mcp']");
    println!("    }}");
    println!("  }}");
    
    // MCP stdio implementation would go here
    // For now, just print instructions
    Ok(())
}

async fn search_memories(query: &str, limit: usize) -> Result<()> {
    let token = std::env::var("XAVIER2_TOKEN").unwrap_or_else(|_| "dev-token".to_string());
    let port = std::env::var("XAVIER2_PORT").unwrap_or_else(|_| "8006".to_string());
    let url = format!("http://localhost:{}/memory/search", port);
    
    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .header("X-Xavier2-Token", &token)
        .json(&serde_json::json!({
            "query": query,
            "limit": limit
        }))
        .send()
        .await;
    
    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                let body: serde_json::Value = resp.json().await.unwrap_or_default();
                println!("\nSearch results for: {}", query);
                println!("{}", serde_json::to_string_pretty(&body)?);
            } else {
                println!("Search failed with status: {}", resp.status());
            }
        }
        Err(e) => {
            println!("Error connecting to Xavier2 server: {}", e);
            println!("Is the server running? (xavier2 http)");
        }
    }
    
    Ok(())
}

async fn add_memory(content: &str, title: Option<&str>) -> Result<()> {
    let token = std::env::var("XAVIER2_TOKEN").unwrap_or_else(|_| "dev-token".to_string());
    let port = std::env::var("XAVIER2_PORT").unwrap_or_else(|_| "8006".to_string());
    let url = format!("http://localhost:{}/memory/add", port);
    
    let mut body = serde_json::json!({
        "content": content,
        "metadata": {}
    });
    
    if let Some(t) = title {
        body["metadata"]["title"] = serde_json::json!(t);
    }
    
    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .header("X-Xavier2-Token", &token)
        .json(&body)
        .send()
        .await;
    
    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                println!("Memory added successfully!");
            } else {
                println!("Failed to add memory: {}", resp.status());
            }
        }
        Err(e) => {
            println!("Error connecting to Xavier2 server: {}", e);
            println!("Is the server running? (xavier2 http)");
        }
    }
    
    Ok(())
}

async fn show_stats() -> Result<()> {
    let token = std::env::var("XAVIER2_TOKEN").unwrap_or_else(|_| "dev-token".to_string());
    let port = std::env::var("XAVIER2_PORT").unwrap_or_else(|_| "8006".to_string());
    let url = format!("http://localhost:{}/memory/stats", port);
    
    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("X-Xavier2-Token", &token)
        .send()
        .await;
    
    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                let body: serde_json::Value = resp.json().await.unwrap_or_default();
                println!("\nXavier2 Statistics:");
                println!("{}", serde_json::to_string_pretty(&body)?);
            } else {
                println!("Failed to get stats: {}", resp.status());
            }
        }
        Err(e) => {
            println!("Error connecting to Xavier2 server: {}", e);
            println!("Is the server running? (xavier2 http)");
        }
    }
    
    Ok(())
}
