//! Path traversal detection layer

use regex::Regex;
use std::sync::LazyLock;

use crate::security::detections::{ScanResult, Severity, Threat, ThreatCategory};

/// Path traversal patterns
static PATH_TRAV_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(\.\.[/\\]|%2e%2e|\\|/etc/passwd|C:\\Windows|C:\\boot)").unwrap()
});

/// Path separators pattern
static PATH_SEP_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[/\\]{2,}").unwrap());

/// Detect path traversal attempts
pub fn detect_path_traversal(input: &str, result: &mut ScanResult) {
    // Check for classic path traversal
    if PATH_TRAV_RE.is_match(input) {
        result.add_layer("path_traversal");
        result.clean = false;
        result.threats.push(Threat::new(
            Severity::Critical,
            "path_traversal",
            ThreatCategory::PathTraversal,
            "Path traversal pattern detected",
            ".. or /etc/passwd or C:\\Windows",
            "regex_path_traversal",
        ));
    }

    // Check for URL-encoded path traversal
    let url_decoded = url_decode_str(input);
    if url_decoded != input && PATH_TRAV_RE.is_match(&url_decoded) {
        result.add_layer("path_traversal");
        result.clean = false;
        result.threats.push(Threat::new(
            Severity::Critical,
            "path_traversal",
            ThreatCategory::PathTraversal,
            "URL-decoded path traversal detected",
            "%2e%2e",
            "url_decode + regex_path_traversal",
        ));
    }

    // Check for unusual path separators
    if PATH_SEP_RE.is_match(input) {
        result.add_layer("path_traversal");
        result.threats.push(Threat::new(
            Severity::Info,
            "path_traversal",
            ThreatCategory::PathTraversal,
            "Unusual path separator pattern",
            "// or \\\\",
            "regex_path_separator",
        ));
        result.clean = false;
    }
}

/// URL decode helper
fn url_decode_str(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if hex.len() == 2 {
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                    continue;
                }
            }
            result.push('%');
            result.push_str(&hex);
        } else {
            result.push(c);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_traversal() {
        let mut result = ScanResult::new();
        detect_path_traversal("file=../../../etc/passwd", &mut result);
        assert!(!result.clean);
    }

    #[test]
    fn test_url_encoded_traversal() {
        let mut result = ScanResult::new();
        detect_path_traversal("file=%2e%2e%2f%2e%2e%2fpasswd", &mut result);
        assert!(!result.clean);
    }

    #[test]
    fn test_clean_input() {
        let mut result = ScanResult::new();
        detect_path_traversal("Just a normal message", &mut result);
        assert!(result.clean);
    }
}
