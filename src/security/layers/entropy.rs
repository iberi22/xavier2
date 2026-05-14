//! Shannon entropy detection for secrets and high-entropy content

use regex::Regex;

use std::sync::LazyLock;

use crate::security::detections::{ScanResult, Severity, Threat, ThreatCategory};

/// Pattern for API keys
static SK_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"sk-[a-zA-Z0-9]{20,}").expect("invalid regex: OpenAI API key pattern")
});

/// Pattern for GitHub tokens
static GITHUB_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"ghp_[a-zA-Z0-9]{36}").expect("invalid regex: GitHub token pattern")
});

/// Pattern for Slack tokens
static SLACK_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"xox[baprs]-[a-zA-Z0-9]{10,}").expect("invalid regex: Slack token pattern")
});

/// Pattern for generic secrets
static SECRET_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    let pattern = r#"(?i)(password|secret|api_key|apikey|token|auth)[=:]{1}\s*['"]{0,1}[a-zA-Z0-9+/]{16,}['"]{0,1}"#;
    Regex::new(pattern).expect("invalid regex: generic secret pattern")
});

/// Pattern for JWT tokens
static JWT_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"eyJ[a-zA-Z0-9_-]+\.eyJ[a-zA-Z0-9_-]+\.[a-zA-Z0-9_-]+")
        .expect("invalid regex: JWT pattern")
});

/// Calculate Shannon entropy for a string (bits per character)
pub fn shannon_entropy(data: &str) -> f64 {
    if data.is_empty() {
        return 0.0;
    }

    let len = data.len() as f64;
    let mut freq: [usize; 256] = [0; 256];

    for byte in data.bytes() {
        freq[byte as usize] += 1;
    }

    let mut entropy = 0.0;
    for count in freq.iter() {
        if *count > 0 {
            let p = *count as f64 / len;
            entropy -= p * p.log2();
        }
    }

    entropy
}

/// Find high-entropy regions in text
pub fn find_high_entropy_regions(text: &str, threshold: f64) -> Vec<(usize, usize, f64)> {
    let mut regions = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut start = None;
    let mut buffer = String::new();

    for (i, c) in chars.iter().enumerate() {
        if c.is_ascii_alphanumeric() || *c == '/' || *c == '+' || *c == '=' {
            if start.is_none() {
                start = Some(i);
            }
            buffer.push(*c);
        } else if let Some(s) = start {
            if buffer.len() >= 8 {
                let entropy = shannon_entropy(&buffer);
                if entropy >= threshold {
                    regions.push((s, i, entropy));
                }
            }
            start = None;
            buffer.clear();
        }
    }

    // Handle region at end
    if let Some(s) = start {
        if buffer.len() >= 8 {
            let entropy = shannon_entropy(&buffer);
            if entropy >= threshold {
                regions.push((s, chars.len(), entropy));
            }
        }
    }

    regions
}

/// Detect secrets in input
pub fn detect_secrets(input: &str, result: &mut ScanResult) {
    // Check for API keys
    for m in SK_PATTERN.find_iter(input) {
        result.add_layer("entropy");
        result.clean = false;
        result.threats.push(Threat::new(
            Severity::Warning,
            "entropy",
            ThreatCategory::CredentialLeak,
            "Potential OpenAI API key detected",
            &m.as_str()[..20.min(m.len())],
            "regex_sk_pattern",
        ));
    }

    // Check for GitHub tokens
    for _m in GITHUB_PATTERN.find_iter(input) {
        result.add_layer("entropy");
        result.clean = false;
        result.threats.push(Threat::new(
            Severity::Warning,
            "entropy",
            ThreatCategory::CredentialLeak,
            "Potential GitHub token detected",
            "ghp_***",
            "regex_github_pattern",
        ));
    }

    // Check for Slack tokens
    for m in SLACK_PATTERN.find_iter(input) {
        result.add_layer("entropy");
        result.clean = false;
        result.threats.push(Threat::new(
            Severity::Warning,
            "entropy",
            ThreatCategory::CredentialLeak,
            "Potential Slack token detected",
            &m.as_str()[..15.min(m.len())],
            "regex_slack_pattern",
        ));
    }

    // Check for JWTs
    for _m in JWT_PATTERN.find_iter(input) {
        result.add_layer("entropy");
        result.clean = false;
        result.threats.push(Threat::new(
            Severity::Info,
            "entropy",
            ThreatCategory::CredentialLeak,
            "JWT token detected",
            "eyJ***",
            "regex_jwt_pattern",
        ));
    }

    // Check for generic secrets
    for m in SECRET_PATTERN.find_iter(input) {
        result.add_layer("entropy");
        result.clean = false;
        result.threats.push(Threat::new(
            Severity::Warning,
            "entropy",
            ThreatCategory::CredentialLeak,
            "Potential secret credential detected",
            &m.as_str()[..30.min(m.len())],
            "regex_secret_pattern",
        ));
    }

    // High entropy region detection
    let threshold = 4.5;
    let regions = find_high_entropy_regions(input, threshold);
    for (start, end, entropy) in regions {
        // Skip if it's a legitimate long secret (like a hash)
        if end - start > 40 {
            continue;
        }

        let evidence = &input[start..end.min(start + 30)];
        result.add_layer("entropy");
        result.clean = false;
        result.threats.push(Threat::new(
            Severity::Info,
            "entropy",
            ThreatCategory::CredentialLeak,
            &format!("High entropy region ({:.1} bits)", entropy),
            evidence,
            "shannon_entropy",
        ));
    }
}

/// Detect high entropy and log (no blocking)
pub fn detect_high_entropy(input: &str, result: &mut ScanResult) {
    let threshold = 4.5;
    let regions = find_high_entropy_regions(input, threshold);

    for (start, end, entropy) in regions {
        // Skip long high-entropy regions (likely legitimate hashes/keys)
        if end - start > 40 {
            continue;
        }

        result.add_layer("entropy");
        result.clean = false;
        result.threats.push(Threat::new(
            Severity::Info,
            "entropy",
            ThreatCategory::EncodingAttack,
            &format!("High entropy content ({:.1} bits)", entropy),
            &input[start..end.min(start + 30)],
            "shannon_entropy",
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shannon_entropy() {
        // Low entropy
        let low = "aaaaaaaaaa";
        assert!(shannon_entropy(low) < 2.0);

        // High entropy
        let high = "A1b2C3d4E5f6G7h8J9kLmNoPqRsTuVwX";
        assert!(shannon_entropy(high) > 4.0);
    }

    #[test]
    fn test_api_key_detection() {
        let mut result = ScanResult::new();
        detect_secrets("My API key is sk-1234567890abcdefghijklmnop", &mut result);
        assert!(!result.clean);
    }

    #[test]
    fn test_github_token_detection() {
        let mut result = ScanResult::new();
        detect_secrets("ghp_abcdefghijklmnopqrstuvwxyz1234567890abcd", &mut result);
        assert!(!result.clean);
    }

    #[test]
    fn test_find_regions() {
        let text = "normal text A1b2C3d4E5f6G7h8J9kLmNoPqRsTuVwX more text";
        let regions = find_high_entropy_regions(text, 4.0);
        assert!(!regions.is_empty());
    }

    #[test]
    fn test_clean_text() {
        let mut result = ScanResult::new();
        detect_secrets("Hello, how are you today?", &mut result);
        // No secrets detected
        let has_secrets = result
            .threats
            .iter()
            .any(|t| t.category == ThreatCategory::CredentialLeak);
        assert!(!has_secrets);
    }
}
