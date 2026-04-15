use crate::domain::security::{ScanResult, ThreatLevel};
use async_trait::async_trait;

#[async_trait]
pub trait SecurityScanPort: Send + Sync {
    async fn scan(&self, target: &str, level: Option<ThreatLevel>) -> anyhow::Result<ScanResult>;
    async fn get_config(&self) -> anyhow::Result<serde_json::Value>;
}
