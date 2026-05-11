use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{Utc, Duration};
use crate::domain::change_control::{FileLease, LeaseRequest, LeaseResponse};

pub struct LeaseRegistry {
    leases: Arc<RwLock<HashMap<String, FileLease>>>,
}

impl LeaseRegistry {
    pub fn new() -> Self {
        Self {
            leases: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn claim_lease(&self, request: LeaseRequest) -> anyhow::Result<LeaseResponse> {
        let mut leases = self.leases.write().await;
        let now = Utc::now();

        // Cleanup expired leases
        leases.retain(|_, lease| lease.expires_at > now);

        // Check for conflicts via glob pattern intersection
        // For simplicity in this stub, we'll just check for exact path match
        for lease in leases.values() {
            if self.patterns_overlap(&lease.resource_path, &request.resource_path) {
                return Err(anyhow::anyhow!("Resource conflict with active lease: {}", lease.id));
            }
        }

        let lease_id = ulid::Ulid::new().to_string();
        let expires_at = now + Duration::seconds(request.duration_seconds as i64);

        let lease = FileLease {
            id: lease_id.clone(),
            agent_id: request.agent_id,
            resource_path: request.resource_path,
            expires_at,
        };

        leases.insert(lease_id.clone(), lease);

        Ok(LeaseResponse {
            lease_id,
            expires_at,
        })
    }

    pub async fn release_lease(&self, lease_id: &str) -> anyhow::Result<()> {
        let mut leases = self.leases.write().await;
        leases.remove(lease_id);
        Ok(())
    }

    pub async fn get_active_leases(&self) -> Vec<FileLease> {
        let mut leases = self.leases.write().await;
        let now = Utc::now();

        // Cleanup expired leases
        leases.retain(|_, lease| lease.expires_at > now);

        leases.values().cloned().collect()
    }

    fn patterns_overlap(&self, p1: &str, p2: &str) -> bool {
        // Basic glob-like overlap check: if one contains the other or they are equal
        // In a real implementation, this would use a glob library
        p1 == p2 || p1.contains(p2) || p2.contains(p1)
    }
}

impl Default for LeaseRegistry {
    fn default() -> Self {
        Self::new()
    }
}
