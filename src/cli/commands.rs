//! CLI commands and subcommand handling

use crate::cli::config::{require_xavier_token, resolve_base_url, resolve_http_port, xavier_token};
use crate::cli::security::secure_cli_input;

use crate::cli::mcp::start_mcp_stdio;
use crate::cli::server::{
    add_memory_hierarchical, search_memories_filtered, start_http_server, SessionContext, SwarmConfig,
};
use crate::cli::state::Cli;
use anyhow::{anyhow, Result};
use clap::Subcommand;

use std::{collections::HashMap, path::PathBuf, sync::Arc, sync::LazyLock, time::Duration};

use tokio::sync::RwLock;

pub static CLI_HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent(concat!("xavier-cli/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("failed to build HTTP client")
});

use xavier::agents::{Agent, AgentConfig};
use xavier::memory::qmd_memory::{MemoryDocument, QmdMemory};

use xavier::memory::sqlite_vec_store::VecSqliteMemoryStore;
use xavier::memory::store::{MemoryRecord, MemoryStore};


#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    /// Start Xavier HTTP server
    Http { port: Option<u16> },
    /// Start Xavier MCP-stdio server
    Mcp,
    /// Search memories
    Search {
        query: String,
        limit: Option<usize>,
        #[arg(long)]
        cluster: Vec<String>,
        #[arg(long)]
        level: Vec<String>,
    },
    /// Add a memory
    Add {
        content: String,
        title: Option<String>,
        /// Memory type: episodic, semantic, procedural, fact, decision, etc.
        #[arg(short, long)]
        kind: Option<String>,
        #[arg(long)]
        cluster: Option<String>,
        #[arg(long)]
        level: Option<String>,
        #[arg(long)]
        relation: Option<String>,
    },
    /// Recall memories with score-based display
    Recall {
        query: String,
        #[arg(short, long, default_value_t = 10)]
        limit: usize,
    },
    /// Show statistics
    Stats,
    /// Query Xavier code graph
    Code {
        #[command(subcommand)]
        cmd: CodeCommand,
    },
    /// Save current session context to Xavier
    SessionSave { session_id: String, content: String },
    /// Spawn multiple agents with provider routing
    Spawn {
        #[arg(long, default_value_t = 1)]
        count: usize,
        #[arg(short, long)]
        provider: Vec<String>,
        #[arg(short, long)]
        model: Vec<String>,
        #[arg(short, long = "skill")]
        skills: Vec<String>,
        #[arg(short = 'x', long)]
        context: Vec<String>,
        #[arg(short, long)]
        task: Option<String>,
    },
    /// Launch parallel agents with a swarm configuration file
    Swarm {
        #[arg(short, long)]
        config: PathBuf,
        #[arg(short, long, default_value_t = 4)]
        parallel: usize,
    },
    /// Batch spawn agents with provider/model routing
    MultiSpawn {
        #[arg(long, default_value_t = 10)]
        agents: usize,
        #[arg(long, default_value_t = 4)]
        batch: usize,
        #[arg(short, long)]
        provider: Vec<String>,
        #[arg(short, long)]
        model: Vec<String>,
        #[arg(short, long)]
        skills: Vec<String>,
        #[arg(short, long)]
        task: Option<String>,
    },
    /// Subcomando para gestionar Chronicle
    Chronicle {
        #[command(subcommand)]
        cmd: xavier::chronicle::cli::ChronicleCommand,
    },
    /// Manage ephemeral secrets (Clavis)
    Secrets {
        #[command(subcommand)]
        cmd: SecretsCommand,
    },
    /// Manage provider usage and rate limits
    Usage {
        #[command(subcommand)]
        cmd: UsageCommand,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum UsageCommand {
    /// Show current usage status for all providers
    Status,
    /// Manually update a provider's used percentage (for providers without API)
    Update { provider: String, percentage: f32 },
    /// Set a manual cooldown for a provider (in minutes)
    Cooldown { provider: String, minutes: i64 },
}

#[derive(Subcommand, Debug, Clone)]
pub enum CodeCommand {
    /// Scan and index a codebase path
    Scan { path: String },
    /// Find symbols by name
    Find {
        query: String,
        #[arg(short, long, default_value_t = 10)]
        limit: usize,
        #[arg(short, long)]
        kind: Option<String>,
    },
    /// Find outgoing dependencies for a symbol/query
    Dependencies {
        query: String,
        #[arg(short, long, default_value_t = 3)]
        depth: usize,
        #[arg(short, long, default_value_t = 50)]
        limit: usize,
        #[arg(short, long)]
        edge_type: Option<String>,
    },
    /// Find incoming dependencies for a symbol/query
    ReverseDependencies {
        query: String,
        #[arg(short, long, default_value_t = 3)]
        depth: usize,
        #[arg(short, long, default_value_t = 50)]
        limit: usize,
        #[arg(short, long)]
        edge_type: Option<String>,
    },
    /// Trace a basic call chain
    CallChain {
        query: String,
        #[arg(short, long, default_value_t = 3)]
        depth: usize,
        #[arg(short, long, default_value_t = 50)]
        limit: usize,
    },
    /// Show highly connected symbols
    Hubs,
    /// Show complexity hotspots
    Hotspots,
    /// Show code graph stats
    Stats,
}

#[derive(Subcommand, Debug, Clone)]
pub enum SecretsCommand {
    /// Lend a secret to an agent
    Lend {
        secret_name: String,
        agent: String,
        /// Time to live in seconds (default 3600)
        #[arg(short, long, default_value_t = 3600)]
        ttl: u64,
    },
    /// List all active secret leases
    ListLeases,
    /// Revoke a specific lease
    Revoke { token: String },
    /// Check the status of a lease
    Status { token: String },
}

impl Cli {
    pub async fn run(&self) -> Result<()> {
        match self.cmd.as_ref().unwrap_or(&Command::Http { port: None }) {
            Command::Http { port } => {
                let port = port.unwrap_or_else(resolve_http_port);
                start_http_server(port).await
            }
            Command::Mcp => start_mcp_stdio().await,
            Command::Search {
                query,
                limit,
                cluster,
                level,
            } => {
                let base_url = resolve_base_url();
                println!("Searching memories via HTTP API on {}", base_url);
                let lim = limit.unwrap_or(10);
                search_memories_filtered(
                    &query,
                    lim,
                    cluster.clone(),
                    level.clone(),
                )
                .await
            }
            Command::Usage { cmd } => {
                let base_url = resolve_base_url();
                match cmd {
                    UsageCommand::Status => {
                        let token = require_xavier_token()?;
                        let client = CLI_HTTP_CLIENT.clone();
                        let providers = ["opencode-go", "deepseek", "groq", "openai", "anthropic"];
                        println!(
                            "{:<15} | {:<10} | {:<10} | {:<10} | {:<10} | {:<20}",
                            "Provider", "Today", "Weekly", "Monthly", "Cache Hits", "Limited Until"
                        );
                        println!(
                            "{:-<15}-+-{:-<10}-+-{:-<10}-+-{:-<10}-+-{:-<10}-+-{:-<20}",
                            "", "", "", "", "", ""
                        );
                        for p in providers {
                            let resp = client
                                .get(format!("{}/v1/usage/status/{}", base_url, p))
                                .header("X-Xavier-Token", &token)
                                .send()
                                .await?;
                            if resp.status().is_success() {
                                let status: xavier::agents::rate_limit::QuotaStatus =
                                    resp.json().await?;
                                let limited = status
                                    .rate_limited_until
                                    .map(|u| u.to_rfc3339())
                                    .unwrap_or_else(|| "No".to_string());
                                println!(
                                    "{:<15} | {:<10} | {:<10} | {:<10} | {:<10} | {:<20}",
                                    status.provider,
                                    status.used_today,
                                    status.used_weekly,
                                    status.used_monthly,
                                    status.cache_hits,
                                    limited
                                );
                            }
                        }
                        Ok(())
                    }
                    UsageCommand::Update {
                        provider,
                        percentage,
                    } => {
                        let token = require_xavier_token()?;
                        let client = CLI_HTTP_CLIENT.clone();
                        let resp = client.post(format!("{}/v1/usage/update", base_url))
                            .header("X-Xavier-Token", &token)
                            .json(&serde_json::json!({ "provider": provider, "percentage": percentage }))
                            .send().await?;
                        if resp.status().is_success() {
                            println!("✅ Manual usage percentage updated for {}", provider);
                        } else {
                            println!("❌ Failed to update usage: {}", resp.text().await?);
                        }
                        Ok(())
                    }
                    UsageCommand::Cooldown { provider, minutes } => {
                        let token = require_xavier_token()?;
                        let client = CLI_HTTP_CLIENT.clone();
                        let resp = client
                            .post(format!("{}/v1/usage/cooldown", base_url))
                            .header("X-Xavier-Token", &token)
                            .json(&serde_json::json!({ "provider": provider, "minutes": minutes }))
                            .send()
                            .await?;
                        if resp.status().is_success() {
                            println!("✅ Cooldown set for {} ({} minutes)", provider, minutes);
                        } else {
                            println!("❌ Failed to set cooldown: {}", resp.text().await?);
                        }
                        Ok(())
                    }
                }
            }
            Command::Add {
                content,
                title,
                kind,
                cluster,
                level,
                relation,
            } => {
                println!("Adding memory...");
                add_memory_hierarchical(
                    content,
                    title.as_ref().map(|s| s.as_str()),
                    kind.as_deref(),
                    cluster.as_deref(),
                    level.as_deref(),
                    relation.as_deref(),
                )
                .await
            }
            Command::Recall { query, limit } => recall_memories(query, *limit).await,
            Command::Stats => {
                println!("Fetching Xavier statistics...");
                show_stats().await
            }
            Command::Code { cmd } => handle_code_command(cmd.clone()).await,
            Command::SessionSave {
                session_id,
                content,
            } => session_save(session_id, content).await,
            Command::Spawn {
                count,
                provider,
                model,
                skills,
                context,
                task,
            } => {
                spawn_agents(
                    *count,
                    provider.clone(),
                    model.clone(),
                    skills,
                    context,
                    task.as_deref(),
                )
                .await
            }
            Command::MultiSpawn {
                agents,
                batch,
                provider,
                model,
                skills,
                task,
            } => {
                multi_spawn_agents(
                    *agents,
                    *batch,
                    provider.clone(),
                    model.clone(),
                    skills.clone(),
                    task.as_deref(),
                )
                .await
            }
            Command::Swarm { config, parallel } => run_swarm(config.clone(), *parallel).await,
            Command::Chronicle { cmd } => {
                xavier::chronicle::cli::handle_chronicle_command(cmd.clone()).await
            }
            Command::Secrets { cmd } => handle_secrets_command(cmd.clone()).await,
        }
    }
}

pub fn estimate_tokens(text: &str) -> usize {
    (text.len() / 4).max(1)
}

pub async fn session_load(ctx: &str) -> Result<String> {
    let token = require_xavier_token()?;
    let url = format!("{}/memory/search", resolve_base_url());

    let client = CLI_HTTP_CLIENT.clone();

    let response = client
        .get(&url)
        .header("X-Xavier-Token", &token)
        .json(&serde_json::json!({
            "query": format!("path:context/{}/latest", ctx),
            "limit": 1
        }))
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(anyhow!("session_load failed: {}", response.status()));
    }

    let body: serde_json::Value = response.json().await?;
    let _results = body
        .get("results")
        .and_then(|r| r.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let context = body
        .get("results")
        .and_then(|r| r.as_array())
        .and_then(|arr| arr.first())
        .and_then(|doc| doc.get("content"))
        .and_then(|c| c.as_str())
        .map(String::from);

    let tokens_restored = context.as_ref().map(|c| estimate_tokens(c)).unwrap_or(0);

    let session_ctx = SessionContext {
        session_id: ctx.to_string(),
        context,
        tokens_restored,
    };

    serde_json::to_string(&session_ctx)
        .map_err(|e| anyhow!("failed to serialize session context: {}", e))
}

pub async fn add_memory(content: &str, title: Option<&str>, kind: Option<&str>) -> Result<()> {
    let content = secure_cli_input("memory content", content, 1_000_000)?;
    let title = title
        .map(|title| secure_cli_input("memory title", title, 512))
        .transpose()?;
    let token = xavier_token();
    let base_url = resolve_base_url();
    let url = format!("{}/memory/add", base_url);

    let mut body = serde_json::json!({
        "content": content,
        "metadata": {}
    });

    if let Some(t) = title.as_deref() {
        body["metadata"]["title"] = serde_json::json!(t);
    }
    if let Some(k) = kind {
        body["metadata"]["kind"] = serde_json::json!(k);
    }

    let client = CLI_HTTP_CLIENT.clone();

    let response = client
        .post(&url)
        .header("X-Xavier-Token", &token)
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
            println!("Error connecting to Xavier server: {}", e);
            println!("Configured endpoint: {}", base_url);
            println!("Is the server running? (xavier http)");
        }
    }

    Ok(())
}

pub async fn recall_memories(query: &str, limit: usize) -> Result<()> {
    let token = xavier_token();
    let base_url = resolve_base_url();
    let url = format!("{}/memory/search", base_url);

    let body = serde_json::json!({
        "query": query,
        "limit": limit,
        "include_scores": true,
    });

    let client = CLI_HTTP_CLIENT.clone();

    let response = client
        .post(&url)
        .header("X-Xavier-Token", &token)
        .json(&body)
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                let json: serde_json::Value = resp.json().await.unwrap_or_default();
                let results = json["results"].as_array().map(|r| r.len()).unwrap_or(0);
                println!("Found {} results for \"{}\":", results, query);
                if let Some(items) = json["results"].as_array() {
                    for (i, item) in items.iter().enumerate() {
                        let content = item["content"].as_str().unwrap_or("(no content)");
                        let kind = item["metadata"]["kind"].as_str().unwrap_or("unknown");
                        let score = item["score"].as_f64().unwrap_or(0.0);
                        let preview = if content.len() > 120 {
                            format!("{}...", &content[..120])
                        } else {
                            content.to_string()
                        };
                        println!("{:>3}. [{:>12}] σ={:.3}  {}", i + 1, kind, score, preview);
                    }
                }
            } else {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                println!("Recall failed ({}): {}", status, text);
            }
        }
        Err(e) => {
            println!("Error connecting to Xavier server: {}", e);
        }
    }

    Ok(())
}

pub async fn show_stats() -> Result<()> {
    let token = xavier_token();
    let base_url = resolve_base_url();
    let url = format!("{}/memory/stats", base_url);

    let client = CLI_HTTP_CLIENT.clone();

    let response = client
        .get(&url)
        .header("X-Xavier-Token", &token)
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                let body: serde_json::Value = resp.json().await.unwrap_or_default();
                println!("\nXavier Statistics:");
                println!("{}", serde_json::to_string_pretty(&body)?);
            } else {
                println!("Failed to get stats: {}", resp.status());
            }
        }
        Err(e) => {
            println!("Error connecting to Xavier server: {}", e);
            println!("Configured endpoint: {}", base_url);
            println!("Is the server running? (xavier http)");
        }
    }

    Ok(())
}

async fn handle_code_command(cmd: CodeCommand) -> Result<()> {
    let token = require_xavier_token()?;
    let base_url = resolve_base_url();
    let client = CLI_HTTP_CLIENT.clone();


    let response = match cmd {
        CodeCommand::Scan { path } => {
            client
                .post(format!("{}/code/scan", base_url))
                .header("X-Xavier-Token", &token)
                .json(&serde_json::json!({ "path": path }))
                .send()
                .await?
        }
        CodeCommand::Find { query, limit, kind } => {
            client
                .post(format!("{}/code/find", base_url))
                .header("X-Xavier-Token", &token)
                .json(&serde_json::json!({ "query": query, "limit": limit, "kind": kind }))
                .send()
                .await?
        }
        CodeCommand::Dependencies {
            query,
            depth,
            limit,
            edge_type,
        } => {
            client
                .post(format!("{}/code/dependencies", base_url))
                .header("X-Xavier-Token", &token)
                .json(&serde_json::json!({
                    "query": query,
                    "depth": depth,
                    "limit": limit,
                    "edge_type": edge_type
                }))
                .send()
                .await?
        }
        CodeCommand::ReverseDependencies {
            query,
            depth,
            limit,
            edge_type,
        } => {
            client
                .post(format!("{}/code/reverse-dependencies", base_url))
                .header("X-Xavier-Token", &token)
                .json(&serde_json::json!({
                    "query": query,
                    "depth": depth,
                    "limit": limit,
                    "edge_type": edge_type
                }))
                .send()
                .await?
        }
        CodeCommand::CallChain {
            query,
            depth,
            limit,
        } => {
            client
                .post(format!("{}/code/call-chain", base_url))
                .header("X-Xavier-Token", &token)
                .json(&serde_json::json!({
                    "query": query,
                    "depth": depth,
                    "limit": limit
                }))
                .send()
                .await?
        }
        CodeCommand::Hubs => {
            client
                .get(format!("{}/code/hubs", base_url))
                .header("X-Xavier-Token", &token)
                .send()
                .await?
        }
        CodeCommand::Hotspots => {
            client
                .get(format!("{}/code/hotspots", base_url))
                .header("X-Xavier-Token", &token)
                .send()
                .await?
        }
        CodeCommand::Stats => {
            client
                .get(format!("{}/code/stats", base_url))
                .header("X-Xavier-Token", &token)
                .send()
                .await?
        }
    };

    let status = response.status();
    let body: serde_json::Value = response.json().await.unwrap_or_default();
    if status.is_success() {
        println!("{}", serde_json::to_string_pretty(&body)?);
    } else {
        println!("Code graph request failed ({}):", status);
        println!("{}", serde_json::to_string_pretty(&body)?);
    }
    Ok(())
}

pub async fn session_save(session_id: &str, content: &str) -> Result<()> {
    let content = secure_cli_input("session content", content, 10_000_000)?;
    let token = require_xavier_token()?;
    let base_url = resolve_base_url();
    let url = format!("{}/memory/add", base_url);

    let body = serde_json::json!({
        "content": content,
        "path": format!("context/{}/save", session_id),
        "metadata": {
            "session_id": session_id,
            "kind": "session_save",
        }
    });

    let client = CLI_HTTP_CLIENT.clone();

    let response = client
        .post(&url)
        .header("X-Xavier-Token", &token)
        .json(&body)
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                println!("Session context saved successfully!");
                println!("Path: context/{}/save", session_id);
            } else {
                println!("Failed to save session context: {}", resp.status());
            }
        }
        Err(e) => {
            println!("Error connecting to Xavier server: {}", e);
            println!("Configured endpoint: {}", base_url);
            println!("Is the server running? (xavier http)");
        }
    }

    Ok(())
}

pub async fn spawn_agents(
    count: usize,
    providers: Vec<String>,
    models: Vec<String>,
    skills: &[String],
    custom_context: &[String],
    task: Option<&str>,
) -> Result<()> {
    println!("Spawning {} agents...", count);

    let available_providers = if providers.is_empty() {
        vec!["local".to_string()]
    } else {
        providers
    };

    let mut agents = Vec::with_capacity(count);
    for i in 0..count {
        let name = format!("agent-{}", i + 1);
        let provider_name = available_providers
            .get(i % available_providers.len())
            .cloned();
        let model_name = models.get(i % models.len().max(1)).cloned();

        let mut context = HashMap::new();
        context.insert("agent_index".to_string(), i.to_string());
        context.insert("total_agents".to_string(), count.to_string());
        if let Some(ref provider_name) = provider_name {
            context.insert("spawn_provider".to_string(), provider_name.clone());
        }

        for kv in custom_context {
            if let Some((key, value)) = kv.split_once('=') {
                context.insert(key.to_string(), value.to_string());
            }
        }

        let mut effective_skills = skills.to_vec();
        if let Some(ref provider_name) = provider_name {
            let provider_key = provider_name.to_lowercase();
            if provider_key.contains("minimax")
                && !effective_skills.iter().any(|skill| skill == "coding-agent")
            {
                effective_skills.push("coding-agent".to_string());
            }
            if provider_key.contains("deepseek")
                && !effective_skills.iter().any(|skill| skill == "research")
            {
                effective_skills.push("research".to_string());
            }
        }

        let mut loaded_skills = Vec::new();
        for skill_name in &effective_skills {
            if let Some(content) = load_skill(skill_name) {
                context.insert(format!("skill_{}", skill_name), content);
                loaded_skills.push(skill_name.clone());
            } else {
                println!("Warning: skill '{}' not found", skill_name);
            }
        }

        let mut config = AgentConfig::new(name.clone())
            .with_skills(loaded_skills)
            .with_context(context);
        if let Some(ref provider_name) = provider_name {
            config = config.with_provider(provider_name.clone());
        }
        if let Some(ref model_name) = model_name {
            config = config.with_model(model_name.clone());
        }
        if let Some(task) = task {
            config = config.with_task(task.to_string());
        }

        println!(
            "  spawned {} [provider: {}, model: {}]",
            name,
            provider_name.as_deref().unwrap_or("auto"),
            model_name.as_deref().unwrap_or("default"),
        );
        agents.push(Agent::new(config));
    }

    if let Some(task) = task {
        println!("Executing task across spawned agents: {}", task);
        let memory = load_spawn_memory().await?;
        let mut futures = Vec::with_capacity(agents.len());
        for mut agent in agents {
            let memory = Arc::clone(&memory);
            futures.push(tokio::spawn(async move {
                let name = agent.name.clone();
                match agent.run(memory).await {
                    Ok(resp) => println!("  {} completed: {}", name, resp.response),
                    Err(error) => println!("  {} failed: {}", name, error),
                }
            }));
        }

        for future in futures {
            let _ = future.await;
        }
    }

    Ok(())
}

pub async fn multi_spawn_agents(
    agents_count: usize,
    batch_size: usize,
    providers: Vec<String>,
    models: Vec<String>,
    skills: Vec<String>,
    task: Option<&str>,
) -> Result<()> {
    println!(
        "Batch spawning {} agents in groups of {}...",
        agents_count, batch_size
    );

    let providers = if providers.is_empty() {
        vec!["local".to_string()]
    } else {
        providers
    };

    for offset in (0..agents_count).step_by(batch_size.max(1)) {
        let current_batch = std::cmp::min(batch_size.max(1), agents_count - offset);
        let batch_providers = (0..current_batch)
            .map(|i| providers[(offset + i) % providers.len()].clone())
            .collect::<Vec<_>>();
        let batch_models = if models.is_empty() {
            Vec::new()
        } else {
            (0..current_batch)
                .map(|i| models[(offset + i) % models.len()].clone())
                .collect::<Vec<_>>()
        };

        spawn_agents(
            current_batch,
            batch_providers,
            batch_models,
            &skills,
            &[],
            task,
        )
        .await?;
    }

    Ok(())
}

pub async fn run_swarm(config_path: PathBuf, parallel: usize) -> Result<()> {
    println!(
        "Loading swarm configuration from {}...",
        config_path.display()
    );
    let content = std::fs::read_to_string(&config_path)?;
    let swarm: SwarmConfig = if matches!(
        config_path.extension().and_then(|s| s.to_str()),
        Some("yaml" | "yml")
    ) {
        serde_yaml::from_str(&content)?
    } else {
        serde_json::from_str(&content)?
    };

    println!(
        "Launching swarm with {} agents (parallelism: {})...",
        swarm.agents.len(),
        parallel
    );
    let memory = load_spawn_memory().await?;

    let semaphore = Arc::new(tokio::sync::Semaphore::new(parallel));
    let mut futures = Vec::new();

    for agent_cfg in swarm.agents {
        let memory = Arc::clone(&memory);
        let semaphore = Arc::clone(&semaphore);

        futures.push(tokio::spawn(async move {
            let _permit = match semaphore.acquire().await {
                Ok(permit) => permit,
                Err(e) => {
                    tracing::error!("Failed to acquire semaphore: {}", e);
                    return;
                }
            };

            let mut config = AgentConfig::new(agent_cfg.name.clone())
                .with_provider(agent_cfg.provider.clone())
                .with_task(agent_cfg.task.clone());

            if let Some(model) = agent_cfg.model {
                config = config.with_model(model);
            }

            if let Some(skills) = agent_cfg.skills {
                config = config.with_skills(skills);
            }

            if let Some(context) = agent_cfg.context {
                config = config.with_context(context);
            }

            let mut agent = Agent::new(config);
            println!("  starting {}", agent.name);
            match agent.run(memory).await {
                Ok(resp) => println!("  {} finished: {}", agent.name, resp.response),
                Err(error) => println!("  {} failed: {}", agent.name, error),
            }
        }));
    }

    for f in futures {
        let _ = f.await;
    }

    println!("Swarm execution completed.");
    Ok(())
}

pub async fn load_spawn_memory() -> Result<Arc<QmdMemory>> {
    let store = VecSqliteMemoryStore::from_env().await?;
    let workspace_id =
        std::env::var("XAVIER_DEFAULT_WORKSPACE_ID").unwrap_or_else(|_| "default".to_string());
    let durable_state = store.load_workspace_state(&workspace_id).await?;
    let docs = Arc::new(RwLock::new(
        durable_state
            .memories
            .iter()
            .map(MemoryRecord::to_document)
            .collect::<Vec<MemoryDocument>>(),
    ));
    let memory = Arc::new(QmdMemory::new_with_workspace(docs, workspace_id));
    memory.set_store(Arc::new(store)).await;
    memory.init().await?;
    Ok(memory)
}

pub fn load_skill(skill_name: &str) -> Option<String> {
    let paths = [
        format!("skills/{}/SKILL.md", skill_name),
        format!("skills/{}.md", skill_name),
        format!(".agents/skills/{}/SKILL.md", skill_name),
        format!(".agents/skills/{}.md", skill_name),
    ];

    for path in paths {
        if let Ok(content) = std::fs::read_to_string(path) {
            return Some(content);
        }
    }
    None
}
async fn handle_secrets_command(cmd: SecretsCommand) -> Result<()> {
    match cmd {
        SecretsCommand::Lend {
            secret_name,
            agent,
            ttl,
        } => lend_secret(&secret_name, &agent, ttl).await,
        SecretsCommand::ListLeases => list_leases().await,
        SecretsCommand::Revoke { token } => revoke_lease(&token).await,
        SecretsCommand::Status { token } => check_lease_status(&token).await,
    }
}

async fn lend_secret(name: &str, agent: &str, ttl: u64) -> Result<()> {
    let token = xavier_token();
    let url = format!("{}/secrets/lend", resolve_base_url());
    let client = CLI_HTTP_CLIENT.clone();


    let response = client
        .post(&url)
        .header("X-Xavier-Token", &token)
        .json(&serde_json::json!({
            "secret_name": name,
            "agent_id": agent,
            "ttl_seconds": ttl
        }))
        .send()
        .await?;

    if response.status().is_success() {
        let body: serde_json::Value = response.json().await?;
        println!("Secret lent successfully!");
        println!("Lease Token: {}", body["token"]);
        println!("Expires: {}", body["expires_at"]);
    } else {
        println!("Failed to lend secret: {}", response.status());
    }
    Ok(())
}

async fn list_leases() -> Result<()> {
    let token = xavier_token();
    let url = format!("{}/secrets/leases", resolve_base_url());
    let client = CLI_HTTP_CLIENT.clone();


    let response = client
        .get(&url)
        .header("X-Xavier-Token", &token)
        .send()
        .await?;

    if response.status().is_success() {
        let leases: Vec<serde_json::Value> = response.json().await?;
        println!(
            "{:<20} {:<20} {:<20} {:<10}",
            "Agent", "Secret", "Expires", "Status"
        );
        for lease in leases {
            println!(
                "{:<20} {:<20} {:<20} {:<10}",
                lease["agent_id"].as_str().unwrap_or("?"),
                lease["secret_name"].as_str().unwrap_or("?"),
                lease["expires_at"].as_str().unwrap_or("?"),
                if lease["revoked"].as_bool().unwrap_or(false) {
                    "Revoked"
                } else {
                    "Active"
                }
            );
        }
    } else {
        println!("Failed to list leases: {}", response.status());
    }
    Ok(())
}

async fn revoke_lease(token_str: &str) -> Result<()> {
    let token = xavier_token();
    let url = format!("{}/secrets/revoke", resolve_base_url());
    let client = CLI_HTTP_CLIENT.clone();


    let response = client
        .post(&url)
        .header("X-Xavier-Token", &token)
        .json(&serde_json::json!({ "token": token_str }))
        .send()
        .await?;

    if response.status().is_success() {
        println!("Lease revoked successfully.");
    } else {
        println!("Failed to revoke lease: {}", response.status());
    }
    Ok(())
}

async fn check_lease_status(token_str: &str) -> Result<()> {
    let token = xavier_token();
    let url = format!("{}/secrets/status/{}", resolve_base_url(), token_str);
    let client = CLI_HTTP_CLIENT.clone();


    let response = client
        .get(&url)
        .header("X-Xavier-Token", &token)
        .send()
        .await?;

    if response.status().is_success() {
        let status: serde_json::Value = response.json().await?;
        println!(
            "Lease Status: {}",
            if status["revoked"].as_bool().unwrap_or(false) {
                "Revoked"
            } else {
                "Active"
            }
        );
        println!("Agent: {}", status["agent_id"]);
        println!("Expires: {}", status["expires_at"]);
    } else {
        println!("Failed to get lease status: {}", response.status());
    }
    Ok(())
}
