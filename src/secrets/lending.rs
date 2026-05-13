use std::collections::HashMap;
use std::time::{SystemTime, Duration};
use uuid::Uuid;
use super::SecretError;

#[derive(Debug, Clone)]
pub struct EphemeralLease {
    pub session_token: String,
    pub real_secret_id: String,
    pub agent_id: String,
    pub expires_at: SystemTime,
}

pub trait AuditLogger {
    fn log_lend(&self, agent_id: &str, secret_id: &str, session_token: &str, ttl_secs: u64);
    fn log_revoke(&self, agent_id: &str, session_token: &str, reason: &str);
}

pub struct DefaultAuditLogger;

impl AuditLogger for DefaultAuditLogger {
    fn log_lend(&self, agent_id: &str, secret_id: &str, session_token: &str, ttl_secs: u64) {
        println!("[AUDIT] LENT secret '{}' to agent '{}' (Session: {}, TTL: {}s)", secret_id, agent_id, session_token, ttl_secs);
    }
    fn log_revoke(&self, agent_id: &str, session_token: &str, reason: &str) {
        println!("[AUDIT] REVOKED session '{}' for agent '{}' (Reason: {})", session_token, agent_id, reason);
    }
}

pub struct KeyLendingEngine<A: AuditLogger> {
    leases: HashMap<String, EphemeralLease>,
    audit_logger: A,
}

impl<A: AuditLogger> KeyLendingEngine<A> {
    pub fn new(audit_logger: A) -> Self {
        Self {
            leases: HashMap::new(),
            audit_logger,
        }
    }

    pub fn lend(&mut self, agent_id: &str, real_secret_id: &str, ttl_secs: u64) -> Result<String, SecretError> {
        let session_token = Uuid::new_v4().to_string();
        let expires_at = SystemTime::now() + Duration::from_secs(ttl_secs);

        let lease = EphemeralLease {
            session_token: session_token.clone(),
            real_secret_id: real_secret_id.to_string(),
            agent_id: agent_id.to_string(),
            expires_at,
        };

        self.leases.insert(session_token.clone(), lease);
        self.audit_logger.log_lend(agent_id, real_secret_id, &session_token, ttl_secs);

        Ok(session_token)
    }

    pub fn revoke(&mut self, session_token: &str, reason: &str) -> Result<(), SecretError> {
        if let Some(lease) = self.leases.remove(session_token) {
            self.audit_logger.log_revoke(&lease.agent_id, session_token, reason);
            Ok(())
        } else {
            Err(SecretError::NotFound(format!("Lease {} not found", session_token)))
        }
    }

    pub fn resolve(&self, session_token: &str) -> Result<String, SecretError> {
        if let Some(lease) = self.leases.get(session_token) {
            if SystemTime::now() > lease.expires_at {
                return Err(SecretError::ApprovalDenied("Session token expired".to_string()));
            }
            Ok(lease.real_secret_id.clone())
        } else {
            Err(SecretError::NotFound("Session token invalid or revoked".to_string()))
        }
    }
    
    pub fn cleanup_expired(&mut self) {
        let now = SystemTime::now();
        let mut expired_tokens = Vec::new();
        
        for (token, lease) in self.leases.iter() {
            if now > lease.expires_at {
                expired_tokens.push(token.clone());
            }
        }
        
        for token in expired_tokens {
            if let Some(lease) = self.leases.remove(&token) {
                self.audit_logger.log_revoke(&lease.agent_id, &token, "TTL Expired");
            }
        }
    }
}
