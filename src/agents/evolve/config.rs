//! Evolve Module Configuration

use serde::{Deserialize, Serialize};

/// Benchmark type for evaluation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BenchmarkType {
    /// LoCoMo: Long-Term Conversation Memory benchmark
    #[default]
    Locomo,
    /// Evo-Memory: Self-Evolving Memory benchmark
    EvoMemory,
    /// Custom benchmark
    Custom,
}

/// Evolve Module configuration
#[derive(Debug, Clone)]
pub struct EvolveConfig {
    /// Tag for this run (e.g., "mar24")
    pub tag: String,

    /// Time budget per experiment in seconds
    pub time_budget_secs: u64,

    /// Metric to optimize
    pub metric: MetricType,

    /// Benchmark to use for evaluation
    pub benchmark: BenchmarkType,

    /// Whether to run in autonomous mode (never stop)
    pub autonomous: bool,

    /// Results file path
    pub results_file: String,

    /// Module paths that can be modified
    pub editable_modules: Vec<String>,

    /// Memory modules path
    pub memory_modules_path: String,
}

impl EvolveConfig {
    pub fn new(tag: String) -> Self {
        let results_file = format!("results/{}.tsv", tag);
        Self {
            tag,
            time_budget_secs: 300, // 5 minutes default
            metric: MetricType::LocomoF1,
            benchmark: BenchmarkType::Locomo,
            autonomous: true,
            results_file,
            editable_modules: vec![
                "src/memory/".to_string(),
                "src/agents/system1.rs".to_string(),
                "src/agents/system2.rs".to_string(),
                "src/agents/system3/".to_string(),
            ],
            memory_modules_path: "src/memory/".to_string(),
        }
    }

    /// Results file path
    pub fn results_path(&self) -> std::path::PathBuf {
        std::path::PathBuf::from(&self.results_file)
    }
}

/// Metric types for optimization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MetricType {
    /// LoCoMo F1 score (higher is better)
    #[default]
    LocomoF1,
    /// Bits per byte (lower is better)
    ValBpb,
    /// Custom metric
    Custom,
}

impl std::fmt::Display for MetricType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetricType::LocomoF1 => write!(f, "locomo_f1"),
            MetricType::ValBpb => write!(f, "val_bpb"),
            MetricType::Custom => write!(f, "custom"),
        }
    }
}
