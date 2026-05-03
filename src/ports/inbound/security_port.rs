use crate::domain::security::{ScanResult, ThreatLevel};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecureInputResult {
    pub allowed: bool,
    pub sanitized_input: Option<String>,
    pub original_input: String,
    pub detection_confidence: f32,
    pub is_injection: bool,
    pub attack_type: String,
}

#[async_trait]
pub trait SecurityScanPort: Send + Sync {
    async fn scan(&self, target: &str, level: Option<ThreatLevel>) -> anyhow::Result<ScanResult>;
    async fn get_config(&self) -> anyhow::Result<serde_json::Value>;
    async fn process_input(&self, input: &str) -> anyhow::Result<SecureInputResult>;
}
