use async_trait::async_trait;

#[async_trait]
pub trait ThreatDetectionPort: Send + Sync {
    /// Scans the given text for security threats and logs them to the audit chain.
    /// Returns true if the content is clean, false if a threat was detected.
    async fn scan_and_log(&self, text: &str, component: &str) -> anyhow::Result<bool>;
}
