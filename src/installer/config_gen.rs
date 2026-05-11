//! Config file generation from installer state.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::Serialize;

use super::InstallerState;

/// Serializable config matching the `xavier2.config.json` schema.
#[derive(Debug, Clone, Serialize)]
pub struct GeneratedConfig {
    pub server: ServerSection,
    pub workspace: WorkspaceSection,
    pub memory: MemorySection,
    pub models: ModelSection,
    pub retrieval: RetrievalSection,
    pub sync: SyncSection,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServerSection {
    pub host: String,
    pub port: u16,
    pub log_level: String,
    pub code_graph_db_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceSection {
    pub default_workspace_id: String,
    pub default_plan: String,
    pub storage_limit_bytes: Option<u64>,
    pub request_limit: Option<usize>,
    pub request_unit_limit: Option<u64>,
    pub embedding_provider_mode: String,
    pub managed_google_embeddings: bool,
    pub sync_policy: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemorySection {
    pub backend: String,
    pub data_dir: String,
    pub embedding_dimensions: usize,
    pub workspace_dir: String,
    pub file_path: String,
    pub sqlite_path: String,
    pub vec_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelSection {
    pub provider: String,
    pub api_flavor: String,
    pub local_llm_url: String,
    pub local_llm_model: String,
    pub embedding_url: String,
    pub embedding_model: String,
    pub router_retrieved_model: String,
    pub router_complex_model: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RetrievalSection {
    pub disable_hyde: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncSection {
    pub interval_ms: u64,
    pub lag_threshold_ms: u64,
    pub save_ok_rate_threshold: f32,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
}

/// Generate a config JSON from installer state.
pub fn generate_config(state: &InstallerState) -> GeneratedConfig {
    let (embedding_url, embedding_model, provider_label) = match state.embedding_provider {
        super::EmbeddingProvider::Bm25 => {
            (String::new(), "bm25".to_string(), "bm25".to_string())
        }
        super::EmbeddingProvider::Ollama => {
            (state.embedding_url.clone(), state.embedding_model.clone(), "ollama".to_string())
        }
        super::EmbeddingProvider::Gllm => {
            (String::new(), "gllm".to_string(), "gllm".to_string())
        }
        super::EmbeddingProvider::Tract => {
            (String::new(), "tract-onnx".to_string(), "tract-onnx".to_string())
        }
        super::EmbeddingProvider::OpenAI => {
            (state.embedding_url.clone(), state.embedding_model.clone(), "openai".to_string())
        }
    };

    let data_dir = if state.data_dir.is_empty() {
        "data".to_string()
    } else {
        state.data_dir.clone()
    };

    let port: u16 = state.port.parse().unwrap_or(8006);

    GeneratedConfig {
        server: ServerSection {
            host: state.host.clone(),
            port,
            log_level: "info".to_string(),
            code_graph_db_path: format!("{}/code_graph.db", data_dir),
        },
        workspace: WorkspaceSection {
            default_workspace_id: "default".to_string(),
            default_plan: "community".to_string(),
            storage_limit_bytes: None,
            request_limit: None,
            request_unit_limit: None,
            embedding_provider_mode: "bring_your_own".to_string(),
            managed_google_embeddings: false,
            sync_policy: "local_only".to_string(),
        },
        memory: MemorySection {
            backend: "vec".to_string(),
            data_dir: data_dir.clone(),
            embedding_dimensions: 384,
            workspace_dir: format!("{}/workspaces", data_dir),
            file_path: format!("{}/workspaces/default/memory-store.json", data_dir),
            sqlite_path: format!("{}/memory-store.sqlite3", data_dir),
            vec_path: format!("{}/vec-store.sqlite3", data_dir),
        },
        models: ModelSection {
            provider: provider_label,
            api_flavor: "openai-compatible".to_string(),
            local_llm_url: "http://localhost:11434/v1".to_string(),
            local_llm_model: "base-model".to_string(),
            embedding_url,
            embedding_model,
            router_retrieved_model: String::new(),
            router_complex_model: String::new(),
        },
        retrieval: RetrievalSection {
            disable_hyde: true,
        },
        sync: SyncSection {
            interval_ms: 300_000,
            lag_threshold_ms: 30_000,
            save_ok_rate_threshold: 0.95,
            max_retries: 3,
            retry_delay_ms: 1_000,
        },
    }
}

/// Write config to disk. Creates parent directories if needed.
pub fn write_config(state: &mut InstallerState) -> Result<()> {
    let config = generate_config(state);
    let json = serde_json::to_string_pretty(&config)
        .context("failed to serialize config")?;

    let path = PathBuf::from(&state.config_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory {}", parent.display()))?;
    }

    fs::write(&path, &json)
        .with_context(|| format!("failed to write config to {}", path.display()))?;

    state.config_written = true;
    Ok(())
}
