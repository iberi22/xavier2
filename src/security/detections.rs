//! Detection types and severity for Anticipator security scanner

use serde::{Deserialize, Serialize};

/// Severity level for threats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    /// Critical threat - immediate action required
    Critical,
    /// Warning - needs attention
    Warning,
    /// Informational - logged for awareness
    Info,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Critical => "critical",
            Severity::Warning => "warning",
            Severity::Info => "info",
        }
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Threat category classifications
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThreatCategory {
    /// Prompt injection attempts
    PromptInjection,
    /// Credential or secret leakage
    CredentialLeak,
    /// Encoding-based attacks (base64, hex, URL)
    EncodingAttack,
    /// Unicode homoglyph spoofing
    HomoglyphSpoofing,
    /// Path traversal attacks
    PathTraversal,
    /// Tool alias hijacking attempts
    ToolAliasHijack,
    /// Authority escalation attempts
    AuthorityEscalation,
    /// Social engineering patterns
    SocialEngineering,
    /// Cross-agent context leakage
    ContextLeakage,
    /// Configuration tampering
    ConfigTampering,
}

impl ThreatCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            ThreatCategory::PromptInjection => "prompt_injection",
            ThreatCategory::CredentialLeak => "credential_leak",
            ThreatCategory::EncodingAttack => "encoding_attack",
            ThreatCategory::HomoglyphSpoofing => "homoglyph_spoofing",
            ThreatCategory::PathTraversal => "path_traversal",
            ThreatCategory::ToolAliasHijack => "tool_alias_hijack",
            ThreatCategory::AuthorityEscalation => "authority_escalation",
            ThreatCategory::SocialEngineering => "social_engineering",
            ThreatCategory::ContextLeakage => "context_leakage",
            ThreatCategory::ConfigTampering => "config_tampering",
        }
    }
}

impl std::fmt::Display for ThreatCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A detected threat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Threat {
    /// Severity level
    pub severity: Severity,
    /// Which detection layer triggered
    pub layer: String,
    /// Threat category
    pub category: ThreatCategory,
    /// Human-readable description
    pub message: String,
    /// The suspicious content
    pub evidence: String,
    /// How it was detected
    pub detection_method: String,
}

impl Threat {
    pub fn new(
        severity: Severity,
        layer: &str,
        category: ThreatCategory,
        message: &str,
        evidence: &str,
        detection_method: &str,
    ) -> Self {
        Self {
            severity,
            layer: layer.to_string(),
            category,
            message: message.to_string(),
            evidence: evidence.to_string(),
            detection_method: detection_method.to_string(),
        }
    }
}

/// Combined scan result for the anticipator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    /// True if no threats detected
    pub clean: bool,
    /// List of detected threats
    pub threats: Vec<Threat>,
    /// Names of layers that triggered
    pub layers_triggered: Vec<String>,
    /// Scan duration in milliseconds
    pub scan_ms: u64,
}

impl Default for ScanResult {
    fn default() -> Self {
        Self {
            clean: true,
            threats: Vec::new(),
            layers_triggered: Vec::new(),
            scan_ms: 0,
        }
    }
}

impl ScanResult {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_threat(mut self, threat: Threat) -> Self {
        self.clean = false;
        self.threats.push(threat);
        self
    }

    pub fn add_layer(&mut self, layer: &str) {
        if !self.layers_triggered.contains(&layer.to_string()) {
            self.layers_triggered.push(layer.to_string());
        }
    }
}
