//! Memory Layers Configuration
//!
//! Provides configuration for the multi-layer memory system.
//! Configuration is loaded from environment variables with sensible defaults.
//!
//! # Environment Variables
//! - `XAVIER2_WORKING_MEMORY_CAPACITY` - Max items in working memory (default: 100)
//! - `XAVIER2_MAX_EPISODIC_SESSIONS` - Max sessions in episodic memory (default: 50)

use serde::{Deserialize, Serialize};

/// Memory layers configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryLayersConfig {
    /// Working memory layer configuration
    pub working: WorkingMemoryLayerConfig,
    /// Episodic memory layer configuration
    pub episodic: EpisodicMemoryLayerConfig,
}

impl Default for MemoryLayersConfig {
    fn default() -> Self {
        Self {
            working: WorkingMemoryLayerConfig::default(),
            episodic: EpisodicMemoryLayerConfig::default(),
        }
    }
}

impl MemoryLayersConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            working: WorkingMemoryLayerConfig::from_env(),
            episodic: EpisodicMemoryLayerConfig::from_env(),
        }
    }
}

/// Working memory layer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingMemoryLayerConfig {
    /// Maximum number of items in working memory (default: 100)
    pub capacity: usize,
    /// Minimum access count to qualify for LRU exemption (default: 2)
    pub lru_exempt_access_threshold: u32,
}

impl Default for WorkingMemoryLayerConfig {
    fn default() -> Self {
        Self {
            capacity: 100,
            lru_exempt_access_threshold: 2,
        }
    }
}

impl WorkingMemoryLayerConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            capacity: std::env::var("XAVIER2_WORKING_MEMORY_CAPACITY")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(100),
            lru_exempt_access_threshold: std::env::var("XAVIER2_WORKING_LRU_THRESHOLD")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(2),
        }
    }
}

/// Episodic memory layer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodicMemoryLayerConfig {
    /// Number of turns before generating a summary (default: 10)
    pub summary_window: usize,
    /// Maximum number of sessions to retain (default: 50)
    pub max_sessions: usize,
    /// Minimum importance score for key events (default: 0.5)
    pub min_event_importance: f32,
}

impl Default for EpisodicMemoryLayerConfig {
    fn default() -> Self {
        Self {
            summary_window: 10,
            max_sessions: 50,
            min_event_importance: 0.5,
        }
    }
}

impl EpisodicMemoryLayerConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            summary_window: std::env::var("XAVIER2_EPISODIC_SUMMARY_WINDOW")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
            max_sessions: std::env::var("XAVIER2_MAX_EPISODIC_SESSIONS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(50),
            min_event_importance: std::env::var("XAVIER2_EPISODIC_MIN_EVENT_IMPORTANCE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.5),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = MemoryLayersConfig::default();
        assert_eq!(config.working.capacity, 100);
        assert_eq!(config.episodic.max_sessions, 50);
    }

    #[test]
    fn test_working_layer_config_from_env() {
        std::env::set_var("XAVIER2_WORKING_MEMORY_CAPACITY", "200");
        std::env::set_var("XAVIER2_WORKING_LRU_THRESHOLD", "5");

        let config = WorkingMemoryLayerConfig::from_env();
        assert_eq!(config.capacity, 200);
        assert_eq!(config.lru_exempt_access_threshold, 5);

        std::env::remove_var("XAVIER2_WORKING_MEMORY_CAPACITY");
        std::env::remove_var("XAVIER2_WORKING_LRU_THRESHOLD");
    }

    #[test]
    fn test_episodic_layer_config_from_env() {
        std::env::set_var("XAVIER2_MAX_EPISODIC_SESSIONS", "100");
        std::env::set_var("XAVIER2_EPISODIC_SUMMARY_WINDOW", "20");
        std::env::set_var("XAVIER2_EPISODIC_MIN_EVENT_IMPORTANCE", "0.7");

        let config = EpisodicMemoryLayerConfig::from_env();
        assert_eq!(config.max_sessions, 100);
        assert_eq!(config.summary_window, 20);
        assert!((config.min_event_importance - 0.7).abs() < 0.01);

        std::env::remove_var("XAVIER2_MAX_EPISODIC_SESSIONS");
        std::env::remove_var("XAVIER2_EPISODIC_SUMMARY_WINDOW");
        std::env::remove_var("XAXIER2_EPISODIC_MIN_EVENT_IMPORTANCE");
    }

    #[test]
    fn test_memory_layers_config_from_env() {
        std::env::set_var("XAVIER2_WORKING_MEMORY_CAPACITY", "150");
        std::env::set_var("XAVIER2_MAX_EPISODIC_SESSIONS", "75");

        let config = MemoryLayersConfig::from_env();
        assert_eq!(config.working.capacity, 150);
        assert_eq!(config.episodic.max_sessions, 75);

        std::env::remove_var("XAVIER2_WORKING_MEMORY_CAPACITY");
        std::env::remove_var("XAVIER2_MAX_EPISODIC_SESSIONS");
    }
}
