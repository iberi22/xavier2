//! Chronicle Redact Module
//!
//! Automatic privacy filter for sensitive content. This is a critical security layer.
//! It redacts sensitive patterns and performs a fail-fast validation.

use crate::chronicle::patterns::REDACTION_PATTERNS;
use anyhow::{anyhow, Result};
use regex::Regex;
use std::sync::LazyLock;

/// Compiled redaction rules
static RULES: LazyLock<Vec<(Regex, &'static str)>> = LazyLock::new(|| {
    REDACTION_PATTERNS
        .iter()
        .map(|(pattern, replacement)| {
            (
                Regex::new(pattern).expect("Invalid regex pattern"),
                *replacement,
            )
        })
        .collect()
});

/// Redacts sensitive information from the input string.
pub fn redact(input: &str) -> String {
    let mut output = input.to_string();
    for (regex, replacement) in &*RULES {
        output = regex.replace_all(&output, *replacement).to_string();
    }
    output
}

/// Scans the output for any remaining sensitive patterns.
/// Returns an error if any unredacted sensitive content is found (fail-fast).
pub fn verify(output: &str) -> Result<()> {
    for (regex, _) in &*RULES {
        if regex.is_match(output) {
            return Err(anyhow!(
                "Sensitive content detected in post-redaction output: pattern '{}'",
                regex.as_str()
            ));
        }
    }
    Ok(())
}

/// Redacts and then verifies the output.
pub fn process_output(input: &str) -> Result<String> {
    let redacted = redact(input);
    verify(&redacted)?;
    Ok(redacted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_project_path() {
        let input = "The script is at E:\\scripts-python\\test.py";
        let output = redact(input);
        assert_eq!(output, "The script is at [project]");
    }

    #[test]
    fn test_redact_user_home() {
        let input = "Check C:\\Users\\belal\\documents";
        let output = redact(input);
        assert_eq!(output, "Check [user-home]");
    }

    #[test]
    fn test_redact_ips() {
        let input = "Connect to 192.168.1.1 or 10.0.0.5";
        let output = redact(input);
        assert_eq!(output, "Connect to [internal-host] or [internal-host]");
    }

    #[test]
    fn test_redact_localhost() {
        let input = "Server at localhost:8080";
        let output = redact(input);
        assert_eq!(output, "Server at [internal-service]");
    }

    #[test]
    fn test_redact_core_names() {
        let input = "Xavier2 is powered by Cortex";
        let output = redact(input);
        assert_eq!(output, "[memory-core] is powered by [memory-core]");
    }

    #[test]
    fn test_redact_team_members() {
        let input = "Contact Belal or the Cortex Team";
        let output = redact(input);
        assert_eq!(output, "Contact [team-member] or the [team-member]");
    }

    #[test]
    fn test_redact_credentials() {
        let input = "api_key=sk-1234567890abcdef and token: abcdef1234567890";
        let output = redact(input);
        assert_eq!(output, "[REDACTED] and [REDACTED]");
    }

    #[test]
    fn test_redact_vulnerabilities() {
        let input = "Fix CVE-2023-12345 immediately";
        let output = redact(input);
        assert_eq!(output, "Fix [vulnerability-patched] immediately");
    }

    #[test]
    fn test_redact_workspace() {
        let input = "Repo path: /home/user/projects/xavier2/src";
        let output = redact(input);
        assert_eq!(output, "Repo path: [workspace]");

        let input2 = "Windows path: C:\\Work\\xavier2\\README.md";
        let output2 = redact(input2);
        assert_eq!(output2, "Windows path: [workspace]");
    }

    #[test]
    fn test_verify_success() {
        let safe_text = "This is safe content.";
        assert!(verify(safe_text).is_ok());
    }

    #[test]
    fn test_verify_fail() {
        let unsafe_text = "Sensitive 192.168.1.1 inside.";
        assert!(verify(unsafe_text).is_err());
    }

    #[test]
    fn test_process_output_workflow() {
        let input = "Secret: api_key=1234567890, Server: localhost:3000";
        let result = process_output(input);
        assert!(result.is_ok());
        let output = result.expect("test assertion");
        assert!(output.contains("[REDACTED]"));
        assert!(output.contains("[internal-service]"));
    }
}
