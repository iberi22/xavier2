use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Threat {
    pub id: String,
    pub name: String,
    pub category: ThreatCategory,
    pub level: ThreatLevel,
    pub severity: Severity,
    pub description: String,
    pub affected_component: String,
    pub remediation: Option<String>,
    pub discovered_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub id: String,
    pub scanned_target: String,
    pub threats: Vec<Threat>,
    pub scan_duration_ms: u64,
    pub completed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ThreatLevel {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ThreatCategory {
    Injection,
    DataExposure,
    AuthBypass,
    ConfigHardening,
    DependencyVuln,
    SecretExposure,
    RateLimit,
    Cors,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, PartialOrd)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactReport {
    pub score: f32, // 0.0 (safe) to 1.0 (critical)
    pub symbols_affected: usize,
    pub dependent_files: Vec<String>,
    pub contracts_affected: Vec<String>,
    pub risk_level: RiskLevel,
    pub recommendation: String,
}
