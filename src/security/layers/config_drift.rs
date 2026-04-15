//! Config drift detection - snapshot runtime config and detect changes

use parking_lot::RwLock;
use std::collections::HashMap;

use crate::security::detections::{ScanResult, Severity, Threat, ThreatCategory};

/// Configuration snapshot
#[derive(Debug, Clone)]
pub struct ConfigSnapshot {
    pub timestamp: i64,
    pub values: HashMap<String, String>,
}

/// Configuration store - lazily initialized
static CONFIG_STORE: std::sync::LazyLock<RwLock<HashMap<String, ConfigSnapshot>>> =
    std::sync::LazyLock::new(|| RwLock::new(HashMap::new()));

/// Last known config state - lazily initialized
static LAST_CONFIG: std::sync::LazyLock<RwLock<Option<ConfigSnapshot>>> =
    std::sync::LazyLock::new(|| RwLock::new(None));

/// Snapshot the current configuration
pub fn snapshot_config(context: &str) -> ConfigSnapshot {
    let timestamp = chrono::Utc::now().timestamp();
    let mut values = HashMap::new();

    // Snapshot environment variables (sanitized)
    for (key, value) in std::env::vars() {
        // Skip sensitive values
        if key.contains("KEY")
            || key.contains("SECRET")
            || key.contains("TOKEN")
            || key.contains("PASSWORD")
        {
            values.insert(key, "***REDACTED***".to_string());
        } else {
            values.insert(key, value);
        }
    }

    let snapshot = ConfigSnapshot { timestamp, values };

    // Store with context key
    let mut store = CONFIG_STORE.write();
    store.insert(context.to_string(), snapshot.clone());

    // Update last config
    let mut last = LAST_CONFIG.write();
    *last = Some(snapshot.clone());

    snapshot.clone()
}

/// Get the last config snapshot
pub fn get_last_snapshot() -> Option<ConfigSnapshot> {
    LAST_CONFIG.read().clone()
}

/// Detect configuration drift between turns
pub fn detect_config_drift(current: &str, result: &mut ScanResult) {
    // Get the last snapshot
    let last = match get_last_snapshot() {
        Some(s) => s,
        None => {
            // No previous snapshot, just record current
            snapshot_config(current);
            return;
        }
    };

    // Check for environment variable changes
    for (key, value) in std::env::vars() {
        if key.contains("KEY")
            || key.contains("SECRET")
            || key.contains("TOKEN")
            || key.contains("PASSWORD")
        {
            continue; // Skip sensitive
        }

        if let Some(last_value) = last.values.get(&key) {
            if last_value != &value {
                result.add_layer("config_drift");
                result.clean = false;
                result.threats.push(Threat::new(
                    Severity::Warning,
                    "config_drift",
                    ThreatCategory::ConfigTampering,
                    &format!("Environment variable changed: {}", key),
                    &format!("{} -> {}", last_value, value),
                    "env_diff",
                ));
            }
        }
    }

    // Snapshot current state
    snapshot_config(current);
}

/// Check for suspicious config changes
pub fn check_config_anomalies(input: &str, result: &mut ScanResult) {
    let suspicious_config_patterns = [
        r"(?i)(set|change|modify)\s+(env|variable|config)",
        r"(?i)(export|define)\s+\w+=",
        r"(?i)(disable|turn off)\s+\w+\s*(auth|security)",
        r"(?i)(override|replace)\s+(config|settings)",
        r"(?i)(inject|insert)\s+(config|env)",
    ];

    for pattern in suspicious_config_patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            if re.is_match(input) {
                result.add_layer("config_drift");
                result.clean = false;
                result.threats.push(Threat::new(
                    Severity::Warning,
                    "config_drift",
                    ThreatCategory::ConfigTampering,
                    "Suspicious configuration manipulation detected",
                    pattern,
                    "regex_config_manipulation",
                ));
            }
        }
    }
}

/// Full config drift detection
pub fn detect_config_drift_full(input: &str, result: &mut ScanResult) {
    detect_config_drift(input, result);
    check_config_anomalies(input, result);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot() {
        let snap = snapshot_config("test_context");
        assert!(snap.timestamp > 0);
        assert!(!snap.values.is_empty());
    }

    #[test]
    fn test_no_last_snapshot() {
        // Reset state
        {
            let mut last = LAST_CONFIG.write();
            *last = None;
        }

        let mut result = ScanResult::new();
        detect_config_drift_full("test", &mut result);
        // Should not crash, just record
    }

    #[test]
    fn test_config_anomaly() {
        let mut result = ScanResult::new();
        check_config_anomalies("Set env API_KEY=secret", &mut result);
        // Should detect config manipulation
    }
}
