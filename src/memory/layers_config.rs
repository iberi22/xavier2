//! Memory Layers Configuration
//!
//! Provides configuration for the multi-layer memory system.
//! Configuration is loaded from environment variables with sensible defaults.
//!
//! # Environment Variables
//! - `XAVIER_WORKING_MEMORY_CAPACITY` - Max items in working memory (default: 100)
//! - `XAVIER_WORKING_LRU_THRESHOLD` - Access count for LRU exemption (default: 2)
//! - `XAVIER_WORKING_BM25_K1` - Working memory BM25 k1 parameter (default: 1.5)
//! - `XAVIER_WORKING_BM25_B` - Working memory BM25 b parameter (default: 0.75)
//! - `XAVIER_EPISODIC_SUMMARY_WINDOW` - Turns before episodic summary (default: 10)
//! - `XAVIER_MAX_EPISODIC_SESSIONS` - Max sessions in episodic memory (default: 50)
//! - `XAVIER_EPISODIC_MIN_EVENT_IMPORTANCE` - Minimum key event importance (default: 0.5)

use serde::{Deserialize, Serialize};

/// Memory layers configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryLayersConfig {
    /// Working memory layer configuration
    pub working: WorkingMemoryLayerConfig,
    /// Episodic memory layer configuration
    pub episodic: EpisodicMemoryLayerConfig,
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
    /// BM25 k1 parameter (default: 1.5)
    pub bm25_k1: f32,
    /// BM25 b parameter (default: 0.75)
    pub bm25_b: f32,
}

impl Default for WorkingMemoryLayerConfig {
    fn default() -> Self {
        let core = crate::memory::working::WorkingMemoryConfig::default();
        Self {
            capacity: core.capacity,
            lru_exempt_access_threshold: core.lru_exempt_access_threshold,
            bm25_k1: core.bm25_k1,
            bm25_b: core.bm25_b,
        }
    }
}

impl WorkingMemoryLayerConfig {
    /// Load configuration from environment variables
    /// Delegates to WorkingMemoryConfig::from_env() for shared parsing + validation.
    pub fn from_env() -> Self {
        let core = crate::memory::working::WorkingMemoryConfig::from_env();
        Self {
            capacity: core.capacity,
            lru_exempt_access_threshold: core.lru_exempt_access_threshold,
            bm25_k1: core.bm25_k1,
            bm25_b: core.bm25_b,
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
        let default = Self::default();
        Self {
            summary_window: std::env::var("XAVIER_EPISODIC_SUMMARY_WINDOW")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(default.summary_window),
            max_sessions: std::env::var("XAVIER_MAX_EPISODIC_SESSIONS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(default.max_sessions),
            min_event_importance: std::env::var("XAVIER_EPISODIC_MIN_EVENT_IMPORTANCE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(default.min_event_importance),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{LazyLock, Mutex};

    static ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    #[test]
    fn test_default_config() {
        let config = MemoryLayersConfig::default();
        assert_eq!(config.working.capacity, 100);
        assert_eq!(config.episodic.max_sessions, 50);
    }

    #[test]
    fn test_working_layer_config_from_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("XAVIER_WORKING_MEMORY_CAPACITY", "200");
        std::env::set_var("XAVIER_WORKING_LRU_THRESHOLD", "5");
        std::env::set_var("XAVIER_WORKING_BM25_K1", "2.0");
        std::env::set_var("XAVIER_WORKING_BM25_B", "0.5");

        let config = WorkingMemoryLayerConfig::from_env();
        assert_eq!(config.capacity, 200);
        assert_eq!(config.lru_exempt_access_threshold, 5);
        assert!((config.bm25_k1 - 2.0).abs() < 0.01);
        assert!((config.bm25_b - 0.5).abs() < 0.01);

        std::env::remove_var("XAVIER_WORKING_MEMORY_CAPACITY");
        std::env::remove_var("XAVIER_WORKING_LRU_THRESHOLD");
        std::env::remove_var("XAVIER_WORKING_BM25_K1");
        std::env::remove_var("XAVIER_WORKING_BM25_B");
    }

    #[test]
    fn test_episodic_layer_config_from_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("XAVIER_MAX_EPISODIC_SESSIONS", "100");
        std::env::set_var("XAVIER_EPISODIC_SUMMARY_WINDOW", "20");
        std::env::set_var("XAVIER_EPISODIC_MIN_EVENT_IMPORTANCE", "0.7");

        let config = EpisodicMemoryLayerConfig::from_env();
        assert_eq!(config.max_sessions, 100);
        assert_eq!(config.summary_window, 20);
        assert!((config.min_event_importance - 0.7).abs() < 0.01);

        std::env::remove_var("XAVIER_MAX_EPISODIC_SESSIONS");
        std::env::remove_var("XAVIER_EPISODIC_SUMMARY_WINDOW");
        std::env::remove_var("XAVIER_EPISODIC_MIN_EVENT_IMPORTANCE");
    }

    #[test]
    fn test_memory_layers_config_from_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("XAVIER_WORKING_MEMORY_CAPACITY", "150");
        std::env::set_var("XAVIER_MAX_EPISODIC_SESSIONS", "75");

        let config = MemoryLayersConfig::from_env();
        assert_eq!(config.working.capacity, 150);
        assert_eq!(config.episodic.max_sessions, 75);

        std::env::remove_var("XAVIER_WORKING_MEMORY_CAPACITY");
        std::env::remove_var("XAVIER_MAX_EPISODIC_SESSIONS");
    }
}
