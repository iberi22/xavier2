#![allow(dead_code)]

use serde::{Deserialize, Serialize};

use super::task::RiskLevel;
use super::conflict::ConflictSeverity;

/// Associates a source-tree layer with a risk level and import rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerRule {
    pub name: String,
    pub path: String,
    pub risk: RiskLevel,
    pub may_import: Vec<String>,
}

/// A single validation rule enforced by the change-control system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    pub id: String,
    pub description: String,
    pub severity: ConflictSeverity,
}

/// Central configuration for change-control validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeControlConfig {
    pub layers: Vec<LayerRule>,
    pub critical_files: Vec<String>,
    pub rules: Vec<ValidationRule>,
}

impl ChangeControlConfig {
    /// Load a `ChangeControlConfig` from a YAML file at `path`.
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read policy file: {}", e))?;
        let config: Self = serde_yaml::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse policy YAML: {}", e))?;
        Ok(config)
    }

    /// Return the default path for the change-control policy file.
    pub fn default_path() -> std::path::PathBuf {
        std::path::PathBuf::from(".gitcore/change-control.yaml")
    }

    /// Check if a file path is in the critical files list.
    pub fn is_critical(&self, file_path: &str) -> bool {
        self.critical_files
            .iter()
            .any(|pattern| glob_match(pattern, file_path))
    }

    /// Get the risk level for a given file path based on layer rules.
    /// Returns `RiskLevel::Low` for unlisted paths.
    pub fn risk_for(&self, file_path: &str) -> RiskLevel {
        for layer in &self.layers {
            if glob_match(&layer.path, file_path) {
                return layer.risk;
            }
        }
        RiskLevel::Low // default for unlisted paths
    }

    /// Check if importing from `from_path` to `to_layer` violates any rules.
    /// Returns the first matching `ValidationRule` on violation, or `None`.
    pub fn check_import(&self, from_path: &str, to_layer: &str) -> Option<&ValidationRule> {
        // Find which layer from_path belongs to
        let from_layer = self
            .layers
            .iter()
            .find(|l| glob_match(&l.path, from_path));

        if let Some(layer) = from_layer {
            if !layer.may_import.iter().any(|allowed| allowed == to_layer) {
                return self
                    .rules
                    .iter()
                    .find(|r| r.id == "domain_no_adapter_imports");
            }
        }
        None
    }
}

/// Simple glob matching (supports `**` and `*` patterns).
///
/// - `**` matches any number of path segments.
/// - `*` matches everything except `/` (single segment).
fn glob_match(pattern: &str, path: &str) -> bool {
    if pattern == "**" {
        return true;
    }

    // Bail fast if pattern has no wildcards.
    if !pattern.contains('*') {
        return path == pattern;
    }

    // Split on "**" — there should be at most one relevant `**`.
    if let Some((left, right)) = pattern.split_once("**") {
        // `**` at start → check suffix
        if left.is_empty() {
            // pattern is "**/rest" or "**rest"
            let trimmed_right = right.trim_start_matches('/');
            // If right is empty, pattern is just "**" or "**/"
            if trimmed_right.is_empty() || right.is_empty() {
                return true;
            }
            return path.ends_with(trimmed_right) || glob_match(trimmed_right, path);
        }
        // `**` at end → check prefix
        let left_trimmed = left.trim_end_matches('/');
        // If right is empty, pattern is "prefix/**" or "prefix/**"
        if right.is_empty() || right.trim_start_matches('/').is_empty() {
            return path.starts_with(left_trimmed);
        }
        // `**` in middle: path must start with left and end with right
        let right_trimmed = right.trim_start_matches('/');
        path.starts_with(left_trimmed) && path.ends_with(right_trimmed)
    } else {
        // Single `*` — match within one path segment (no `/`)
        let parts: Vec<&str> = pattern.split('*').collect();
        // Requires prefix to match, then suffix to match within the same segment
        // This handles: "src/*/mod.rs" — any single dir level
        match parts.len() {
            1 => path == pattern,
            2 => {
                let prefix = parts[0];
                let suffix = parts[1];
                path.starts_with(prefix) && path.ends_with(suffix)
                    && path[prefix.len()..path.len() - suffix.len()]
                        .find('/')
                        .is_none()
            },
            _ => {
                // Fallback for multiple `*` in one segment: just check prefix + suffix
                let prefix = parts[0];
                let suffix = parts[parts.len() - 1];
                path.starts_with(prefix) && path.ends_with(suffix)
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_critical() {
        let config = ChangeControlConfig {
            layers: vec![],
            critical_files: vec![
                "src/memory/qmd_memory.rs".into(),
                ".gitcore/ARCHITECTURE.md".into(),
            ],
            rules: vec![],
        };
        assert!(config.is_critical("src/memory/qmd_memory.rs"));
        assert!(config.is_critical(".gitcore/ARCHITECTURE.md"));
        assert!(!config.is_critical("src/memory/mod.rs"));
    }

    #[test]
    fn test_risk_for_high() {
        let config = ChangeControlConfig {
            layers: vec![LayerRule {
                name: "domain".into(),
                path: "src/domain/**".into(),
                risk: RiskLevel::High,
                may_import: vec![],
            }],
            critical_files: vec![],
            rules: vec![],
        };
        assert_eq!(config.risk_for("src/domain/memory/mod.rs"), RiskLevel::High);
    }

    #[test]
    fn test_risk_for_default_low() {
        let config = ChangeControlConfig {
            layers: vec![],
            critical_files: vec![],
            rules: vec![],
        };
        assert_eq!(
            config.risk_for("src/misc/unlisted.rs"),
            RiskLevel::Low
        );
    }

    #[test]
    fn test_load_from_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test-policy.yaml");
        std::fs::write(
            &path,
            r#"
layers:
  - name: domain
    path: "src/domain/**"
    risk: high
    may_import: []
critical_files:
  - "src/memory/qmd_memory.rs"
rules:
  - id: domain_no_adapter_imports
    description: "Domain cannot import from adapters"
    severity: warning
"#,
        )
        .unwrap();

        let config = ChangeControlConfig::load(&path).unwrap();
        assert_eq!(config.layers.len(), 1);
        assert_eq!(config.layers[0].name, "domain");
        assert_eq!(config.layers[0].risk, RiskLevel::High);
        assert!(config.is_critical("src/memory/qmd_memory.rs"));
    }

    #[test]
    fn test_glob_match_double_star() {
        assert!(glob_match("src/domain/**", "src/domain/memory/mod.rs"));
        assert!(glob_match("src/domain/**", "src/domain/"));
        assert!(!glob_match("src/domain/**", "src/app/mod.rs"));
    }

    #[test]
    fn test_glob_match_single_star() {
        assert!(glob_match("src/*/mod.rs", "src/domain/mod.rs"));
        assert!(glob_match("src/*/mod.rs", "src/app/mod.rs"));
        assert!(!glob_match("src/*/mod.rs", "src/domain/types.rs"));
    }

    #[test]
    fn test_glob_match_exact() {
        assert!(glob_match("src/main.rs", "src/main.rs"));
        assert!(!glob_match("src/main.rs", "src/lib.rs"));
    }

    #[test]
    fn test_check_import_violation() {
        let config = ChangeControlConfig {
            layers: vec![LayerRule {
                name: "domain".into(),
                path: "src/domain/**".into(),
                risk: RiskLevel::High,
                may_import: vec![],
            }],
            critical_files: vec![],
            rules: vec![ValidationRule {
                id: "domain_no_adapter_imports".into(),
                description: "Domain cannot import from adapters".into(),
                severity: ConflictSeverity::Warning,
            }],
        };
        let rule = config.check_import("src/domain/foo.rs", "adapters");
        assert!(rule.is_some());
        assert_eq!(rule.unwrap().id, "domain_no_adapter_imports");
    }

    #[test]
    fn test_check_import_allowed() {
        let config = ChangeControlConfig {
            layers: vec![LayerRule {
                name: "domain".into(),
                path: "src/domain/**".into(),
                risk: RiskLevel::High,
                may_import: vec!["ports".into()],
            }],
            critical_files: vec![],
            rules: vec![],
        };
        assert!(config.check_import("src/domain/foo.rs", "ports").is_none());
    }

    #[test]
    fn test_check_import_no_layer_match() {
        let config = ChangeControlConfig {
            layers: vec![],
            critical_files: vec![],
            rules: vec![],
        };
        assert!(
            config.check_import("src/unknown.rs", "ports").is_none()
        );
    }

    #[test]
    fn test_default_path_ends_correctly() {
        let p = ChangeControlConfig::default_path();
        assert!(p.ends_with(".gitcore/change-control.yaml"));
    }
}
