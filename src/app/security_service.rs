use crate::domain::security::{ScanResult, ThreatLevel};
use crate::ports::inbound::SecurityScanPort;
use async_trait::async_trait;

pub struct SecurityService;

impl Default for SecurityService {
    fn default() -> Self {
        Self::new()
    }
}

impl SecurityService {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl SecurityScanPort for SecurityService {
    async fn scan(&self, target: &str, level: Option<ThreatLevel>) -> anyhow::Result<ScanResult> {
        let _ = target;
        let _ = level;
        todo!()
    }

    async fn get_config(&self) -> anyhow::Result<serde_json::Value> {
        todo!()
    }
}
