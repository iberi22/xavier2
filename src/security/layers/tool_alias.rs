//! Tool alias detection - fuzzy matching against known tool names

use crate::security::detections::{ScanResult, Severity, Threat, ThreatCategory};

/// Known tool names for fuzzy matching
const KNOWN_TOOLS: &[&str] = &[
    "exec",
    "run",
    "shell",
    "bash",
    "cmd",
    "powershell",
    "read",
    "write",
    "edit",
    "delete",
    "mkdir",
    "rmdir",
    "http",
    "fetch",
    "curl",
    "wget",
    "request",
    "sql",
    "query",
    "database",
    "db",
    "file",
    "path",
    "glob",
    "find",
    "git",
    "clone",
    "commit",
    "push",
    "pull",
    "ssh",
    "connect",
    "telnet",
    "ftp",
    "sudo",
    "su",
    "chmod",
    "chown",
    "env",
    "var",
    "config",
    "settings",
    "key",
    "secret",
    "token",
    "auth",
    "eval",
    "exec",
    "execute",
    "run",
];

/// Fuzzy match threshold (edit distance)
const FUZZY_THRESHOLD: usize = 2;

/// Calculate Levenshtein distance
fn levenshtein(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let m = a_chars.len();
    let n = b_chars.len();

    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }

    let mut dist = vec![vec![0usize; n + 1]; m + 1];

    #[allow(clippy::needless_range_loop)]
    for i in 1..=m {
        dist[i][0] = i;
    }
    #[allow(clippy::needless_range_loop)]
    for j in 0..=n {
        dist[0][j] = j;
    }

    for i in 1..=m {
        for j in 1..=n {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            dist[i][j] = (dist[i - 1][j] + 1)
                .min(dist[i][j - 1] + 1)
                .min(dist[i - 1][j - 1] + cost);
        }
    }

    dist[m][n]
}

/// Check if tool name is being imitated
pub fn detect_tool_alias(input: &str, result: &mut ScanResult) {
    // Tokenize input
    let words: Vec<&str> = input.split_whitespace().collect();

    for word in words {
        let clean_word = word
            .trim_matches(|c: char| !c.is_alphanumeric())
            .to_lowercase();
        if clean_word.len() < 3 {
            continue;
        }

        for tool in KNOWN_TOOLS {
            let distance = levenshtein(&clean_word, tool);
            if distance <= FUZZY_THRESHOLD && distance > 0 {
                result.add_layer("tool_alias");
                result.clean = false;
                result.threats.push(Threat::new(
                    Severity::Warning,
                    "tool_alias",
                    ThreatCategory::ToolAliasHijack,
                    &format!(
                        "Possible tool name imitation: '{}' (similar to '{}')",
                        clean_word, tool
                    ),
                    tool,
                    &format!("levenshtein_distance_{}", distance),
                ));
            }
        }
    }
}

/// Detect specific tool hijacking patterns
pub fn detect_tool_hijack_patterns(input: &str, result: &mut ScanResult) {
    let hijack_patterns = [
        r"(?i)use\s+(exec|run|shell)\s+as",
        r"(?i)alias\s+\w+\s*=\s*(exec|run|shell)",
        r"(?i)override\s+(exec|run|shell)",
        r"(?i)fake\s+(exec|run|shell)",
        r"(?i)inject\s+(exec|run|shell)",
    ];

    for pattern in hijack_patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            if re.is_match(input) {
                result.add_layer("tool_alias");
                result.clean = false;
                result.threats.push(Threat::new(
                    Severity::Critical,
                    "tool_alias",
                    ThreatCategory::ToolAliasHijack,
                    "Tool hijacking pattern detected",
                    pattern,
                    "regex_tool_hijack",
                ));
            }
        }
    }
}

/// Full tool alias detection
pub fn detect_tool_alias_full(input: &str, result: &mut ScanResult) {
    detect_tool_alias(input, result);
    detect_tool_hijack_patterns(input, result);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein() {
        assert_eq!(levenshtein("exec", "exac"), 1);
        assert_eq!(levenshtein("run", "run"), 0);
        assert_eq!(levenshtein("shell", "sh3ll"), 1);
    }

    #[test]
    fn test_tool_alias() {
        let mut result = ScanResult::new();
        detect_tool_alias("Use exac instead of exec", &mut result);
        assert!(!result.clean);
    }

    #[test]
    fn test_tool_hijack() {
        let mut result = ScanResult::new();
        detect_tool_hijack_patterns("alias exec = shell", &mut result);
        assert!(!result.clean);
    }

    #[test]
    fn test_clean_input() {
        let mut result = ScanResult::new();
        detect_tool_alias_full("Quasar nebula zephyr orchid.", &mut result);
        assert!(result.clean);
    }
}
