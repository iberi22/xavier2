use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

const DEFAULT_CONFIG_PATH: &str = "config/xavier2.config.json";

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Xavier2Settings {
    #[serde(default)]
    pub server: ServerSettings,
    #[serde(default)]
    pub workspace: WorkspaceSettings,
    #[serde(default)]
    pub memory: MemorySettings,
    #[serde(default)]
    pub models: ModelSettings,
    #[serde(default)]
    pub retrieval: RetrievalSettings,
    #[serde(default)]
    pub sync: SyncSettings,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerSettings {
    pub host: String,
    pub port: u16,
    pub log_level: String,
    pub code_graph_db_path: String,
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8006,
            log_level: "info".to_string(),
            code_graph_db_path: "data/code_graph.db".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkspaceSettings {
    pub default_workspace_id: String,
    pub default_plan: String,
    pub storage_limit_bytes: Option<u64>,
    pub request_limit: Option<usize>,
    pub request_unit_limit: Option<u64>,
    pub embedding_provider_mode: String,
    pub managed_google_embeddings: bool,
    pub sync_policy: String,
}

impl Default for WorkspaceSettings {
    fn default() -> Self {
        Self {
            default_workspace_id: "default".to_string(),
            default_plan: "community".to_string(),
            storage_limit_bytes: None,
            request_limit: None,
            request_unit_limit: None,
            embedding_provider_mode: "bring_your_own".to_string(),
            managed_google_embeddings: false,
            sync_policy: "local_only".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct MemorySettings {
    pub backend: String,
    pub data_dir: String,
    pub embedding_dimensions: usize,
    pub workspace_dir: String,
    pub file_path: String,
    pub sqlite_path: String,
    pub vec_path: String,
}

impl Default for MemorySettings {
    fn default() -> Self {
        Self {
            backend: "vec".to_string(),
            data_dir: "data".to_string(),
            embedding_dimensions: 768,
            workspace_dir: "data/workspaces".to_string(),
            file_path: "data/workspaces/default/memory-store.json".to_string(),
            sqlite_path: "data/memory-store.sqlite3".to_string(),
            vec_path: "data/vec-store.sqlite3".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModelSettings {
    pub provider: String,
    pub api_flavor: String,
    pub local_llm_url: String,
    pub local_llm_model: String,
    pub embedding_url: String,
    pub embedding_model: String,
    pub router_retrieved_model: String,
    pub router_complex_model: String,
}

impl Default for ModelSettings {
    fn default() -> Self {
        Self {
            provider: "local".to_string(),
            api_flavor: "openai-compatible".to_string(),
            local_llm_url: "http://localhost:11434/v1".to_string(),
            local_llm_model: "qwen3-coder".to_string(),
            embedding_url: "http://localhost:11434/v1".to_string(),
            embedding_model: "embeddinggemma".to_string(),
            router_retrieved_model: String::new(),
            router_complex_model: String::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RetrievalSettings {
    pub disable_hyde: bool,
}

impl Default for RetrievalSettings {
    fn default() -> Self {
        Self { disable_hyde: true }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SyncSettings {
    pub interval_ms: u64,
    pub lag_threshold_ms: u64,
    pub save_ok_rate_threshold: f32,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
}

impl Default for SyncSettings {
    fn default() -> Self {
        Self {
            interval_ms: 300_000,
            lag_threshold_ms: 30_000,
            save_ok_rate_threshold: 0.95,
            max_retries: 3,
            retry_delay_ms: 1_000,
        }
    }
}

impl Xavier2Settings {
    pub fn load() -> Result<Option<Self>> {
        let path = std::env::var("XAVIER2_CONFIG_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(DEFAULT_CONFIG_PATH));

        if !path.exists() {
            return Ok(None);
        }

        let raw = fs::read_to_string(&path)
            .with_context(|| format!("failed to read config file at {}", path.display()))?;
        let parsed = serde_json::from_str::<Self>(&raw)
            .with_context(|| format!("failed to parse config file at {}", path.display()))?;
        Ok(Some(parsed))
    }

    pub fn apply_to_env(&self) {
        set_if_absent("XAVIER2_HOST", &self.server.host);
        set_if_absent("XAVIER2_PORT", &self.server.port.to_string());
        set_if_absent("XAVIER2_LOG_LEVEL", &self.server.log_level);
        set_if_absent(
            "XAVIER2_CODE_GRAPH_DB_PATH",
            &self.server.code_graph_db_path,
        );

        set_if_absent(
            "XAVIER2_DEFAULT_WORKSPACE_ID",
            &self.workspace.default_workspace_id,
        );
        set_if_absent("XAVIER2_DEFAULT_PLAN", &self.workspace.default_plan);
        set_optional_if_absent(
            "XAVIER2_STORAGE_LIMIT_BYTES",
            self.workspace.storage_limit_bytes.map(|v| v.to_string()),
        );
        set_optional_if_absent(
            "XAVIER2_REQUEST_LIMIT",
            self.workspace.request_limit.map(|v| v.to_string()),
        );
        set_optional_if_absent(
            "XAVIER2_REQUEST_UNIT_LIMIT",
            self.workspace.request_unit_limit.map(|v| v.to_string()),
        );
        set_if_absent(
            "XAVIER2_EMBEDDING_PROVIDER_MODE",
            &self.workspace.embedding_provider_mode,
        );
        set_if_absent(
            "XAVIER2_MANAGED_GOOGLE_EMBEDDINGS",
            if self.workspace.managed_google_embeddings {
                "1"
            } else {
                "0"
            },
        );
        set_if_absent("XAVIER2_SYNC_POLICY", &self.workspace.sync_policy);

        set_if_absent("XAVIER2_MEMORY_BACKEND", &self.memory.backend);
        set_if_absent("XAVIER2_DATA_DIR", &self.memory.data_dir);
        set_if_absent(
            "XAVIER2_EMBEDDING_DIMENSIONS",
            &self.memory.embedding_dimensions.to_string(),
        );
        set_if_absent("XAVIER2_WORKSPACE_DIR", &self.memory.workspace_dir);
        set_if_absent("XAVIER2_MEMORY_FILE_PATH", &self.memory.file_path);
        set_if_absent("XAVIER2_MEMORY_SQLITE_PATH", &self.memory.sqlite_path);
        set_if_absent("XAVIER2_MEMORY_VEC_PATH", &self.memory.vec_path);

        set_if_absent("XAVIER2_MODEL_PROVIDER", &self.models.provider);
        set_if_absent("XAVIER2_API_FLAVOR", &self.models.api_flavor);
        set_if_absent("XAVIER2_LOCAL_LLM_URL", &self.models.local_llm_url);
        set_if_absent("XAVIER2_LOCAL_LLM_MODEL", &self.models.local_llm_model);
        set_if_absent("XAVIER2_EMBEDDING_URL", &self.models.embedding_url);
        set_if_absent("XAVIER2_EMBEDDING_MODEL", &self.models.embedding_model);
        set_optional_if_absent(
            "XAVIER2_ROUTER_RETRIEVED_MODEL",
            non_empty(&self.models.router_retrieved_model),
        );
        set_optional_if_absent(
            "XAVIER2_ROUTER_COMPLEX_MODEL",
            non_empty(&self.models.router_complex_model),
        );

        set_if_absent(
            "XAVIER2_DISABLE_HYDE",
            if self.retrieval.disable_hyde {
                "1"
            } else {
                "0"
            },
        );

        set_if_absent(
            "XAVIER2_SYNC_INTERVAL_MS",
            &self.sync.interval_ms.to_string(),
        );
        set_if_absent(
            "XAVIER2_SYNC_LAG_THRESHOLD_MS",
            &self.sync.lag_threshold_ms.to_string(),
        );
        set_if_absent(
            "XAVIER2_SYNC_SAVE_OK_RATE_THRESHOLD",
            &self.sync.save_ok_rate_threshold.to_string(),
        );
        set_if_absent(
            "XAVIER2_SYNC_MAX_RETRIES",
            &self.sync.max_retries.to_string(),
        );
        set_if_absent(
            "XAVIER2_SYNC_RETRY_DELAY_MS",
            &self.sync.retry_delay_ms.to_string(),
        );
    }

    pub fn current() -> Self {
        Self::load().ok().flatten().unwrap_or_default()
    }

    pub fn client_base_url(&self) -> String {
        let host = match self.server.host.as_str() {
            "0.0.0.0" | "::" => "127.0.0.1",
            other => other,
        };
        format!("http://{}:{}", host, self.server.port)
    }
}

fn set_if_absent(key: &str, value: &str) {
    if std::env::var_os(key).is_none() {
        std::env::set_var(key, value);
    }
}

fn set_optional_if_absent(key: &str, value: Option<String>) {
    if let Some(value) = value {
        set_if_absent(key, &value);
    }
}

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
