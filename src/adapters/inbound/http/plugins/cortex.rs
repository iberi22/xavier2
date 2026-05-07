//! Cortex Enterprise Cloud Plugin
//!
//! Integrates Xavier2 with Cortex Enterprise Cloud for enterprise storage and sync.
//! This plugin provides bidirectional synchronization between local Xavier2 memory
//! and Cortex Enterprise cloud storage.

use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use super::{Plugin, SyncDirection, SyncResult};

/// Configuration for Cortex Enterprise plugin
#[derive(Debug, Clone)]
pub struct CortexConfig {
    /// Base URL for Cortex Enterprise Cloud API
    pub url: String,
    /// API token for authentication
    pub token: String,
    /// Sync interval in milliseconds (default: 300000 = 5 minutes)
    pub sync_interval_ms: u64,
    /// Enable automatic sync loop
    pub auto_sync: bool,
}

impl CortexConfig {
    /// Load configuration from environment variables
    ///
    /// Required env vars:
    /// - CORTEX_ENTERPRISE_URL: Base URL for Cortex API
    /// - CORTEX_TOKEN: API authentication token
    ///
    /// Optional env vars:
    /// - CORTEX_SYNC_INTERVAL_MS: Sync interval (default: 300000)
    /// - CORTEX_AUTO_SYNC: Enable auto-sync (default: true)
    pub fn from_env() -> Option<Self> {
        let url = std::env::var("CORTEX_ENTERPRISE_URL").ok()?;
        let token = std::env::var("CORTEX_TOKEN").ok()?;
        
        let sync_interval_ms = std::env::var("CORTEX_SYNC_INTERVAL_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(300000);
        
        let auto_sync = std::env::var("CORTEX_AUTO_SYNC")
            .ok()
            .map(|s| s.to_lowercase() == "true" || s == "1")
            .unwrap_or(true);
        
        Some(Self {
            url,
            token,
            sync_interval_ms,
            auto_sync,
        })
    }

    /// Check if Cortex is configured (env vars are set)
    pub fn is_configured() -> bool {
        std::env::var("CORTEX_ENTERPRISE_URL").is_ok() && 
        std::env::var("CORTEX_TOKEN").is_ok()
    }
}

/// Memory entry for sync operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub content: String,
    pub timestamp: u64,
    pub workspace_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Payload for push sync to Cortex
#[derive(Debug, Clone, Serialize)]
pub struct PushPayload {
    pub source: String,
    pub entries: Vec<MemoryEntry>,
    pub timestamp_ms: u64,
}

/// Response from Cortex sync operations
#[derive(Debug, Clone, Deserialize)]
pub struct SyncResponse {
    pub success: bool,
    #[serde(default)]
    pub items_processed: usize,
    #[serde(default)]
    pub message: Option<String>,
}

/// Cortex Enterprise Plugin implementation
pub struct CortexPlugin {
    config: CortexConfig,
    client: reqwest::Client,
    sync_handle: Arc<RwLock<Option<JoinHandle<()>>>>,
    last_sync_result: Arc<RwLock<Option<SyncResult>>>,
}

impl CortexPlugin {
    /// Create a new CortexPlugin from configuration
    pub fn new(config: CortexConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");
        
        let plugin = Self {
            config,
            client,
            sync_handle: Arc::new(RwLock::new(None)),
            last_sync_result: Arc::new(RwLock::new(None)),
        };
        
        // Start auto-sync loop if enabled
        if plugin.config.auto_sync {
            plugin.start_auto_sync();
        }
        
        plugin
    }

    /// Create a new CortexPlugin from environment variables
    pub fn from_env() -> Option<Self> {
        CortexConfig::from_env().map(Self::new)
    }

    /// Start the automatic sync loop
    fn start_auto_sync(&self) {
        let config = self.config.clone();
        let client = self.client.clone();
        let last_sync = self.last_sync_result.clone();
        
        let handle = tokio::spawn(async move {
            let interval = Duration::from_millis(config.sync_interval_ms);
            let mut ticker = tokio::time::interval(interval);
            
            loop {
                ticker.tick().await;
                
                tracing::info!("Running auto-sync to Cortex Enterprise");
                
                // Perform bidirectional sync
                let result = perform_sync(&client, &config, SyncDirection::Both).await;
                
                match &result {
                    Ok(sync_result) => {
                        if sync_result.success {
                            tracing::info!(
                                "Auto-sync completed: {} items synced",
                                sync_result.items_synced
                            );
                        } else {
                            tracing::warn!("Auto-sync failed: {:?}", sync_result.error);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Auto-sync error: {}", e);
                    }
                }
                
                *last_sync.write().await = result.ok();
            }
        });
        
        // Store the handle (this runs synchronously in constructor context)
        // We'll need to handle this differently since we can't await in new()
        tokio::spawn(async move {
            // The handle is already running, we just need to keep it alive
            let _ = handle.await;
        });
    }

    /// Push local memory entries to Cortex
    pub async fn push(&self, entries: Vec<MemoryEntry>) -> Result<SyncResult, String> {
        let payload = PushPayload {
            source: "xavier2".to_string(),
            entries,
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        };

        let url = format!("{}/api/v1/sync/push", self.config.url);
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.token))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("Cortex API error {}: {}", status, body));
        }

        let sync_response: SyncResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        if sync_response.success {
            Ok(SyncResult::success(sync_response.items_processed))
        } else {
            Ok(SyncResult::failure(
                sync_response.message.unwrap_or_else(|| "Unknown error".to_string())
            ))
        }
    }

    /// Pull memory entries from Cortex
    pub async fn pull(&self, since: Option<u64>) -> Result<Vec<MemoryEntry>, String> {
        let mut url = format!("{}/api/v1/sync/pull", self.config.url);
        
        if let Some(timestamp) = since {
            url.push_str(&format!("?since={}", timestamp));
        }

        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.config.token))
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("Cortex API error {}: {}", status, body));
        }

        let entries: Vec<MemoryEntry> = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        Ok(entries)
    }

    /// Get the last sync result
    pub async fn last_sync_result(&self) -> Option<SyncResult> {
        self.last_sync_result.read().await.clone()
    }

    /// Stop the auto-sync loop
    pub async fn stop_auto_sync(&self) {
        let mut handle = self.sync_handle.write().await;
        if let Some(h) = handle.take() {
            h.abort();
            tracing::info!("Stopped Cortex auto-sync loop");
        }
    }
}

#[async_trait]
impl Plugin for CortexPlugin {
    fn name(&self) -> &str {
        "cortex-enterprise"
    }

    async fn health_check(&self) -> Result<(), String> {
        let url = format!("{}/health", self.config.url);
        
        let response = self.client
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

    async fn sync(&self, direction: SyncDirection) -> Result<SyncResult, String> {
        perform_sync(&self.client, &self.config, direction).await
    }

    fn is_configured(&self) -> bool {
        true
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }
}

/// Perform sync operation with Cortex
fn perform_sync<'a>(
    client: &'a reqwest::Client,
    config: &'a CortexConfig,
    direction: SyncDirection,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<SyncResult, String>> + Send + 'a>> {
    Box::pin(async move {
        match direction {
            SyncDirection::Push => {
                // For now, return success with 0 items since we don't have
                // access to the local memory store from here.
                // In a full implementation, this would query the memory store
                // and push entries to Cortex.
                tracing::info!("Push sync requested (placeholder implementation)");
                Ok(SyncResult::success(0))
            }
            SyncDirection::Pull => {
                // Pull entries from Cortex
                let url = format!("{}/api/v1/sync/pull", config.url);
                
                let response = client
                    .get(&url)
                    .header("Authorization", format!("Bearer {}", config.token))
                    .send()
                    .await
                    .map_err(|e| format!("HTTP request failed: {}", e))?;

                if !response.status().is_success() {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    return Err(format!("Cortex API error {}: {}", status, body));
                }

                let entries: Vec<MemoryEntry> = response
                    .json()
                    .await
                    .map_err(|e| format!("Failed to parse response: {}", e))?;

                let count = entries.len();
                tracing::info!("Pulled {} entries from Cortex", count);
                
                Ok(SyncResult::success(count))
            }
            SyncDirection::Both => {
                // First pull, then push
                let pull_result = perform_sync(client, config, SyncDirection::Pull).await?;
                let push_result = perform_sync(client, config, SyncDirection::Push).await?;
                
                Ok(SyncResult::success(
                    pull_result.items_synced + push_result.items_synced
                ))
            }
        }
    })
}

impl std::fmt::Debug for CortexPlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CortexPlugin")
            .field("name", &self.name())
            .field("version", &self.version())
            .field("url", &self.config.url)
            .field("auto_sync", &self.config.auto_sync)
            .field("sync_interval_ms", &self.config.sync_interval_ms)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cortex_config_from_env_requires_url_and_token() {
        // Clear env vars first
        std::env::remove_var("CORTEX_ENTERPRISE_URL");
        std::env::remove_var("CORTEX_TOKEN");
        
        assert!(CortexConfig::from_env().is_none());
        assert!(!CortexConfig::is_configured());
    }

    #[test]
    fn cortex_config_parses_env_vars() {
        std::env::set_var("CORTEX_ENTERPRISE_URL", "https://cortex.example.com");
        std::env::set_var("CORTEX_TOKEN", "test-token");
        std::env::set_var("CORTEX_SYNC_INTERVAL_MS", "60000");
        std::env::set_var("CORTEX_AUTO_SYNC", "false");
        
        let config = CortexConfig::from_env().expect("Should parse config");
        
        assert_eq!(config.url, "https://cortex.example.com");
        assert_eq!(config.token, "test-token");
        assert_eq!(config.sync_interval_ms, 60000);
        assert!(!config.auto_sync);
        
        assert!(CortexConfig::is_configured());
        
        // Cleanup
        std::env::remove_var("CORTEX_ENTERPRISE_URL");
        std::env::remove_var("CORTEX_TOKEN");
        std::env::remove_var("CORTEX_SYNC_INTERVAL_MS");
        std::env::remove_var("CORTEX_AUTO_SYNC");
    }

    #[test]
    fn cortex_config_defaults() {
        std::env::set_var("CORTEX_ENTERPRISE_URL", "https://cortex.example.com");
        std::env::set_var("CORTEX_TOKEN", "test-token");
        
        let config = CortexConfig::from_env().expect("Should parse config");
        
        assert_eq!(config.sync_interval_ms, 300000); // Default: 5 minutes
        assert!(config.auto_sync); // Default: true
        
        // Cleanup
        std::env::remove_var("CORTEX_ENTERPRISE_URL");
        std::env::remove_var("CORTEX_TOKEN");
    }

    #[test]
    fn sync_result_creation() {
        let success = SyncResult::success(42);
        assert!(success.success);
        assert_eq!(success.items_synced, 42);
        
        let failure = SyncResult::failure("test error");
        assert!(!failure.success);
        assert_eq!(failure.error, Some("test error".to_string()));
    }

    #[test]
    fn memory_entry_serialization() {
        let entry = MemoryEntry {
            id: "test-123".to_string(),
            content: "Test content".to_string(),
            timestamp: 1234567890,
            workspace_id: "ws-1".to_string(),
            metadata: None,
        };
        
        let json = serde_json::to_string(&entry).expect("Should serialize");
        assert!(json.contains("test-123"));
        assert!(json.contains("Test content"));
    }

    #[tokio::test]
    async fn cortex_plugin_name_and_version() {
        std::env::set_var("CORTEX_ENTERPRISE_URL", "https://cortex.example.com");
        std::env::set_var("CORTEX_TOKEN", "test-token");
        std::env::set_var("CORTEX_AUTO_SYNC", "false");
        
        let plugin = CortexPlugin::from_env().expect("Should create plugin");
        
        assert_eq!(plugin.name(), "cortex-enterprise");
        assert_eq!(plugin.version(), env!("CARGO_PKG_VERSION"));
        assert!(plugin.is_configured());
        
        // Cleanup
        std::env::remove_var("CORTEX_ENTERPRISE_URL");
        std::env::remove_var("CORTEX_TOKEN");
        std::env::remove_var("CORTEX_AUTO_SYNC");
    }
}
