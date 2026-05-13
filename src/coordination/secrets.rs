use std::collections::HashMap;
use std::sync::Arc;
use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;
use anyhow::{Result, anyhow};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretLease {
    pub token: String,
    pub secret_name: String,
    pub secret_value: String,
    pub agent_id: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl SecretLease {
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

pub struct KeyLendingEngine {
    leases: Arc<RwLock<HashMap<String, SecretLease>>>,
}

impl KeyLendingEngine {
    pub fn new() -> Self {
        Self {
            leases: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Lend a secret to an agent for a specific duration (TTL)
    pub async fn lend(&self, name: &str, value: &str, agent_id: &str, ttl_secs: u64) -> Result<SecretLease> {
        let token = Uuid::new_v4().to_string();
        let now = Utc::now();
        let expires_at = now + Duration::seconds(ttl_secs as i64);

        let lease = SecretLease {
            token: token.clone(),
            secret_name: name.to_string(),
            secret_value: value.to_string(),
            agent_id: agent_id.to_string(),
            expires_at,
            created_at: now,
        };

        let mut leases = self.leases.write().await;
        leases.insert(token, lease.clone());
        
        tracing::info!("Lent secret '{}' to agent '{}'. Lease token: {}", name, agent_id, lease.token);
        Ok(lease)
    }

    /// Revoke a lease immediately
    pub async fn revoke(&self, token: &str) -> Result<()> {
        let mut leases = self.leases.write().await;
        if leases.remove(token).is_some() {
            tracing::info!("Revoked secret lease: {}", token);
            Ok(())
        } else {
            Err(anyhow!("Lease token not found"))
        }
    }

    /// Revoke all leases for a specific agent
    pub async fn revoke_for_agent(&self, agent_id: &str) -> usize {
        let mut leases = self.leases.write().await;
        let initial_count = leases.len();
        leases.retain(|_, lease| lease.agent_id != agent_id);
        let removed = initial_count - leases.len();
        if removed > 0 {
            tracing::info!("Revoked {} leases for agent '{}'", removed, agent_id);
        }
        removed
    }

    /// Get lease details by token
    pub async fn get_lease(&self, token: &str) -> Option<SecretLease> {
        let leases = self.leases.read().await;
        leases.get(token).cloned()
    }

    /// List all active leases
    pub async fn list_leases(&self) -> Vec<SecretLease> {
        let leases = self.leases.read().await;
        leases.values().cloned().collect()
    }

    /// Cleanup expired leases
    pub async fn cleanup_expired(&self) -> usize {
        let mut leases = self.leases.write().await;
        let initial_count = leases.len();
        leases.retain(|_, lease| !lease.is_expired());
        initial_count - leases.len()
    }
}

impl Default for KeyLendingEngine {
    fn default() -> Self {
        Self::new()
    }
}
