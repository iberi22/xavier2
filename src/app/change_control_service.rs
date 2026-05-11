use crate::domain::change_control::{
    ChangeLease, ChangeTask, ChangeTaskStatus, ConflictReport, LeaseClaimResponse, LeaseMode,
    MergePlan, ValidationResult,
};
use crate::ports::inbound::ChangeControlPort;
use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use uuid::Uuid;

pub struct ChangeControlService {
    tasks: RwLock<HashMap<String, ChangeTask>>,
    leases: RwLock<HashMap<String, ChangeLease>>,
}

impl ChangeControlService {
    pub fn new() -> Self {
        Self {
            tasks: RwLock::new(HashMap::new()),
            leases: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl ChangeControlPort for ChangeControlService {
    async fn create_task(
        &self,
        agent_id: String,
        title: String,
        intent: String,
        scope: Vec<String>,
    ) -> anyhow::Result<ChangeTask> {
        let id = Uuid::new_v4().to_string();
        let task = ChangeTask {
            id: id.clone(),
            agent_id,
            title,
            intent,
            scope,
            status: ChangeTaskStatus::Open,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            completed_at: None,
            result: None,
        };
        self.tasks.write().await.insert(id, task.clone());
        Ok(task)
    }

    async fn get_task(&self, id: String) -> anyhow::Result<Option<ChangeTask>> {
        Ok(self.tasks.read().await.get(&id).cloned())
    }

    async fn claim_lease(
        &self,
        agent_id: String,
        task_id: String,
        patterns: Vec<String>,
        mode: LeaseMode,
        ttl_seconds: u64,
    ) -> anyhow::Result<LeaseClaimResponse> {
        // Simple in-memory lease management (no real conflict detection for now)
        let lease_id = format!("lease_{}", Uuid::new_v4());
        let expires_at = Utc::now() + chrono::Duration::seconds(ttl_seconds as i64);

        let lease = ChangeLease {
            id: lease_id.clone(),
            agent_id,
            task_id,
            patterns,
            mode,
            expires_at,
        };

        self.leases.write().await.insert(lease_id.clone(), lease);

        Ok(LeaseClaimResponse {
            status: "granted".to_string(),
            lease_id: Some(lease_id),
            conflicts: vec![],
            memory_context: vec![],
            required_checks: vec!["cargo check".to_string()],
        })
    }

    async fn release_lease(&self, lease_id: String) -> anyhow::Result<bool> {
        Ok(self.leases.write().await.remove(&lease_id).is_some())
    }

    async fn get_active_leases(&self) -> anyhow::Result<Vec<ChangeLease>> {
        Ok(self.leases.read().await.values().cloned().collect())
    }

    async fn check_conflicts(&self, _task_id: String, _scope: Vec<String>) -> anyhow::Result<ConflictReport> {
        Ok(ConflictReport {
            has_conflicts: false,
            conflicts: vec![],
        })
    }

    async fn validate_task(&self, _task_id: String) -> anyhow::Result<ValidationResult> {
        Ok(ValidationResult {
            valid: true,
            errors: vec![],
        })
    }

    async fn complete_task(&self, task_id: String, result: String) -> anyhow::Result<ChangeTask> {
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(&task_id) {
            task.status = ChangeTaskStatus::Completed;
            task.result = Some(result);
            task.completed_at = Some(Utc::now());
            task.updated_at = Utc::now();
            Ok(task.clone())
        } else {
            Err(anyhow::anyhow!("Task not found"))
        }
    }

    async fn get_merge_plan(&self) -> anyhow::Result<MergePlan> {
        Ok(MergePlan {
            ready: true,
            strategies: vec!["fast-forward".to_string()],
        })
    }
}

impl Default for ChangeControlService {
    fn default() -> Self {
        Self::new()
    }
}
