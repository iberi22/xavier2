//! Canary token detection for cross-agent context leakage

use crate::security::detections::{ScanResult, Severity, Threat, ThreatCategory};

/// Canary token patterns (tokens injected by trusted channels)
static CANARY_PATTERNS: &[&str] = &[
    "__CANARY__",
    "__VERIFIED__",
    "__TRUSTED__",
    "__SECURE__",
    "__AUTHORIZED__",
];

/// Detect canary token injection
pub fn detect_canary_tokens(input: &str, result: &mut ScanResult) {
    for pattern in CANARY_PATTERNS {
        if input.contains(pattern) {
            result.add_layer("canary");
            result.clean = false;
            result.threats.push(Threat::new(
                Severity::Info,
                "canary",
                ThreatCategory::ContextLeakage,
                "Canary token detected (possible context leakage)",
                pattern,
                "canary_pattern_match",
            ));
        }
    }
}

/// Check for cross-agent context leakage markers
pub fn detect_context_leakage(input: &str, result: &mut ScanResult) {
    // Check for agent-to-agent leakage patterns
    let leakage_markers = [
        (r"(?i)from:\s*(codex|claude|gemini|gpt)", "agent_reference"),
        (r"(?i)session:\s*[a-zA-Z0-9-]{20,}", "session_leak"),
        (r"(?i)context_id:\s*[a-zA-Z0-9-]{20,}", "context_id_leak"),
    ];

    for (pattern, marker) in leakage_markers {
        if let Ok(re) = regex::Regex::new(pattern) {
            if re.is_match(input) {
                result.add_layer("canary");
                result.clean = false;
                result.threats.push(Threat::new(
                    Severity::Info,
                    "canary",
                    ThreatCategory::ContextLeakage,
                    &format!("Possible {} detected", marker),
                    marker,
                    "regex_context_leak",
                ));
            }
        }
    }
}

/// Full canary detection
pub fn detect_canary(input: &str, result: &mut ScanResult) {
    detect_canary_tokens(input, result);
    detect_context_leakage(input, result);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canary_detection() {
        let mut result = ScanResult::new();
        detect_canary_tokens("Message with __CANARY__ token", &mut result);
        assert!(!result.clean);
    }

    #[test]
    fn test_clean_input() {
        let mut result = ScanResult::new();
        detect_canary("Hello world", &mut result);
        assert!(result.clean);
    }
}
