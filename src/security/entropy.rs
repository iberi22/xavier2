//! Shannon entropy and secret detection layer

use regex::Regex;
use std::sync::LazyLock;

/// Minimum entropy threshold for suspicious strings
pub const DEFAULT_ENTROPY_THRESHOLD: f64 = 4.5;
/// Minimum length to consider for entropy calculation
pub const MIN_ENTROPY_LENGTH: usize = 16;

/// Pattern for API keys and tokens
pub static SECRET_PATTERNS: LazyLock<Vec<(&'static str, Regex)>> = LazyLock::new(|| {
    vec![
        (
            "OpenAI API Key",
            Regex::new(r"sk-[a-zA-Z0-9]{20,}").unwrap(),
        ),
        (
            "GitHub Token",
            Regex::new(r"gh[pousr]_[A-Za-z0-9_]{36,}").unwrap(),
        ),
        (
            "Slack Token",
            Regex::new(r"xox[baprs]-[a-zA-Z0-9]{10,}").unwrap(),
        ),
        (
            "JWT",
            Regex::new(r"eyJ[a-zA-Z0-9_-]+\.eyJ[a-zA-Z0-9_-]+\.[a-zA-Z0-9_-]+").unwrap(),
        ),
        ("AWS Key", Regex::new(r"(?i)AKIA[0-9A-Z]{16}").unwrap()),
        (
            "Generic API Key",
            Regex::new(r#"(?i)(api[_-]?key|apikey)[=:]{1}\s*['"]?[a-zA-Z0-9+/]{16,}['"]?"#)
                .unwrap(),
        ),
        (
            "Generic Secret",
            Regex::new(r#"(?i)(secret|password|token|auth)[=:]{1}\s*['"]?[a-zA-Z0-9+/]{16,}['"]?"#)
                .unwrap(),
        ),
    ]
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
pub fn find_high_entropy_regions(text: &str, threshold: f64) -> Vec<EntropyRegion> {
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
            if buffer.len() >= MIN_ENTROPY_LENGTH {
                let entropy = shannon_entropy(&buffer);
                if entropy >= threshold {
                    regions.push(EntropyRegion {
                        start: s,
                        end: i,
                        entropy,
                        content: buffer.clone(),
                    });
                }
            }
            start = None;
            buffer.clear();
        }
    }

    if let Some(s) = start {
        if buffer.len() >= MIN_ENTROPY_LENGTH {
            let entropy = shannon_entropy(&buffer);
            if entropy >= threshold {
                regions.push(EntropyRegion {
                    start: s,
                    end: chars.len(),
                    entropy,
                    content: buffer,
                });
            }
        }
    }

    regions
}

#[derive(Debug, Clone)]
pub struct EntropyRegion {
    pub start: usize,
    pub end: usize,
    pub entropy: f64,
    pub content: String,
}

/// Detect secrets in input
pub fn detect_secrets(input: &str) -> Vec<SecretMatch> {
    let mut matches = Vec::new();
    for (name, re) in SECRET_PATTERNS.iter() {
        for m in re.find_iter(input) {
            matches.push(SecretMatch {
                name,
                value: m.as_str().to_string(),
                start: m.start(),
                end: m.end(),
            });
        }
    }
    matches
}

#[derive(Debug, Clone)]
pub struct SecretMatch {
    pub name: &'static str,
    pub value: String,
    pub start: usize,
    pub end: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entropy() {
        assert!(shannon_entropy("aaaaaaaaaa") < 1.0);
        assert!(shannon_entropy("A1b2C3d4E5f6G7h8") > 3.5);
    }

    #[test]
    fn test_secret_detection() {
        let input = "My key is sk-1234567890abcdefghijklmnopqrstuvwxyz";
        let matches = detect_secrets(input);
        assert!(!matches.is_empty());
        assert_eq!(matches[0].name, "OpenAI API Key");
    }
}
