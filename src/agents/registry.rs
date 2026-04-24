use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCard {
    pub id: String,
    pub name: String,
    pub capabilities: Vec<String>,
}

pub struct AgentRegistry {
    agents: RwLock<HashMap<String, AgentCard>>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self {
            agents: RwLock::new(HashMap::new()),
        }
    }

    pub async fn register(&self, card: AgentCard) {
        let mut agents = self.agents.write().await;
        agents.insert(card.id.clone(), card);
    }

    pub async fn get_agent(&self, id: &str) -> Option<AgentCard> {
        let agents = self.agents.read().await;
        agents.get(id).cloned()
    }

    pub async fn list_agents(&self) -> Vec<AgentCard> {
        let agents = self.agents.read().await;
        agents.values().cloned().collect()
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}
