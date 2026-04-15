//! Experiment - Represents a single experiment in the evolution loop

use serde::{Deserialize, Serialize};

/// Experiment status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExperimentStatus {
    /// Experiment is still running
    Running,
    /// Experiment completed successfully
    Completed,
    /// Experiment crashed
    Crashed,
    /// Experiment timed out
    Timeout,
    /// Improvement was kept
    Kept,
    /// No improvement - discarded
    Discarded,
}

/// A single experiment hypothesis
#[derive(Debug, Clone)]
pub struct Hypothesis {
    pub id: String,
    pub description: String,
    pub hypothesis_type: HypothesisType,
    /// Files to modify
    pub files: Vec<String>,
    /// Code changes (patch format)
    pub patch: String,
    /// Complexity cost (lines added)
    pub complexity_cost: usize,
}

impl Hypothesis {
    pub fn new(description: String, hypothesis_type: HypothesisType) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string()[..8].to_string(),
            description,
            hypothesis_type,
            files: Vec::new(),
            patch: String::new(),
            complexity_cost: 0,
        }
    }

    /// Create a simplicity-based hypothesis (remove code)
    pub fn simplification(description: String, files: Vec<String>, lines_removed: usize) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string()[..8].to_string(),
            description,
            hypothesis_type: HypothesisType::Simplification,
            files,
            patch: String::new(),
            complexity_cost: lines_removed,
        }
    }

    /// Create an optimization hypothesis
    pub fn optimization(
        description: String,
        files: Vec<String>,
        patch: String,
        lines_added: usize,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string()[..8].to_string(),
            description,
            hypothesis_type: HypothesisType::Optimization,
            files,
            patch,
            complexity_cost: lines_added,
        }
    }

    /// Create a hyperparameter tuning hypothesis
    pub fn hyperparameter(
        description: String,
        files: Vec<String>,
        patch: String,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string()[..8].to_string(),
            description,
            hypothesis_type: HypothesisType::Hyperparameter,
            files,
            patch,
            complexity_cost: 0,
        }
    }

    /// Create an architecture change hypothesis
    pub fn architecture(
        description: String,
        files: Vec<String>,
        patch: String,
        lines_added: usize,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string()[..8].to_string(),
            description,
            hypothesis_type: HypothesisType::Architecture,
            files,
            patch,
            complexity_cost: lines_added,
        }
    }
}

/// Types of hypotheses
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HypothesisType {
    /// Architecture change
    Architecture,
    /// Algorithm optimization
    Optimization,
    /// Hyperparameter tuning
    Hyperparameter,
    /// Removing code (simplicity)
    Simplification,
    /// New feature addition
    Feature,
}

impl std::fmt::Display for HypothesisType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HypothesisType::Architecture => write!(f, "architecture"),
            HypothesisType::Optimization => write!(f, "optimization"),
            HypothesisType::Hyperparameter => write!(f, "hyperparameter"),
            HypothesisType::Simplification => write!(f, "simplification"),
            HypothesisType::Feature => write!(f, "feature"),
        }
    }
}
