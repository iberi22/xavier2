use std::collections::HashMap;
use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};
use glob::Pattern;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ChangeControlError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("Invalid pattern: {0}")]
    Pattern(#[from] glob::PatternError),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChangePolicy {
    pub layers: HashMap<String, LayerRule>,
    pub critical_files: Vec<String>,
    pub rules: Vec<ValidationRule>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LayerRule {
    pub path: String,
    pub risk: RiskLevel,
    pub may_import: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    High,
    Medium,
    Low,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ValidationRule {
    pub id: String,
    pub description: String,
    pub severity: Severity,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Block,
    Warn,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationStatus {
    Allowed,
    Warn(String),
    Rejected(String),
}

impl ChangePolicy {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, ChangeControlError> {
        let content = fs::read_to_string(path)?;
        let policy: ChangePolicy = serde_yaml::from_str(&content)?;
        Ok(policy)
    }

    pub fn is_critical(&self, path: &str) -> bool {
        for pattern_str in &self.critical_files {
            if let Ok(pattern) = Pattern::new(pattern_str) {
                if pattern.matches(path) {
                    return true;
                }
            }
        }
        false
    }

    pub fn get_layer_for_path(&self, path: &str) -> Option<(String, &LayerRule)> {
        for (name, rule) in &self.layers {
            if let Ok(pattern) = Pattern::new(&rule.path) {
                if pattern.matches(path) {
                    return Some((name.clone(), rule));
                }
            }
        }
        None
    }

    pub fn validate_import(&self, from_path: &str, to_path: &str) -> ValidationStatus {
        let from_layer = match self.get_layer_for_path(from_path) {
            Some((name, rule)) => (name, rule),
            None => return ValidationStatus::Allowed,
        };

        let to_layer = match self.get_layer_for_path(to_path) {
            Some((name, _)) => name,
            None => return ValidationStatus::Allowed,
        };

        if from_layer.0 == to_layer {
            return ValidationStatus::Allowed;
        }

        if from_layer.1.may_import.contains(&to_layer) {
            ValidationStatus::Allowed
        } else {
            ValidationStatus::Rejected(format!(
                "Layer '{}' is not allowed to import from layer '{}'",
                from_layer.0, to_layer
            ))
        }
    }
}
