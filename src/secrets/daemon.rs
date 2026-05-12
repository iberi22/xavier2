use super::store::SecretStore;
use super::SecretError;
use super::SecretResult;
use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct Grant {
    pub id: String,
    pub key: String,
    pub permissions: HashSet<String>,
    pub expires_at: u64,
}

pub struct SecretDaemon {
    store: Box<dyn SecretStore>,
    active_grants: Vec<Grant>,
}

impl SecretDaemon {
    pub fn new(store: Box<dyn SecretStore>) -> Self {
        Self {
            store,
            active_grants: Vec::new(),
        }
    }

    pub async fn get_secret(&self, key: &str, grant_id: &str) -> SecretResult<String> {
        let grant = self
            .active_grants
            .iter()
            .find(|g| g.id == grant_id && g.key == key)
            .ok_or_else(|| {
                SecretError::ApprovalDenied(format!(
                    "No valid grant found for key {} with id {}",
                    key, grant_id
                ))
            })?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("test assertion")
            .as_secs();

        if now > grant.expires_at {
            return Err(SecretError::ApprovalDenied("Grant expired".to_string()));
        }

        if !grant.permissions.contains("read") {
            return Err(SecretError::ApprovalDenied(
                "Read permission missing in grant".to_string(),
            ));
        }

        self.store.get(key).await
    }

    pub async fn set_secret(&self, key: &str, value: &str) -> SecretResult<()> {
        // High-stakes action as per System 3 oversight
        log_high_stakes_action("SET_SECRET", key);
        self.store.set(key, value).await
    }

    pub fn issue_grant(
        &mut self,
        key: &str,
        permissions: HashSet<String>,
        ttl_secs: u64,
    ) -> String {
        let id = ulid::Ulid::new().to_string();
        let expires_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("test assertion")
            .as_secs()
            + ttl_secs;

        self.active_grants.push(Grant {
            id: id.clone(),
            key: key.to_string(),
            permissions,
            expires_at,
        });

        id
    }
}

fn log_high_stakes_action(action: &str, key: &str) {
    println!("⚠️  HIGH STAKES ACTION DETECTED IN DAEMON");
    println!("Action: {}", action);
    println!("Key: {}", key);
    println!("Context: System 3 Overseer notified.");
}
