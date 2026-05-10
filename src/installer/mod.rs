//! Xavier2 Installer — TUI Onboarding Wizard
//!
//! Multi-step guided setup that generates `config/xavier2.config.json`.
//! Steps: Welcome → Token → Server → Storage → Embeddings → Review → Write.

pub mod config_gen;
#[cfg(feature = "cli-interactive")]
pub mod wizard;
#[cfg(test)]
mod wizard_test;

use serde::{Deserialize, Serialize};

/// Linear wizard steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WizardStep {
    Welcome,
    TokenSetup,
    ServerConfig,
    StorageConfig,
    EmbeddingsConfig,
    Review,
    Done,
}

impl WizardStep {
    pub fn title(&self) -> &'static str {
        match self {
            Self::Welcome => " Welcome to Xavier2",
            Self::TokenSetup => " Authentication Token",
            Self::ServerConfig => " Server Configuration",
            Self::StorageConfig => " Storage Configuration",
            Self::EmbeddingsConfig => " Embeddings Provider",
            Self::Review => " Review & Save",
            Self::Done => " Done!",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::Welcome => Self::TokenSetup,
            Self::TokenSetup => Self::ServerConfig,
            Self::ServerConfig => Self::StorageConfig,
            Self::StorageConfig => Self::EmbeddingsConfig,
            Self::EmbeddingsConfig => Self::Review,
            Self::Review => Self::Done,
            Self::Done => Self::Done,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            Self::Welcome => Self::Welcome,
            Self::TokenSetup => Self::Welcome,
            Self::ServerConfig => Self::TokenSetup,
            Self::StorageConfig => Self::ServerConfig,
            Self::EmbeddingsConfig => Self::StorageConfig,
            Self::Review => Self::EmbeddingsConfig,
            Self::Done => Self::Review,
        }
    }
}

/// Embedding provider options and their metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmbeddingProvider {
    /// BM25 keyword search only (no vector embeddings needed)
    Bm25,
    /// Ollama local server (openai-compatible endpoint)
    Ollama,
    /// gllm Rust-native embeddings (GPU capable)
    Gllm,
    /// Tract ONNX pure Rust inference (CPU, universal)
    Tract,
    /// OpenAI / compatible API (cloud)
    OpenAI,
}

impl EmbeddingProvider {
    pub const ALL: &[Self] = &[
        Self::Bm25,
        Self::Ollama,
        Self::Gllm,
        Self::Tract,
        Self::OpenAI,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::Bm25 => "BM25 (keyword-only, no GPU needed)",
            Self::Ollama => "Ollama (local, openai-compatible)",
            Self::Gllm => "gllm (Rust-native, GPU-capable)",
            Self::Tract => "tract-ONNX (pure Rust CPU inference)",
            Self::OpenAI => "OpenAI / Compatible API (cloud)",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Bm25 => "Fast keyword search. No embeddings model needed. Good for small collections.",
            Self::Ollama => "Runs locally via Ollama. Use embeddinggemma, nomic-embed-text, etc.",
            Self::Gllm => "Rust-native inference via gllm crate. GPU acceleration with CUDA/Metal.",
            Self::Tract => "Pure Rust ONNX inference. Universal CPU support. ~600ms per encode.",
            Self::OpenAI => "Cloud API (OpenAI, MiniMax, etc). Fastest, requires internet + API key.",
        }
    }
}

impl std::fmt::Display for EmbeddingProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Mutable state for the entire installer wizard.
#[derive(Debug, Clone)]
pub struct InstallerState {
    pub current_step: WizardStep,
    /// Auth token (dev-token for local use)
    pub token: String,
    /// Server host binding
    pub host: String,
    /// Server port
    pub port: String,
    /// Data directory for storage
    pub data_dir: String,
    /// Selected embedding provider
    pub embedding_provider: EmbeddingProvider,
    /// Selected provider index in the list
    pub provider_index: usize,
    /// Embedding model name (for ollama/openai)
    pub embedding_model: String,
    /// Embedding API URL (for ollama/openai)
    pub embedding_url: String,
    /// OpenAI API key (only for OpenAI provider)
    pub api_key: String,
    /// Input buffer for text fields
    pub input_buffer: String,
    /// Cursor position in input buffer
    pub cursor_pos: usize,
    /// Currently focused field index within a step
    pub focused_field: usize,
    /// Error message to display
    pub error_message: Option<String>,
    /// Whether config was successfully written
    pub config_written: bool,
    /// Output path for config
    pub config_path: String,
}

impl Default for InstallerState {
    fn default() -> Self {
        Self {
            current_step: WizardStep::Welcome,
            token: "xavier-dev-token".to_string(),
            host: "127.0.0.1".to_string(),
            port: "8006".to_string(),
            data_dir: "data".to_string(),
            embedding_provider: EmbeddingProvider::Bm25,
            provider_index: 0,
            embedding_model: "embeddinggemma".to_string(),
            embedding_url: "http://localhost:11434/v1".to_string(),
            api_key: String::new(),
            input_buffer: String::new(),
            cursor_pos: 0,
            focused_field: 0,
            error_message: None,
            config_written: false,
            config_path: "config/xavier2.config.json".to_string(),
        }
    }
}

impl InstallerState {
    /// Number of input fields in the current step.
    pub fn field_count(&self) -> usize {
        match self.current_step {
            WizardStep::Welcome => 0,
            WizardStep::TokenSetup => 1,
            WizardStep::ServerConfig => 2,
            WizardStep::StorageConfig => 1,
            WizardStep::EmbeddingsConfig => {
                // provider selector + conditional fields
                let base = 1; // provider list
                match self.embedding_provider {
                    EmbeddingProvider::Bm25 | EmbeddingProvider::Gllm | EmbeddingProvider::Tract => base,
                    EmbeddingProvider::Ollama => base + 2, // model + url
                    EmbeddingProvider::OpenAI => base + 3, // model + url + api key
                }
            }
            WizardStep::Review => 0,
            WizardStep::Done => 0,
        }
    }

    /// Get the value for a field by index (used to pre-fill input buffer).
    pub fn field_value(&self, idx: usize) -> &str {
        match self.current_step {
            WizardStep::TokenSetup => match idx {
                0 => &self.token,
                _ => "",
            },
            WizardStep::ServerConfig => match idx {
                0 => &self.host,
                1 => &self.port,
                _ => "",
            },
            WizardStep::StorageConfig => match idx {
                0 => &self.data_dir,
                _ => "",
            },
            WizardStep::EmbeddingsConfig => match idx {
                0 => "", // provider list, no text value
                1 => &self.embedding_model,
                2 => &self.embedding_url,
                3 => &self.api_key,
                _ => "",
            },
            _ => "",
        }
    }

    /// Set a field value by index.
    pub fn set_field(&mut self, idx: usize, value: &str) {
        match self.current_step {
            WizardStep::TokenSetup if idx == 0 => self.token = value.to_string(),
            WizardStep::ServerConfig => match idx {
                0 => self.host = value.to_string(),
                1 => self.port = value.to_string(),
                _ => {}
            },
            WizardStep::StorageConfig if idx == 0 => self.data_dir = value.to_string(),
            WizardStep::EmbeddingsConfig => match idx {
                1 => self.embedding_model = value.to_string(),
                2 => self.embedding_url = value.to_string(),
                3 => self.api_key = value.to_string(),
                _ => {}
            },
            _ => {}
        }
    }

    /// Field label for a given index in the current step.
    pub fn field_label(&self, idx: usize) -> &'static str {
        match self.current_step {
            WizardStep::TokenSetup => match idx {
                0 => "Dev Token",
                _ => "",
            },
            WizardStep::ServerConfig => match idx {
                0 => "Host",
                1 => "Port",
                _ => "",
            },
            WizardStep::StorageConfig => match idx {
                0 => "Data Directory",
                _ => "",
            },
            WizardStep::EmbeddingsConfig => match idx {
                0 => "Provider",
                1 => "Model Name",
                2 => "API URL",
                3 => "API Key",
                _ => "",
            },
            _ => "",
        }
    }

    /// Whether a field should show masked input (password style).
    pub fn field_masked(&self, idx: usize) -> bool {
        matches!(
            (self.current_step, idx),
            (WizardStep::TokenSetup, 0) | (WizardStep::EmbeddingsConfig, 3)
        )
    }
}
