use std::sync::Arc;
use async_trait::async_trait;
use crate::domain::change_control::*;
use crate::ports::inbound::ChangeControlPort;
use crate::memory::store::MemoryStore;
use crate::coordination::AgentRegistry;
use crate::coordination::LeaseRegistry;
use anyhow::Result;

pub struct ChangeControlService {
    _memory_store: Arc<dyn MemoryStore>,
    agent_registry: Arc<AgentRegistry>,
    lease_registry: Arc<LeaseRegistry>,
}

impl ChangeControlService {
    pub fn new(
        memory_store: Arc<dyn MemoryStore>,
        agent_registry: Arc<AgentRegistry>,
        lease_registry: Arc<LeaseRegistry>,
    ) -> Self {
        Self {
            _memory_store: memory_store,
            agent_registry,
            lease_registry,
        }
    }
}

#[async_trait]
impl ChangeControlPort for ChangeControlService {
    async fn create_task(&self, task: AgentTask) -> Result<String> {
        // Validate agent
        if self.agent_registry.get(&task.agent_id).await.is_none() {
            return Err(anyhow::anyhow!("Agent {} not registered", task.agent_id));
        }

        // Persistence could be done via memory_store
        // For now, returning task.id
        Ok(task.id)
    }

    async fn get_task(&self, _id: &str) -> Result<Option<AgentTask>> {
        // Stub
        Ok(None)
    }

    async fn claim_lease(&self, request: LeaseRequest) -> Result<LeaseResponse> {
        // Validate agent
        if self.agent_registry.get(&request.agent_id).await.is_none() {
            return Err(anyhow::anyhow!("Agent {} not registered", request.agent_id));
        }

        self.lease_registry.claim_lease(request).await
    }

    async fn release_lease(&self, lease_id: &str) -> Result<()> {
        self.lease_registry.release_lease(lease_id).await
    }

    async fn active_leases(&self) -> Result<Vec<FileLease>> {
        Ok(self.lease_registry.get_active_leases().await)
    }

    async fn check_conflicts(&self, _task_id: &str) -> Result<Vec<ConflictReport>> {
        // Stub
        Ok(vec![])
    }

    async fn validate_change(&self, _task_id: &str) -> Result<ValidationReport> {
        // Stub
        Ok(ValidationReport { valid: true, errors: vec![] })
    }

    async fn complete_task(&self, task_id: &str) -> Result<TaskCompletionReport> {
        // Stub
        Ok(TaskCompletionReport {
            task_id: task_id.to_string(),
            summary: "Task completed successfully".to_string(),
        })
    }

    async fn merge_plan(&self) -> Result<MergePlan> {
        // Stub
        Ok(MergePlan {
            plan_id: "plan-123".to_string(),
            changes: vec![],
        })
    }
}
