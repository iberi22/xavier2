use crate::domain::change_control::{LeaseRequest, LeaseResponse, TaskCompletionRequest};
use crate::memory::schema::{MemoryKind, MemoryQueryFilters};
use crate::memory::store::MemoryRecord;
use crate::ports::inbound::{ChangeControlPort, MemoryQueryPort};
use async_trait::async_trait;
use std::sync::Arc;
use ulid::Ulid;

pub struct ChangeControlService {
    memory: Arc<dyn MemoryQueryPort>,
}

impl ChangeControlService {
    pub fn new(memory: Arc<dyn MemoryQueryPort>) -> Self {
        Self { memory }
    }
}

#[async_trait]
impl ChangeControlPort for ChangeControlService {
    async fn claim_lease(&self, request: LeaseRequest) -> anyhow::Result<LeaseResponse> {
        let mut context = Vec::new();

        // 1. Search relevant decisions for affected files
        for file in &request.files_affected {
            let filters = MemoryQueryFilters {
                kinds: Some(vec![MemoryKind::Decision]),
                file_path: Some(file.clone()),
                ..Default::default()
            };
            let results = self.memory.search(file, Some(filters)).await?;
            for res in results {
                context.push(serde_json::to_value(res)?);
            }
        }

        // 2. Search agent_change_summary for similar past tasks
        let summary_filters = MemoryQueryFilters {
            kinds: Some(vec![MemoryKind::AgentChangeSummary]),
            ..Default::default()
        };
        let similar_summaries = self.memory.search(&request.task_description, Some(summary_filters)).await?;
        for res in similar_summaries {
            context.push(serde_json::to_value(res)?);
        }

        // 3. Search conflict history
        let conflict_filters = MemoryQueryFilters {
            kinds: Some(vec![MemoryKind::AgentChangeSummary]),
            ..Default::default()
        };
        let conflicts = self.memory.search("conflict", Some(conflict_filters)).await?;
        for res in conflicts {
            context.push(serde_json::to_value(res)?);
        }

        Ok(LeaseResponse {
            lease_id: Ulid::new().to_string(),
            memory_context: context,
        })
    }

    async fn complete_task(&self, request: TaskCompletionRequest) -> anyhow::Result<String> {
        let metadata = serde_json::json!({
            "type": "agent_change_summary",
            "task_id": request.task_id,
            "agent_id": request.agent_id,
            "files_changed": request.files_changed,
            "contracts_affected": request.contracts_affected,
            "risk_level": request.risk_level,
            "checks_passed": request.checks_passed,
            "pr": request.pr_link,
        });

        let record = MemoryRecord {
            id: String::new(),
            workspace_id: "default".to_string(), // TODO: Get workspace_id from context
            path: request.path,
            content: request.content,
            metadata,
            embedding: Vec::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            revision: 1,
            primary: true,
            parent_id: None,
            cluster_id: None,
            level: crate::memory::schema::MemoryLevel::Raw,
            relation: None,
            revisions: Vec::new(),
        };

        self.memory.add(record).await
    }
}
