//! Plugin system for Xavier Enterprise
//!
//! Provides a trait-based plugin architecture for integrating external systems
//! like Cortex Enterprise Cloud.

use async_trait::async_trait;

pub mod cortex;
pub mod pgheart;
use std::future::Future;
use std::pin::Pin;

/// Boxed future type for async trait methods
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Direction for synchronization operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncDirection {
    /// Push local data to external system
    Push,
    /// Pull data from external system to local
    Pull,
    /// Bidirectional sync
    Both,
}

/// Result of a sync operation
#[derive(Debug, Clone)]
pub struct SyncResult {
    /// Number of items synced
    pub items_synced: usize,
    /// Whether the sync was successful
    pub success: bool,
    /// Optional error message if sync failed
    pub error: Option<String>,
    /// Timestamp of the sync (ms since epoch)
    pub timestamp_ms: u64,
}

impl SyncResult {
    /// Create a successful sync result
    pub fn success(items_synced: usize) -> Self {
        Self {
            items_synced,
            success: true,
            error: None,
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }

    /// Create a failed sync result
    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            items_synced: 0,
            success: false,
            error: Some(error.into()),
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }
}

/// Core trait for Xavier plugins
///
/// Plugins can integrate external systems for sync, storage, or other capabilities.
/// This trait is implemented for Cortex Enterprise and can be extended for other integrations.
#[async_trait]
pub trait Plugin: Send + Sync {
    /// Returns the unique name of the plugin
    fn name(&self) -> &str;

    /// Performs a health check on the plugin connection
    ///
    /// Returns Ok(()) if healthy, or an error message describing the issue.
    async fn health_check(&self) -> Result<(), String>;

    /// Performs a sync operation in the specified direction
    ///
    /// # Arguments
    /// * `direction` - The direction of sync (Push, Pull, or Both)
    async fn sync(&self, direction: SyncDirection) -> Result<SyncResult, String>;

    /// Returns true if the plugin is configured and ready to use
    fn is_configured(&self) -> bool {
        true
    }

    /// Returns the plugin version
    fn version(&self) -> &str {
        "0.1.0"
    }
}

/// Plugin registry for managing multiple plugins
#[derive(Default)]
pub struct PluginRegistry {
    plugins: Vec<Box<dyn Plugin>>,
}

impl PluginRegistry {
    /// Create a new empty plugin registry
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    /// Register a plugin
    pub fn register(&mut self, plugin: Box<dyn Plugin>) {
        tracing::info!("Registering plugin: {}", plugin.name());
        self.plugins.push(plugin);
    }

    /// Get all registered plugins
    pub fn plugins(&self) -> &[Box<dyn Plugin>] {
        &self.plugins
    }

    /// Get a plugin by name
    pub fn get(&self, name: &str) -> Option<&dyn Plugin> {
        self.plugins
            .iter()
            .find(|p| p.name() == name)
            .map(|p| p.as_ref())
    }

    /// Check health of all plugins
    pub async fn health_check_all(&self) -> Vec<(String, Result<(), String>)> {
        let mut results = Vec::new();
        for plugin in &self.plugins {
            let name = plugin.name().to_string();
            let result = plugin.health_check().await;
            results.push((name, result));
        }
        results
    }

    /// Get plugin names
    pub fn plugin_names(&self) -> Vec<String> {
        self.plugins.iter().map(|p| p.name().to_string()).collect()
    }

    /// Returns the number of registered plugins
    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    /// Returns true if no plugins are registered
    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }
}

impl std::fmt::Debug for PluginRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginRegistry")
            .field("plugin_count", &self.plugins.len())
            .field("plugins", &self.plugin_names())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestPlugin {
        name: String,
        healthy: bool,
    }

    #[async_trait]
    impl Plugin for TestPlugin {
        fn name(&self) -> &str {
            &self.name
        }

        async fn health_check(&self) -> Result<(), String> {
            if self.healthy {
                Ok(())
            } else {
                Err("Test plugin unhealthy".to_string())
            }
        }

        async fn sync(&self, _direction: SyncDirection) -> Result<SyncResult, String> {
            Ok(SyncResult::success(0))
        }
    }

    #[test]
    fn plugin_registry_empty_by_default() {
        let registry = PluginRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn plugin_registry_can_register_plugins() {
        let mut registry = PluginRegistry::new();
        let plugin = TestPlugin {
            name: "test".to_string(),
            healthy: true,
        };
        registry.register(Box::new(plugin));
        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());
    }

    #[test]
    fn plugin_registry_can_get_plugin_by_name() {
        let mut registry = PluginRegistry::new();
        let plugin = TestPlugin {
            name: "test-plugin".to_string(),
            healthy: true,
        };
        registry.register(Box::new(plugin));

        let retrieved = registry.get("test-plugin");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name(), "test-plugin");

        assert!(registry.get("nonexistent").is_none());
    }

    #[tokio::test]
    async fn plugin_registry_health_check_all() {
        let mut registry = PluginRegistry::new();

        let healthy_plugin = TestPlugin {
            name: "healthy".to_string(),
            healthy: true,
        };
        let unhealthy_plugin = TestPlugin {
            name: "unhealthy".to_string(),
            healthy: false,
        };

        registry.register(Box::new(healthy_plugin));
        registry.register(Box::new(unhealthy_plugin));

        let results = registry.health_check_all().await;
        assert_eq!(results.len(), 2);

        let healthy_result = results.iter().find(|(n, _)| n == "healthy").unwrap();
        assert!(healthy_result.1.is_ok());

        let unhealthy_result = results.iter().find(|(n, _)| n == "unhealthy").unwrap();
        assert!(unhealthy_result.1.is_err());
    }

    #[test]
    fn sync_result_success() {
        let result = SyncResult::success(42);
        assert!(result.success);
        assert_eq!(result.items_synced, 42);
        assert!(result.error.is_none());
        assert!(result.timestamp_ms > 0);
    }

    #[test]
    fn sync_result_failure() {
        let result = SyncResult::failure("test error");
        assert!(!result.success);
        assert_eq!(result.items_synced, 0);
        assert_eq!(result.error, Some("test error".to_string()));
        assert!(result.timestamp_ms > 0);
    }
}
