use anyhow::{anyhow, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

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

use crate::secrets::lending::AuditLogger;

pub struct KeyLendingEngine {
    leases: Arc<RwLock<HashMap<String, SecretLease>>>,
    audit_logger: Box<dyn AuditLogger + Send + Sync>,
}

impl KeyLendingEngine {
    pub fn new(audit_logger: Box<dyn AuditLogger + Send + Sync>) -> Self {
        Self {
            leases: Arc::new(RwLock::new(HashMap::new())),
            audit_logger,
        }
    }

    /// Lend a secret to an agent for a specific duration (TTL)
    pub async fn lend(
        &self,
        name: &str,
        value: &str,
        agent_id: &str,
        ttl_secs: u64,
    ) -> Result<SecretLease> {
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
        leases.insert(token.clone(), lease.clone());

        self.audit_logger.log_lend(agent_id, name, &token, ttl_secs);
        tracing::info!(
            "Lent secret '{}' to agent '{}'. Lease token: {}",
            name,
            agent_id,
            lease.token
        );
        Ok(lease)
    }

    /// Revoke a lease immediately
    pub async fn revoke(&self, token: &str, reason: &str) -> Result<()> {
        let mut leases = self.leases.write().await;
        if let Some(lease) = leases.remove(token) {
            self.audit_logger.log_revoke(&lease.agent_id, token, reason);
            tracing::info!("Revoked secret lease: {} (Reason: {})", token, reason);
            Ok(())
        } else {
            Err(anyhow!("Lease token not found"))
        }
    }

    /// Revoke all leases for a specific agent
    pub async fn revoke_for_agent(&self, agent_id: &str, reason: &str) -> usize {
        let mut leases = self.leases.write().await;
        let mut tokens_to_remove = Vec::new();
        for (token, lease) in leases.iter() {
            if lease.agent_id == agent_id {
                tokens_to_remove.push(token.clone());
            }
        }

        let count = tokens_to_remove.len();
        for token in tokens_to_remove {
            leases.remove(&token);
            self.audit_logger.log_revoke(agent_id, &token, reason);
        }

        if count > 0 {
            tracing::info!(
                "Revoked {} leases for agent '{}' (Reason: {})",
                count,
                agent_id,
                reason
            );
        }
        count
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
        let mut tokens_to_remove = Vec::new();
        for (token, lease) in leases.iter() {
            if lease.is_expired() {
                tokens_to_remove.push(token.clone());
            }
        }

        let count = tokens_to_remove.len();
        for token in tokens_to_remove {
            if let Some(lease) = leases.remove(&token) {
                self.audit_logger
                    .log_revoke(&lease.agent_id, &token, "TTL Expired");
            }
        }
        count
    }
}
