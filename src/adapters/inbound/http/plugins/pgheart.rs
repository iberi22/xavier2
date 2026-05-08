//! PgHeart PostgreSQL Monitoring Plugin
//!
//! Integrates Xavier2 with PgHeart for PostgreSQL monitoring and heartbeat.
//! This plugin provides bidirectional synchronization between local Xavier2 memory
//! and PgHeart monitoring system.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Configuration for PgHeart plugin
#[derive(Debug, Clone)]
pub struct PgHeartConfig {
    /// PgHeart API base URL
    pub url: String,
    /// API token for authentication
    pub token: String,
    /// Instance ID to monitor
    pub instance_id: String,
    /// Sync interval in milliseconds
    pub sync_interval_ms: u64,
    /// Enable automatic heartbeat
    pub auto_heartbeat: bool,
}

impl PgHeartConfig {
    /// Load configuration from environment variables
    ///
    /// Required env vars:
    /// - PGHEART_URL: Base URL for PgHeart API
    /// - PGHEART_TOKEN: API authentication token
    /// - PGHEART_INSTANCE_ID: Instance ID to monitor
    ///
    /// Optional env vars:
    /// - PGHEART_SYNC_INTERVAL_MS: Sync interval (default: 60000 = 1 minute)
    /// - PGHEART_AUTO_HEARTBEAT: Enable auto-heartbeat (default: true)
    pub fn from_env() -> Option<Self> {
        let url = std::env::var("PGHEART_URL").ok()?;
        let token = std::env::var("PGHEART_TOKEN").ok()?;
        let instance_id = std::env::var("PGHEART_INSTANCE_ID").ok()?;

        let sync_interval_ms = std::env::var("PGHEART_SYNC_INTERVAL_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(60000);

        let auto_heartbeat = std::env::var("PGHEART_AUTO_HEARTBEAT")
            .ok()
            .map(|s| s.to_lowercase() == "true" || s == "1")
            .unwrap_or(true);

        Some(Self {
            url,
            token,
            instance_id,
            sync_interval_ms,
            auto_heartbeat,
        })
    }

    /// Check if PgHeart is configured (required env vars are set)
    pub fn is_configured() -> bool {
        std::env::var("PGHEART_URL").is_ok()
            && std::env::var("PGHEART_TOKEN").is_ok()
            && std::env::var("PGHEART_INSTANCE_ID").is_ok()
    }
}

/// Heartbeat entry for PgHeart
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatEntry {
    pub instance_id: String,
    pub status: String,
    pub timestamp_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Response from PgHeart API
#[derive(Debug, Clone, Deserialize)]
pub struct PgHeartResponse {
    pub success: bool,
    #[serde(default)]
    pub message: Option<String>,
}

/// PgHeart PostgreSQL Monitoring Plugin
pub struct PgHeartPlugin {
    config: PgHeartConfig,
    client: reqwest::Client,
    last_heartbeat: Arc<RwLock<Option<u64>>>,
}

impl PgHeartPlugin {
    /// Create a new PgHeartPlugin from configuration
    pub fn new(config: PgHeartConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            config,
            client,
            last_heartbeat: Arc::new(RwLock::new(None)),
        }
    }

    /// Create a new PgHeartPlugin from environment variables
    pub fn from_env() -> Option<Self> {
        PgHeartConfig::from_env().map(Self::new)
    }

    /// Record a heartbeat to PgHeart
    pub async fn heartbeat(&self, status: &str) -> Result<super::SyncResult, String> {
        let entry = HeartbeatEntry {
            instance_id: self.config.instance_id.clone(),
            status: status.to_string(),
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            metadata: None,
        };

        let url = format!("{}/api/v1/heartbeat", self.config.url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.token))
            .header("Content-Type", "application/json")
            .json(&entry)
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("PgHeart API error {}: {}", status, body));
        }

        let _sync_response: PgHeartResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        *self.last_heartbeat.write().await = Some(now);

        Ok(super::SyncResult::success(1))
    }

    /// Get the last heartbeat timestamp
    pub async fn last_heartbeat(&self) -> Option<u64> {
        self.last_heartbeat.read().await.clone()
    }
}

#[async_trait]
impl super::Plugin for PgHeartPlugin {
    fn name(&self) -> &str {
        "pgheart"
    }

    async fn health_check(&self) -> Result<(), String> {
        let url = format!("{}/health", self.config.url);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.config.token))
            .send()
            .await
            .map_err(|e| format!("Health check request failed: {}", e))?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(format!("Health check failed: HTTP {}", response.status()))
        }
    }

    async fn sync(&self, _direction: super::SyncDirection) -> Result<super::SyncResult, String> {
        self.heartbeat("synced").await
    }

    fn is_configured(&self) -> bool {
        true
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }
}

impl std::fmt::Debug for PgHeartPlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PgHeartPlugin")
            .field("name", &self.name())
            .field("version", &self.version())
            .field("url", &self.config.url)
            .field("instance_id", &self.config.instance_id)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pgheart_config_requires_all_vars() {
        std::env::remove_var("PGHEART_URL");
        std::env::remove_var("PGHEART_TOKEN");
        std::env::remove_var("PGHEART_INSTANCE_ID");

        assert!(PgHeartConfig::from_env().is_none());
        assert!(!PgHeartConfig::is_configured());
    }

    #[test]
    fn pgheart_config_parses_env_vars() {
        std::env::set_var("PGHEART_URL", "https://pgheart.example.com");
        std::env::set_var("PGHEART_TOKEN", "test-token");
        std::env::set_var("PGHEART_INSTANCE_ID", "instance-123");
        std::env::set_var("PGHEART_SYNC_INTERVAL_MS", "30000");
        std::env::set_var("PGHEART_AUTO_HEARTBEAT", "false");

        let config = PgHeartConfig::from_env().expect("Should parse config");

        assert_eq!(config.url, "https://pgheart.example.com");
        assert_eq!(config.token, "test-token");
        assert_eq!(config.instance_id, "instance-123");
        assert_eq!(config.sync_interval_ms, 30000);
        assert!(!config.auto_heartbeat);

        assert!(PgHeartConfig::is_configured());

        std::env::remove_var("PGHEART_URL");
        std::env::remove_var("PGHEART_TOKEN");
        std::env::remove_var("PGHEART_INSTANCE_ID");
        std::env::remove_var("PGHEART_SYNC_INTERVAL_MS");
        std::env::remove_var("PGHEART_AUTO_HEARTBEAT");
    }

    #[test]
    fn pgheart_config_defaults() {
        std::env::set_var("PGHEART_URL", "https://pgheart.example.com");
        std::env::set_var("PGHEART_TOKEN", "test-token");
        std::env::set_var("PGHEART_INSTANCE_ID", "instance-123");

        let config = PgHeartConfig::from_env().expect("Should parse config");

        assert_eq!(config.sync_interval_ms, 60000);
        assert!(config.auto_heartbeat);

        std::env::remove_var("PGHEART_URL");
        std::env::remove_var("PGHEART_TOKEN");
        std::env::remove_var("PGHEART_INSTANCE_ID");
    }
}
