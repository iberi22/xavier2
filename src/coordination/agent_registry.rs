//! Agent Registry - Track and manage active agents with heartbeats.
//!
//! Provides a simple in-memory registry for agents to:
//! - Register with a session ID
//! - Send heartbeats to indicate liveness
//! - Query active agents (heartbeat < 5 minutes)
//! - Store/retrieve agent context in memory

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Active agent entry
#[derive(Debug, Clone)]
pub struct AgentEntry {
    pub agent_id: String,
    pub session_id: String,
    pub last_heartbeat: DateTime<Utc>,
    pub metadata: AgentMetadata,
}

/// Additional agent metadata
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentMetadata {
    pub name: Option<String>,
    pub capabilities: Vec<String>,
    pub role: Option<String>,
}

const HEARTBEAT_TIMEOUT_SECS: i64 = 300; // 5 minutes

/// Agent registry for tracking active agents
#[derive(Default)]
pub struct SimpleAgentRegistry {
    agents: RwLock<HashMap<String, AgentEntry>>,
}

impl SimpleAgentRegistry {
    /// Create a new registry
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Register a new agent
    pub async fn register(&self, agent_id: String, session_id: String, metadata: AgentMetadata) -> bool {
        let now = Utc::now();
        let mut agents = self.agents.write().await;
        
        let entry = AgentEntry {
            agent_id: agent_id.clone(),
            session_id,
            last_heartbeat: now,
            metadata,
        };
        
        agents.insert(agent_id, entry);
        true
    }

    /// Unregister an agent
    pub async fn unregister(&self, agent_id: &str) -> bool {
        let mut agents = self.agents.write().await;
        agents.remove(agent_id).is_some()
    }

    /// Update heartbeat for an agent
    pub async fn heartbeat(&self, agent_id: &str) -> bool {
        let mut agents = self.agents.write().await;
        if let Some(entry) = agents.get_mut(agent_id) {
            entry.last_heartbeat = Utc::now();
            true
        } else {
            false
        }
    }

    /// Get all active agents (heartbeat < 5 minutes old)
    pub async fn get_active_agents(&self) -> Vec<AgentEntry> {
        let now = Utc::now();
        let agents = self.agents.read().await;
        
        agents
            .values()
            .filter(|entry| {
                let age = now.signed_duration_since(entry.last_heartbeat);
                age.num_seconds() < HEARTBEAT_TIMEOUT_SECS
            })
            .cloned()
            .collect()
    }

    /// Get a specific agent
    pub async fn get(&self, agent_id: &str) -> Option<AgentEntry> {
        let agents = self.agents.read().await;
        agents.get(agent_id).cloned()
    }

    /// List all registered agent IDs
    pub async fn list_ids(&self) -> Vec<String> {
        let agents = self.agents.read().await;
        agents.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_and_heartbeat() {
        let registry = SimpleAgentRegistry::new();
        
        // Register an agent
        let meta = AgentMetadata {
            name: Some("test-agent".to_string()),
            capabilities: vec!["coding".to_string()],
            role: Some("worker".to_string()),
        };
        
        let result = registry.register("agent-1".to_string(), "session-abc".to_string(), meta).await;
        assert!(result);
        
        // Get active agents
        let active = registry.get_active_agents().await;
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].agent_id, "agent-1");
        assert_eq!(active[0].session_id, "session-abc");
        
        // Heartbeat
        let result = registry.heartbeat("agent-1").await;
        assert!(result);
        
        // Unregister
        let result = registry.unregister("agent-1").await;
        assert!(result);
        
        let active = registry.get_active_agents().await;
        assert!(active.is_empty());
    }

    #[tokio::test]
    async fn test_get_active_agents_filters_stale() {
        let registry = SimpleAgentRegistry::new();
        
        let meta = AgentMetadata::default();
        registry.register("agent-1".to_string(), "s1".to_string(), meta.clone()).await;
        
        // Add another agent
        registry.register("agent-2".to_string(), "s2".to_string(), meta.clone()).await;
        
        let active = registry.get_active_agents().await;
        assert_eq!(active.len(), 2);
        
        // Manually expire one agent (modify its heartbeat in the map)
        {
            let mut agents = registry.agents.write().await;
            if let Some(entry) = agents.get_mut("agent-1") {
                entry.last_heartbeat = Utc::now() - chrono::Duration::seconds(400);
            }
        }
        
        let active = registry.get_active_agents().await;
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].agent_id, "agent-2");
    }
}