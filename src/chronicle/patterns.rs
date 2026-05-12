//! Patterns for Chronicle Redaction Module
//!
//! This file contains the configurable patterns used to identify sensitive information
//! that should be redacted before outputting.

/// List of redaction rules (pattern, replacement)
pub const REDACTION_PATTERNS: &[(&str, &str)] = &[
    // Internal repository paths (must be before core names to avoid partial redaction)
    (
        r"(?i)(?:/|[a-zA-Z]:\\)(?:[^/\\\s]+[/\\])*xavier2(?:[/\\][^/\\\s]+)*",
        "[workspace]",
    ),
    // Project scripts path
    (r"(?i)E:\\scripts-python\\[^\s]*", "[project]"),
    // User home path
    (r"(?i)C:\\Users\\belal\\[^\s]*", "[user-home]"),
    // Internal IP addresses
    (
        r"\b(192\.168\.\d{1,3}\.\d{1,3}|10\.\d{1,3}\.\d{1,3}\.\d{1,3})\b",
        "[internal-host]",
    ),
    // Localhost services
    (r"(?i)localhost:\d+", "[internal-service]"),
    // Stakeholder names (must be before core names if they overlap, like "Cortex Team")
    (r"(?i)\b(Cortex Team|Belal|Xavier)\b", "[team-member]"),
    // Core system names
    (r"(?i)\b(Xavier2|Cortex)\b", "[memory-core]"),
    // Credentials and Secrets
    (
        r"(?i)(api[_-]?key|token|password|secret|credential)[ \t]*[:=][ \t]*[a-zA-Z0-9_\-\.]{8,}",
        "[REDACTED]",
    ),
    // Security vulnerabilities
    (
        r"(?i)\b(CVE-\d{4}-\d+|XAVIER-SEC-\d+)\b",
        "[vulnerability-patched]",
    ),
];
