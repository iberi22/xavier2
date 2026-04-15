//! Experiment Results - Logging and tracking

use crate::agents::evolve::experiment::ExperimentStatus;
use serde::{Deserialize, Serialize};

/// Result of a single experiment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentResult {
    /// Hypothesis ID
    pub hypothesis_id: String,
    /// Metric value achieved
    pub metric_value: f32,
    /// Experiment status
    pub status: ExperimentStatus,
    /// Git commit hash (if kept)
    pub commit_hash: Option<String>,
    /// Whether experiment crashed
    pub crashed: bool,
}

impl ExperimentResult {
    /// Create a baseline result (no changes)
    pub fn baseline() -> Self {
        Self {
            hypothesis_id: "baseline".to_string(),
            metric_value: 0.0,
            status: ExperimentStatus::Completed,
            commit_hash: None,
            crashed: false,
        }
    }
}

/// Results log entry (tab-separated for easy parsing)
#[derive(Debug, Clone)]
pub struct ResultsLogEntry {
    pub commit: String,
    pub metric: f32,
    pub memory_gb: f32,
    pub status: String,
    pub description: String,
}

impl ResultsLogEntry {
    /// Convert to TSV line
    pub fn to_tsv(&self) -> String {
        format!(
            "{}\t{:.6}\t{:.1}\t{}\t{}",
            self.commit, self.metric, self.memory_gb, self.status, self.description
        )
    }

    /// Parse from TSV line
    pub fn from_tsv(line: &str) -> Option<Self> {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() != 5 {
            return None;
        }
        Some(Self {
            commit: parts[0].to_string(),
            metric: parts[1].parse().ok()?,
            memory_gb: parts[2].parse().ok()?,
            status: parts[3].to_string(),
            description: parts[4].to_string(),
        })
    }
}

/// Header for results TSV
pub const RESULTS_HEADER: &str = "commit\tmetric\tmemory_gb\tstatus\tdescription";
