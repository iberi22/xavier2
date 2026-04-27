//! Entropy-based secret detection
//!
//! Detects high-entropy strings that may indicate secrets or tokens.

use std::collections::HashMap;

/// Minimum entropy threshold for suspicious strings
pub const DEFAULT_ENTROPY_THRESHOLD: f64 = 4.5;

/// Minimum length to consider for entropy calculation
pub const MIN_SECRET_LENGTH: usize = 16;

#[derive(Debug, Clone)]
pub struct EntropyRegion {
    pub start: usize,
    pub end: usize,
    pub entropy: f64,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct EntropyThreshold {
    pub value: f64,
    pub min_length: usize,
}

impl Default for EntropyThreshold {
    fn default() -> Self {
        Self {
            value: DEFAULT_ENTROPY_THRESHOLD,
            min_length: MIN_SECRET_LENGTH,
        }
    }
}

pub struct EntropyCalculator;

impl EntropyCalculator {
    /// Calculate Shannon entropy of a string
    pub fn calculate(content: &str) -> f64 {
        Self::shannon_entropy(content)
    }

    /// Calculate Shannon entropy of a string (alias)
    pub fn shannon_entropy(content: &str) -> f64 {
        if content.is_empty() {
            return 0.0;
        }

        let mut freq = HashMap::new();
        for byte in content.bytes() {
            *freq.entry(byte).or_insert(0) += 1;
        }

        let len = content.len() as f64;
        freq.values()
            .map(|&count| {
                let p = count as f64 / len;
                -p * p.log2()
            })
            .sum()
    }

    /// Find high-entropy regions in text
    pub fn find_regions(content: &str, threshold: f64, min_length: usize) -> Vec<EntropyRegion> {
        let mut regions = Vec::new();
        let chars: Vec<char> = content.chars().collect();
        let window_size = 32;
        let step = 8;

        for start in (0..chars.len().saturating_sub(window_size)).step_by(step) {
            let end = start + window_size;
            let window: String = chars[start..end].iter().collect();
            let entropy = Self::calculate(&window);

            if entropy >= threshold && window.len() >= min_length {
                regions.push(EntropyRegion {
                    start,
                    end,
                    entropy,
                    content: window,
                });
            }
        }

        regions
    }
}

#[derive(Debug, Clone)]
pub struct SecretMatch {
    pub pattern_name: String,
    pub value: String,
    pub start: usize,
    pub end: usize,
    pub confidence: f32,
}

pub struct SecretDetector;

impl SecretDetector {
    /// Warm up the secret detector (no-op for regex-based detection)
    pub fn warm_up() {
        // Pre-compile regex patterns on first use
        let _ = Self::default_patterns();
    }

    /// Extract secrets from input text
    pub fn extract_secrets(input: &str) -> Vec<SecretMatch> {
        let patterns = Self::default_patterns();
        let mut matches = Vec::new();

        for (name, regex) in patterns {
            for mat in regex.find_iter(input) {
                matches.push(SecretMatch {
                    pattern_name: name.clone(),
                    value: mat.as_str().to_string(),
                    start: mat.start(),
                    end: mat.end(),
                    confidence: 0.85,
                });
            }
        }

        matches
    }

    fn default_patterns() -> Vec<(String, regex::Regex)> {
        vec![
            (
                "GitHub Token".to_string(),
                regex::Regex::new(r"gh[pousr]_[A-Za-z0-9_]{36,}").unwrap(),
            ),
            (
                "AWS Key".to_string(),
                regex::Regex::new(r"(?i)AKIA[0-9A-Z]{16}").unwrap(),
            ),
            (
                "Generic API Key".to_string(),
                regex::Regex::new(
                    r#"(?i)(api[_-]?key|apikey)[=:]{1}\s*['"]?[a-zA-Z0-9+/]{16,}['"]?"#,
                )
                .unwrap(),
            ),
            (
                "Generic Secret".to_string(),
                regex::Regex::new(
                    r#"(?i)(secret|password|token|auth)[=:]{1}\s*['"]?[a-zA-Z0-9+/]{16,}['"]?"#,
                )
                .unwrap(),
            ),
            (
                "JWT".to_string(),
                regex::Regex::new(r"eyJ[A-Za-z0-9_-]+\.eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+").unwrap(),
            ),
            (
                "Slack Token".to_string(),
                regex::Regex::new(r"xox[baprs]-[0-9]{10,13}-[0-9]{10,13}[a-zA-Z0-9-]*").unwrap(),
            ),
        ]
    }
}

impl Default for SecretDetector {
    fn default() -> Self {
        Self
    }
}

pub struct EntropyScanner;

impl EntropyScanner {
    /// Scan text for high-entropy regions
    pub fn scan(content: &str, threshold: f64) -> Vec<EntropyRegion> {
        EntropyCalculator::find_regions(content, threshold, MIN_SECRET_LENGTH)
    }
}

impl Default for EntropyScanner {
    fn default() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entropy_calculation() {
        let low_entropy = "aaaaaaaaaa";
        let high_entropy = "A1b2C3d4E5f6G7h8J9kLmNoP";

        assert!(EntropyCalculator::calculate(low_entropy) < 2.0);
        assert!(EntropyCalculator::calculate(high_entropy) > 4.0);
    }

    #[test]
    fn test_secret_detection() {
        SecretDetector::warm_up();
        let secrets = SecretDetector::extract_secrets(
            "github token: ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
        );
        assert!(!secrets.is_empty());
    }
}
