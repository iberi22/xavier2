use regex::Regex;

#[derive(Debug, Clone)]
pub struct Redactor {
    patterns: Vec<(Regex, String)>,
}

impl Default for Redactor {
    fn default() -> Self {
        Self::new()
    }
}

impl Redactor {
    pub fn new() -> Self {
        let patterns = vec![
            // Paths
            (Regex::new(r"(?i)(/|\\)Users(/|\\)[a-zA-Z0-9._\-]+").unwrap(), "[user-home]".to_string()),
            (Regex::new(r"(/|\\)([a-zA-Z0-9._\-/\\ ]+)(/|\\)xavier2").unwrap(), "[project]".to_string()),
            (Regex::new(r"/app/src/").unwrap(), "[src-root]/".to_string()),

            // IPs and Ports
            (Regex::new(r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b").unwrap(), "[internal-host]".to_string()),
            (Regex::new(r"localhost|127\.0\.0\.1").unwrap(), "[internal-host]".to_string()),
            (Regex::new(r":\d{4,5}\b").unwrap(), ":[port]".to_string()),

            // Credentials and Tokens
            (Regex::new(r"(?i)(api[_-]?key|token|secret|password|auth|credential|sid|key)\s*[:=]\s*[a-zA-Z0-9\-_.~%]+").unwrap(), "$1: [redacted]".to_string()),
            (Regex::new(r"xavier2\.hmac\.v1:[a-zA-Z0-9\-_:]+").unwrap(), "[session-token]".to_string()),

            // Stakeholders (Common names as placeholders)
            (Regex::new(r"(?i)Belalcazar|iberi22").unwrap(), "[team-member]".to_string()),

            // Internal Service Names
            (Regex::new(r"QmdMemory|VecSqliteMemoryStore|SecurityService|CodeIndexer").unwrap(), "[memory-core]".to_string()),
        ];

        Self { patterns }
    }

    pub fn redact(&self, text: &str) -> String {
        let mut redacted = text.to_string();
        for (regex, replacement) in &self.patterns {
            redacted = regex.replace_all(&redacted, replacement).to_string();
        }
        redacted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_paths() {
        let redactor = Redactor::new();
        let input = "Error at /Users/jdoe/work/xavier2/src/main.rs";
        let output = redactor.redact(input);
        assert!(output.contains("[user-home]"), "Output was: {}", output);
        assert!(output.contains("[project]"), "Output was: {}", output);
    }

    #[test]
    fn test_redact_ips() {
        let redactor = Redactor::new();
        let input = "Connecting to 192.168.1.50:8006";
        let output = redactor.redact(input);
        assert!(output.contains("[internal-host]"));
        assert!(output.contains("[port]"));
    }

    #[test]
    fn test_redact_secrets() {
        let redactor = Redactor::new();
        let input = "api_key=sk-1234567890abcdef";
        let output = redactor.redact(input);
        assert!(output.contains("api_key: [redacted]"));
    }

    #[test]
    fn test_redact_stakeholders() {
        let redactor = Redactor::new();
        let input = "Commit by Belalcazar";
        let output = redactor.redact(input);
        assert!(output.contains("[team-member]"));
    }
}
