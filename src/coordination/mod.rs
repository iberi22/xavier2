//! Coordination Module - Agent coordination and orchestration
//!
//! Provides:
//! - AgentRegistry: Track and manage registered agents
//! - AgentHandle: Reference to a remote agent
//! - CoordinationService: Orchestrate multiple agents
//!
//! Integrates with TaskService for task distribution

pub mod agent_registry;
pub mod message_bus;

pub use agent_registry::SimpleAgentRegistry;
pub use message_bus::*;

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use ulid::Ulid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    TaskAssigned,
    TaskCompleted,
    SystemShutdown,
    Test,
}

#[derive(Debug, Clone)]
pub struct CoordinationMessage {
    pub from: String,
    pub to: String,
    pub event: Event,
}

impl CoordinationMessage {
    pub fn new(from: String, to: String, event: Event) -> Self {
        Self { from, to, event }
    }
}

#[derive(Default)]
pub struct Coordinator {
    subscribers: RwLock<HashMap<String, Vec<CoordinationMessage>>>,
}

impl Coordinator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_idle(&self) -> bool {
        self.subscribers
            .try_read()
            .map(|subs| subs.values().all(Vec::is_empty))
            .unwrap_or(true)
    }

    pub async fn subscribe(&mut self, agent: String) {
        self.subscribers.write().await.entry(agent).or_default();
    }

    pub async fn unsubscribe(&mut self, agent: &str) {
        self.subscribers.write().await.remove(agent);
    }

    pub async fn broadcast(&self, event: Event) {
        let agents: Vec<String> = self.subscribers.read().await.keys().cloned().collect();
        for agent in agents {
            self.send_to("coordinator".to_string(), agent, event.clone())
                .await;
        }
    }

    pub async fn send_to(&self, from: String, to: String, event: Event) {
        self.subscribers
            .write()
            .await
            .entry(to.clone())
            .or_default()
            .push(CoordinationMessage::new(from, to, event));
    }

    pub async fn get_events(&self, agent: &str) -> Vec<CoordinationMessage> {
        self.subscribers
            .read()
            .await
            .get(agent)
            .cloned()
            .unwrap_or_default()
    }
}

pub struct DistributedLock {
    // TODO: Dead code - remove or expose the resource id in lock state reporting.
    #[allow(dead_code)]
    resource_id: String,
    owner: RwLock<Option<String>>,
}

impl DistributedLock {
    pub fn new(resource_id: String) -> Self {
        Self {
            resource_id,
            owner: RwLock::new(None),
        }
    }

    pub async fn try_acquire(&self, owner: &str) -> bool {
        let mut current = self.owner.write().await;
        if current.is_none() {
            *current = Some(owner.to_string());
            true
        } else {
            false
        }
    }

    pub async fn release(&self, owner: &str) {
        let mut current = self.owner.write().await;
        if current.as_deref() == Some(owner) {
            *current = None;
        }
    }
}

/// Agent handle - reference to a registered agent
#[derive(Clone)]
pub struct AgentHandle {
    /// Agent ID
    pub id: String,

    /// Agent name
    pub name: String,

    /// Agent capabilities
    pub capabilities: Vec<String>,

    /// Reference to message bus
    bus: Arc<MessageBus>,
}

impl AgentHandle {
    /// Create a new agent handle
    pub fn new(id: String, name: String, capabilities: Vec<String>, bus: Arc<MessageBus>) -> Self {
        Self {
            id,
            name,
            capabilities,
            bus,
        }
    }

    /// Send a message to this agent
    pub async fn send(&self, content: serde_json::Value) -> Result<String, MessageBusError> {
        self.bus.send_direct(&self.id, &self.id, content).await
    }

    /// Send a task to this agent
    pub async fn send_task(&self, task: serde_json::Value) -> Result<String, MessageBusError> {
        let msg = AgentMessage::task(&self.id, task);
        let id = msg.id.clone();
        self.bus.publish(msg).await?;
        Ok(id)
    }

    /// Get agent info
    pub async fn info(&self) -> Option<AgentInfo> {
        self.bus.get_agent(&self.id).await
    }

    /// Update heartbeat
    pub async fn heartbeat(&self) -> Result<(), MessageBusError> {
        self.bus.heartbeat(&self.id).await
    }
}

/// Registry for managing agents
pub struct AgentRegistry {
    /// Message bus reference
    bus: Arc<MessageBus>,

    /// Local agent handles
    handles: RwLock<HashMap<String, AgentHandle>>,

    /// Agent metadata
    metadata: RwLock<HashMap<String, AgentMetadata>>,
}

/// Additional agent metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetadata {
    pub id: String,
    pub name: String,
    pub role: Option<String>,
    pub description: Option<String>,
    pub owner: Option<String>,
    pub config: HashMap<String, serde_json::Value>,
    pub registered_at: DateTime<Utc>,
}

impl AgentRegistry {
    /// Create a new agent registry
    pub fn new(bus: Arc<MessageBus>) -> Arc<Self> {
        Arc::new(Self {
            bus,
            handles: RwLock::new(HashMap::new()),
            metadata: RwLock::new(HashMap::new()),
        })
    }

    /// Register a new agent
    pub async fn register(
        &self,
        id: &str,
        name: &str,
        capabilities: Vec<String>,
        role: Option<String>,
        description: Option<String>,
    ) -> Result<AgentHandle, MessageBusError> {
        // Register with message bus
        let _rx = self
            .bus
            .register_agent(id, name, capabilities.clone())
            .await?;

        // Create handle
        let handle = AgentHandle::new(
            id.to_string(),
            name.to_string(),
            capabilities.clone(),
            self.bus.clone(),
        );

        // Store handle
        {
            let mut handles = self.handles.write().await;
            handles.insert(id.to_string(), handle.clone());
        }

        // Store metadata
        {
            let mut metadata = self.metadata.write().await;
            metadata.insert(
                id.to_string(),
                AgentMetadata {
                    id: id.to_string(),
                    name: name.to_string(),
                    role,
                    description,
                    owner: None,
                    config: HashMap::new(),
                    registered_at: Utc::now(),
                },
            );
        }

        tracing::info!("Agent {} registered in registry", id);

        Ok(handle)
    }

    /// Unregister an agent
    pub async fn unregister(&self, id: &str) -> Result<(), MessageBusError> {
        // Remove handle
        {
            let mut handles = self.handles.write().await;
            handles.remove(id);
        }

        // Remove metadata
        {
            let mut metadata = self.metadata.write().await;
            metadata.remove(id);
        }

        // Unregister from bus
        self.bus.unregister_agent(id).await
    }

    /// Get agent handle
    pub async fn get(&self, id: &str) -> Option<AgentHandle> {
        let handles = self.handles.read().await;
        handles.get(id).cloned()
    }

    /// List all registered agent IDs
    pub async fn list_ids(&self) -> Vec<String> {
        let handles = self.handles.read().await;
        handles.keys().cloned().collect()
    }

    /// List all registered agents
    pub async fn list(&self) -> Vec<AgentInfo> {
        self.bus.list_agents().await
    }

    /// Find agents by capability
    pub async fn find_by_capability(&self, capability: &str) -> Vec<AgentHandle> {
        let handles = self.handles.read().await;

        handles
            .values()
            .filter(|h| h.capabilities.iter().any(|c| c == capability))
            .cloned()
            .collect()
    }

    /// Get agent metadata
    pub async fn get_metadata(&self, id: &str) -> Option<AgentMetadata> {
        let metadata = self.metadata.read().await;
        metadata.get(id).cloned()
    }

    /// Update agent metadata
    pub async fn update_metadata(
        &self,
        id: &str,
        role: Option<String>,
        description: Option<String>,
        owner: Option<String>,
    ) -> Result<(), MessageBusError> {
        let mut metadata = self.metadata.write().await;

        if let Some(agent) = metadata.get_mut(id) {
            if let Some(r) = role {
                agent.role = Some(r);
            }
            if let Some(d) = description {
                agent.description = Some(d);
            }
            if let Some(o) = owner {
                agent.owner = Some(o);
            }
            return Ok(());
        }

        Err(MessageBusError::AgentNotFound(id.to_string()))
    }

    /// Get or create agent receiver
    pub async fn get_receiver(
        &self,
        _id: &str,
    ) -> Option<tokio::sync::mpsc::Receiver<AgentMessage>> {
        // The receiver was returned during registration
        // For now, we need a way to store it
        None
    }
}

/// Task distribution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDistribution {
    pub task_id: String,
    pub assigned_agents: Vec<String>,
    pub status: DistributionStatus,
}

/// Distribution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DistributionStatus {
    Pending,
    Assigned,
    Failed,
    Completed,
}

/// Coordination service for orchestrating agents
pub struct CoordinationService {
    /// Agent registry
    registry: Arc<AgentRegistry>,

    /// Message bus
    bus: Arc<MessageBus>,

    /// Active tasks
    tasks: RwLock<HashMap<String, CoordinationTask>>,

    /// Task queue
    task_queue: RwLock<Vec<String>>,
}

/// Coordination task
#[derive(Debug, Clone)]
pub struct CoordinationTask {
    pub id: String,
    pub name: String,
    pub description: String,
    pub required_capabilities: Vec<String>,
    pub assigned_agents: Vec<String>,
    pub status: CoordinationTaskStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub result: Option<serde_json::Value>,
}

/// Coordination task status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CoordinationTaskStatus {
    Pending,
    Dispatched,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl CoordinationService {
    /// Create a new coordination service
    pub fn new(bus: Arc<MessageBus>) -> Arc<Self> {
        let registry = AgentRegistry::new(bus.clone());

        Arc::new(Self {
            registry,
            bus,
            tasks: RwLock::new(HashMap::new()),
            task_queue: RwLock::new(Vec::new()),
        })
    }

    /// Get the agent registry
    pub fn registry(&self) -> &Arc<AgentRegistry> {
        &self.registry
    }

    /// Get the message bus
    pub fn bus(&self) -> &Arc<MessageBus> {
        &self.bus
    }

    /// Create a coordination task
    pub async fn create_task(
        &self,
        name: &str,
        description: &str,
        required_capabilities: Vec<String>,
    ) -> String {
        let task_id = Ulid::new().to_string();

        let task = CoordinationTask {
            id: task_id.clone(),
            name: name.to_string(),
            description: description.to_string(),
            required_capabilities,
            assigned_agents: Vec::new(),
            status: CoordinationTaskStatus::Pending,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            result: None,
        };

        {
            let mut tasks = self.tasks.write().await;
            tasks.insert(task_id.clone(), task);
        }

        {
            let mut queue = self.task_queue.write().await;
            queue.push(task_id.clone());
        }

        tracing::info!("Coordination task {} created", task_id);

        task_id
    }

    /// Dispatch a task to available agents
    pub async fn dispatch_task(&self, task_id: &str) -> Result<TaskDistribution, MessageBusError> {
        let task = {
            let tasks = self.tasks.read().await;
            tasks.get(task_id).cloned()
        };

        let task = match task {
            Some(t) => t,
            None => return Err(MessageBusError::SendFailed("Task not found".to_string())),
        };

        // Find agents with required capabilities
        let agents = self
            .registry
            .find_by_capability(
                task.required_capabilities
                    .first()
                    .unwrap_or(&"default".to_string()),
            )
            .await;

        if agents.is_empty() {
            return Err(MessageBusError::SendFailed(
                "No agents available".to_string(),
            ));
        }

        // Assign to first available agent
        let agent = &agents[0];

        // Send task to agent
        let msg = serde_json::json!({
            "task_id": task_id,
            "task_name": task.name,
            "task_description": task.description,
            "required_capabilities": task.required_capabilities,
        });

        agent.send_task(msg).await?;

        // Update task status
        {
            let mut tasks = self.tasks.write().await;
            if let Some(t) = tasks.get_mut(task_id) {
                t.status = CoordinationTaskStatus::Dispatched;
                t.assigned_agents.push(agent.id.clone());
                t.updated_at = Utc::now();
            }
        }

        tracing::info!("Task {} dispatched to agent {}", task_id, agent.id);

        Ok(TaskDistribution {
            task_id: task_id.to_string(),
            assigned_agents: vec![agent.id.clone()],
            status: DistributionStatus::Assigned,
        })
    }

    /// Broadcast task to all agents
    pub async fn broadcast_task(&self, task_id: &str) -> Result<TaskDistribution, MessageBusError> {
        let task = {
            let tasks = self.tasks.read().await;
            tasks.get(task_id).cloned()
        };

        let task = match task {
            Some(t) => t,
            None => return Err(MessageBusError::SendFailed("Task not found".to_string())),
        };

        // Get all agents
        let agents = self.registry.list().await;

        if agents.is_empty() {
            return Err(MessageBusError::SendFailed(
                "No agents available".to_string(),
            ));
        }

        let assigned: Vec<String> = agents.iter().map(|a| a.id.clone()).collect();

        // Broadcast to all
        let msg = serde_json::json!({
            "task_id": task_id,
            "task_name": task.name,
            "task_description": task.description,
            "required_capabilities": task.required_capabilities,
        });

        self.bus.broadcast("coordinator", msg, None).await?;

        // Update task status
        {
            let mut tasks = self.tasks.write().await;
            if let Some(t) = tasks.get_mut(task_id) {
                t.status = CoordinationTaskStatus::Dispatched;
                t.assigned_agents = assigned.clone();
                t.updated_at = Utc::now();
            }
        }

        tracing::info!("Task {} broadcast to {} agents", task_id, assigned.len());

        Ok(TaskDistribution {
            task_id: task_id.to_string(),
            assigned_agents: assigned,
            status: DistributionStatus::Assigned,
        })
    }

    /// Get task status
    pub async fn get_task(&self, task_id: &str) -> Option<CoordinationTask> {
        let tasks = self.tasks.read().await;
        tasks.get(task_id).cloned()
    }

    /// List all tasks
    pub async fn list_tasks(&self) -> Vec<CoordinationTask> {
        let tasks = self.tasks.read().await;
        tasks.values().cloned().collect()
    }

    /// Complete a task with result
    pub async fn complete_task(
        &self,
        task_id: &str,
        result: serde_json::Value,
    ) -> Result<(), MessageBusError> {
        let mut tasks = self.tasks.write().await;

        if let Some(task) = tasks.get_mut(task_id) {
            task.status = CoordinationTaskStatus::Completed;
            task.result = Some(result);
            task.updated_at = Utc::now();
            return Ok(());
        }

        Err(MessageBusError::SendFailed("Task not found".to_string()))
    }

    /// Cancel a task
    pub async fn cancel_task(&self, task_id: &str) -> Result<(), MessageBusError> {
        let mut tasks = self.tasks.write().await;

        if let Some(task) = tasks.get_mut(task_id) {
            task.status = CoordinationTaskStatus::Cancelled;
            task.updated_at = Utc::now();
            return Ok(());
        }

        Err(MessageBusError::SendFailed("Task not found".to_string()))
    }

    /// Get pending tasks from queue
    pub async fn get_pending_tasks(&self) -> Vec<CoordinationTask> {
        let queue = self.task_queue.read().await;
        let tasks = self.tasks.read().await;

        queue
            .iter()
            .filter_map(|id| tasks.get(id))
            .filter(|t| t.status == CoordinationTaskStatus::Pending)
            .cloned()
            .collect()
    }

    /// Process task results
    pub async fn process_result(&self, message: AgentMessage) -> Result<(), MessageBusError> {
        if message.msg_type == MessageType::Result {
            if let Some(correlation_id) = message.correlation_id {
                // Extract task_id from correlation
                // For now, just log
                tracing::debug!("Received result for correlation {}", correlation_id);
            }
        }

        Ok(())
    }

    /// Health check - verify agents are responsive
    pub async fn health_check(&self) -> HashMap<String, bool> {
        let agents = self.registry.list().await;
        let mut status = HashMap::new();

        for agent in agents {
            // Simple check - just verify registered
            status.insert(agent.id, agent.status != AgentStatus::Offline);
        }

        status
    }

    /// Get service metrics
    pub async fn metrics(&self) -> CoordinationMetrics {
        let agents = self.registry.list().await;
        let tasks = self.list_tasks().await;

        CoordinationMetrics {
            total_agents: agents.len(),
            active_agents: agents
                .iter()
                .filter(|a| a.status == AgentStatus::Active)
                .count(),
            total_tasks: tasks.len(),
            pending_tasks: tasks
                .iter()
                .filter(|t| t.status == CoordinationTaskStatus::Pending)
                .count(),
            completed_tasks: tasks
                .iter()
                .filter(|t| t.status == CoordinationTaskStatus::Completed)
                .count(),
            failed_tasks: tasks
                .iter()
                .filter(|t| t.status == CoordinationTaskStatus::Failed)
                .count(),
            bus_metrics: self.bus.get_metrics().await,
        }
    }
}

/// Coordination service metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationMetrics {
    pub total_agents: usize,
    pub active_agents: usize,
    pub total_tasks: usize,
    pub pending_tasks: usize,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
    pub bus_metrics: BusMetrics,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_registry() {
        let bus = MessageBus::new();
        let registry = AgentRegistry::new(bus);

        // Register agent
        let handle = registry
            .register(
                "agent1",
                "Test Agent",
                vec!["test".to_string()],
                Some("worker".to_string()),
                Some("Test agent description".to_string()),
            )
            .await;

        assert!(handle.is_ok());

        // List agents
        let agents = registry.list().await;
        assert_eq!(agents.len(), 1);

        // Get metadata
        let metadata = registry.get_metadata("agent1").await;
        assert!(metadata.is_some());
        assert_eq!(metadata.expect("test assertion").role, Some("worker".to_string()));
    }

    #[tokio::test]
    async fn test_coordination_service() {
        let bus = MessageBus::new();
        let service = CoordinationService::new(bus);

        // Register an agent first
        service
            .registry()
            .register("agent1", "Worker", vec!["default".to_string()], None, None)
            .await
            .expect("test assertion");

        // Create task
        let task_id = service
            .create_task("Test Task", "A test task", vec!["default".to_string()])
            .await;

        assert!(!task_id.is_empty());

        // Dispatch task
        let result = service.dispatch_task(&task_id).await;
        assert!(result.is_ok());

        // Get task
        let task = service.get_task(&task_id).await;
        assert!(task.is_some());
    }
}
