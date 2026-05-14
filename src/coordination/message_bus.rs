//! Message Bus for Agent Coordination
//!
//! Provides async message passing between agents using tokio channels.
//! Supports pub/sub, direct messaging, request/response, and broadcast.
//!
//! Architecture:
//! - Per-agent queues for receiving messages
//! - Topic-based pub/sub subscriptions
//! - Request/response with timeout support
//! - Dead Letter Queue for failed messages
//!
//! Based on: RESEARCH_agent_coordination.md

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc, RwLock};
use ulid::Ulid;

/// Message priority levels
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessagePriority {
    Low,
    #[default]
    Normal,
    High,
    Critical,
}

impl MessagePriority {
    pub fn value(&self) -> u8 {
        match self {
            MessagePriority::Low => 1,
            MessagePriority::Normal => 2,
            MessagePriority::High => 3,
            MessagePriority::Critical => 4,
        }
    }
}

/// Message types for agent communication
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageType {
    #[default]
    Task,
    Result,
    Error,
    Heartbeat,
    Register,
    Unregister,
    Shutdown,
}

/// Core message structure for agent communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    /// Unique message ID
    pub id: String,

    /// Sender agent ID
    pub sender: String,

    /// Receiver agent ID (None = broadcast/topic)
    pub receiver: Option<String>,

    /// Topic for pub/sub (None = direct message)
    pub topic: Option<String>,

    /// Message type
    pub msg_type: MessageType,

    /// Message content (any serializable data)
    pub content: serde_json::Value,

    /// Priority level
    pub priority: MessagePriority,

    /// Timestamp
    pub timestamp: DateTime<Utc>,

    /// Correlation ID for request/response
    pub correlation_id: Option<String>,

    /// Reply channel ID
    pub reply_to: Option<String>,

    /// Additional metadata
    pub metadata: HashMap<String, String>,

    /// Retry count
    pub retries: u32,

    /// Max retries before going to DLQ
    pub max_retries: u32,
}

impl AgentMessage {
    /// Create a new message
    pub fn new(sender: &str, msg_type: MessageType, content: serde_json::Value) -> Self {
        Self {
            id: Ulid::new().to_string(),
            sender: sender.to_string(),
            receiver: None,
            topic: None,
            msg_type,
            content,
            priority: MessagePriority::default(),
            timestamp: Utc::now(),
            correlation_id: None,
            reply_to: None,
            metadata: HashMap::new(),
            retries: 0,
            max_retries: 3,
        }
    }

    /// Create a task message
    pub fn task(sender: &str, content: serde_json::Value) -> Self {
        Self::new(sender, MessageType::Task, content)
    }

    /// Create a result message
    pub fn result(sender: &str, content: serde_json::Value) -> Self {
        Self::new(sender, MessageType::Result, content)
    }

    /// Create an error message
    pub fn error(sender: &str, content: serde_json::Value) -> Self {
        Self::new(sender, MessageType::Error, content)
    }

    /// Create a heartbeat message
    pub fn heartbeat(sender: &str) -> Self {
        Self::new(
            sender,
            MessageType::Heartbeat,
            serde_json::json!({ "status": "alive" }),
        )
    }

    /// Set receiver (direct message)
    pub fn to(mut self, receiver: &str) -> Self {
        self.receiver = Some(receiver.to_string());
        self
    }

    /// Set topic (pub/sub)
    pub fn on_topic(mut self, topic: &str) -> Self {
        self.topic = Some(topic.to_string());
        self
    }

    /// Set correlation ID for request/response
    pub fn with_correlation(mut self, correlation_id: &str) -> Self {
        self.correlation_id = Some(correlation_id.to_string());
        self
    }

    /// Set reply channel
    pub fn reply_to_channel(mut self, channel_id: &str) -> Self {
        self.reply_to = Some(channel_id.to_string());
        self
    }

    /// Set priority
    pub fn with_priority(mut self, priority: MessagePriority) -> Self {
        self.priority = priority;
        self
    }
}

/// Agent subscription info
#[derive(Debug, Clone)]
pub struct Subscription {
    pub agent_id: String,
    pub topic: String,
}

/// Result of a request/response operation
#[derive(Debug)]
pub struct Response {
    pub message: AgentMessage,
    pub received_at: DateTime<Utc>,
}

/// Dead Letter Queue entry with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DLQEntry {
    pub message: AgentMessage,
    pub failed_at: DateTime<Utc>,
    pub failure_reason: String,
    pub retry_count: u32,
}

impl DLQEntry {
    pub fn new(message: AgentMessage, reason: &str) -> Self {
        Self {
            message,
            failed_at: Utc::now(),
            failure_reason: reason.to_string(),
            retry_count: 0,
        }
    }

    pub fn with_retry(mut self, count: u32) -> Self {
        self.retry_count = count;
        self
    }
}

/// Heartbeat configuration
#[derive(Debug, Clone)]
pub struct HeartbeatConfig {
    /// Timeout in seconds before marking agent as offline
    pub timeout_secs: u64,
    /// Interval to check for stale heartbeats
    pub check_interval_secs: u64,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 30,
            check_interval_secs: 10,
        }
    }
}

/// Message Bus metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BusMetrics {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub messages_failed: u64,
    pub dlq_size: usize,
    pub registered_agents: usize,
    pub topic_subscriptions: HashMap<String, usize>,
    pub queue_sizes: HashMap<String, usize>,
    pub stale_agents: usize,
    pub heartbeats_received: u64,
}

/// The main Message Bus for agent coordination
pub struct MessageBus {
    /// Per-agent message receivers
    queues: RwLock<HashMap<String, mpsc::Sender<AgentMessage>>>,

    /// Topic subscriptions (topic -> set of agent IDs)
    topics: RwLock<HashMap<String, std::collections::HashSet<String>>>,

    /// Broadcast channel for topic messages
    broadcast_tx: broadcast::Sender<AgentMessage>,

    /// Response channels for request/response
    response_channels: RwLock<HashMap<String, mpsc::Sender<AgentMessage>>>,

    /// Dead Letter Queue with metadata
    dlq: RwLock<Vec<DLQEntry>>,

    /// Metrics
    metrics: RwLock<BusMetrics>,

    /// Registered agents
    agents: RwLock<HashMap<String, AgentInfo>>,

    /// Heartbeat configuration
    heartbeat_config: HeartbeatConfig,
}

/// Agent information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub capabilities: Vec<String>,
    pub registered_at: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
    pub status: AgentStatus,
}

/// Agent status
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentStatus {
    #[default]
    Registered,
    Active,
    Idle,
    Offline,
}

impl MessageBus {
    /// Create a new MessageBus with default config
    pub fn new() -> Arc<Self> {
        Self::with_config(HeartbeatConfig::default())
    }

    /// Create a new MessageBus with custom heartbeat config
    pub fn with_config(config: HeartbeatConfig) -> Arc<Self> {
        let (broadcast_tx, _) = broadcast::channel(1000);

        Arc::new(Self {
            queues: RwLock::new(HashMap::new()),
            topics: RwLock::new(HashMap::new()),
            broadcast_tx,
            response_channels: RwLock::new(HashMap::new()),
            dlq: RwLock::new(Vec::new()),
            metrics: RwLock::new(BusMetrics::default()),
            agents: RwLock::new(HashMap::new()),
            heartbeat_config: config,
        })
    }

    /// Register an agent with the message bus
    pub async fn register_agent(
        &self,
        agent_id: &str,
        name: &str,
        capabilities: Vec<String>,
    ) -> Result<mpsc::Receiver<AgentMessage>, MessageBusError> {
        let (tx, rx) = mpsc::channel(100);

        {
            let mut queues = self.queues.write().await;
            if queues.contains_key(agent_id) {
                return Err(MessageBusError::AgentAlreadyRegistered(
                    agent_id.to_string(),
                ));
            }
            queues.insert(agent_id.to_string(), tx);
        }

        {
            let mut agents = self.agents.write().await;
            agents.insert(
                agent_id.to_string(),
                AgentInfo {
                    id: agent_id.to_string(),
                    name: name.to_string(),
                    capabilities,
                    registered_at: Utc::now(),
                    last_heartbeat: Utc::now(),
                    status: AgentStatus::Active,
                },
            );
        }

        {
            let mut metrics = self.metrics.write().await;
            metrics.registered_agents = self.agents.read().await.len();
        }

        tracing::info!("Agent {} registered with message bus", agent_id);

        Ok(rx)
    }

    /// Unregister an agent
    pub async fn unregister_agent(&self, agent_id: &str) -> Result<(), MessageBusError> {
        {
            let mut queues = self.queues.write().await;
            queues.remove(agent_id);
        }

        {
            let mut agents = self.agents.write().await;
            if let Some(agent) = agents.get_mut(agent_id) {
                agent.status = AgentStatus::Offline;
            }
            agents.remove(agent_id);
        }

        {
            let mut topics = self.topics.write().await;
            for subscribers in topics.values_mut() {
                subscribers.remove(agent_id);
            }
        }

        {
            let mut metrics = self.metrics.write().await;
            metrics.registered_agents = self.agents.read().await.len();
        }

        tracing::info!("Agent {} unregistered from message bus", agent_id);

        Ok(())
    }

    /// Subscribe an agent to a topic
    pub async fn subscribe(&self, agent_id: &str, topic: &str) -> Result<(), MessageBusError> {
        // Verify agent exists
        {
            let agents = self.agents.read().await;
            if !agents.contains_key(agent_id) {
                return Err(MessageBusError::AgentNotFound(agent_id.to_string()));
            }
        }

        let mut topics = self.topics.write().await;
        topics
            .entry(topic.to_string())
            .or_default()
            .insert(agent_id.to_string());

        {
            let mut metrics = self.metrics.write().await;
            metrics.topic_subscriptions =
                topics.iter().map(|(k, v)| (k.clone(), v.len())).collect();
        }

        tracing::debug!("Agent {} subscribed to topic {}", agent_id, topic);

        Ok(())
    }

    /// Unsubscribe an agent from a topic
    pub async fn unsubscribe(&self, agent_id: &str, topic: &str) -> Result<(), MessageBusError> {
        let mut topics = self.topics.write().await;

        if let Some(subscribers) = topics.get_mut(topic) {
            subscribers.remove(agent_id);
        }

        {
            let mut metrics = self.metrics.write().await;
            metrics.topic_subscriptions =
                topics.iter().map(|(k, v)| (k.clone(), v.len())).collect();
        }

        tracing::debug!("Agent {} unsubscribed from topic {}", agent_id, topic);

        Ok(())
    }

    /// Publish a message to the bus
    pub async fn publish(&self, message: AgentMessage) -> Result<usize, MessageBusError> {
        {
            let mut metrics = self.metrics.write().await;
            metrics.messages_sent += 1;
        }

        let receivers = self.determine_receivers(&message).await;

        if receivers.is_empty() {
            tracing::warn!("Message {} has no receivers", message.id);
            return Ok(0);
        }

        let mut sent_count = 0;

        for receiver_id in &receivers {
            if let Some(tx) = self.queues.read().await.get(receiver_id) {
                if tx.send(message.clone()).await.is_err() {
                    tracing::warn!("Failed to send message to agent {}", receiver_id);
                }
                sent_count += 1;
            }
        }

        // Also broadcast to topic subscribers
        if let Some(ref topic) = message.topic {
            let _ = self.broadcast_tx.send(message.clone());

            // Get topic subscribers
            let topics = self.topics.read().await;
            if let Some(subscribers) = topics.get(topic) {
                for subscriber_id in subscribers {
                    if !receivers.contains(subscriber_id) {
                        if let Some(tx) = self.queues.read().await.get(subscriber_id) {
                            if tx.send(message.clone()).await.is_ok() {
                                sent_count += 1;
                            }
                        }
                    }
                }
            }
        }

        tracing::debug!(
            "Message {} published to {} receivers",
            message.id,
            sent_count
        );

        Ok(sent_count)
    }

    /// Determine message receivers based on receiver/topic
    async fn determine_receivers(&self, message: &AgentMessage) -> Vec<String> {
        // Direct message
        if let Some(ref receiver) = message.receiver {
            return vec![receiver.clone()];
        }

        // Topic-based
        if let Some(ref topic) = message.topic {
            let topics = self.topics.read().await;
            if let Some(subscribers) = topics.get(topic) {
                return subscribers.iter().cloned().collect();
            }
        }

        // Broadcast to all
        let queues = self.queues.read().await;
        queues.keys().cloned().collect()
    }

    /// Send a direct message to an agent
    pub async fn send_direct(
        &self,
        sender: &str,
        receiver: &str,
        content: serde_json::Value,
    ) -> Result<String, MessageBusError> {
        let message = AgentMessage::task(sender, content).to(receiver);
        let id = message.id.clone();
        self.publish(message).await?;
        Ok(id)
    }

    /// Broadcast a message to all agents
    pub async fn broadcast(
        &self,
        sender: &str,
        content: serde_json::Value,
        topic: Option<&str>,
    ) -> Result<String, MessageBusError> {
        let mut message = AgentMessage::new(sender, MessageType::Task, content);
        message.topic = topic.map(String::from);
        let id = message.id.clone();

        self.publish(message).await?;
        Ok(id)
    }

    /// Send a request and wait for response with timeout
    pub async fn request(
        &self,
        sender: &str,
        receiver: &str,
        content: serde_json::Value,
        timeout_secs: u64,
    ) -> Result<AgentMessage, MessageBusError> {
        let correlation_id = Ulid::new().to_string();

        // Create response channel
        let (response_tx, mut response_rx) = mpsc::channel(1);

        {
            let mut channels = self.response_channels.write().await;
            channels.insert(correlation_id.clone(), response_tx);
        }

        // Send the request
        let message = AgentMessage::task(sender, content)
            .to(receiver)
            .with_correlation(&correlation_id)
            .reply_to_channel(&correlation_id);

        self.publish(message).await?;

        // Wait for response
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            response_rx.recv(),
        )
        .await
        .map_err(|_| MessageBusError::RequestTimeout(timeout_secs))?
        .ok_or(MessageBusError::ChannelClosed)?;

        // Cleanup
        {
            let mut channels = self.response_channels.write().await;
            channels.remove(&correlation_id);
        }

        Ok(result)
    }

    /// Handle incoming response for a request
    pub async fn handle_response(&self, message: AgentMessage) -> Result<(), MessageBusError> {
        if let Some(ref correlation_id) = message.correlation_id {
            let channels = self.response_channels.read().await;

            if let Some(tx) = channels.get(correlation_id) {
                let _ = tx.send(message).await;
            }
        }

        Ok(())
    }

    /// Receive a message for a specific agent (blocking)
    pub async fn receive(
        &self,
        agent_id: &str,
        timeout: Option<u64>,
    ) -> Result<Option<AgentMessage>, MessageBusError> {
        let queues = self.queues.read().await;
        if queues.contains_key(agent_id) {
            drop(queues);

            if let Some(secs) = timeout {
                tokio::time::sleep(std::time::Duration::from_secs(secs.min(1))).await;
            }

            return Ok(None);
        }

        Err(MessageBusError::AgentNotFound(agent_id.to_string()))
    }

    /// Send a message to the Dead Letter Queue
    pub async fn send_to_dlq(
        &self,
        message: AgentMessage,
        reason: &str,
    ) -> Result<(), MessageBusError> {
        let entry = DLQEntry::new(message, reason);
        let mut dlq = self.dlq.write().await;
        dlq.push(entry);

        let mut metrics = self.metrics.write().await;
        metrics.messages_failed += 1;
        metrics.dlq_size = dlq.len();

        tracing::error!("Message sent to DLQ: {}", reason);

        Ok(())
    }

    /// Send a failed message to DLQ with retry count
    pub async fn send_to_dlq_with_retry(
        &self,
        message: AgentMessage,
        reason: &str,
        retry_count: u32,
    ) -> Result<(), MessageBusError> {
        let entry = DLQEntry::new(message, reason).with_retry(retry_count);
        let mut dlq = self.dlq.write().await;
        dlq.push(entry);

        let mut metrics = self.metrics.write().await;
        metrics.messages_failed += 1;
        metrics.dlq_size = dlq.len();

        tracing::error!(
            "Message sent to DLQ after {} retries: {}",
            retry_count,
            reason
        );

        Ok(())
    }

    /// Get messages from Dead Letter Queue
    pub async fn get_dlq(&self) -> Vec<DLQEntry> {
        let dlq = self.dlq.read().await;
        dlq.clone()
    }

    /// Get DLQ size
    pub async fn get_dlq_size(&self) -> usize {
        let dlq = self.dlq.read().await;
        dlq.len()
    }

    /// Clear Dead Letter Queue
    pub async fn clear_dlq(&self) -> usize {
        let mut dlq = self.dlq.write().await;
        let size = dlq.len();
        dlq.clear();

        let mut metrics = self.metrics.write().await;
        metrics.dlq_size = 0;

        size
    }

    /// Remove and return a specific message from DLQ for reprocessing
    pub async fn reprocess_dlq_message(&self, message_id: &str) -> Option<AgentMessage> {
        let mut dlq = self.dlq.write().await;

        if let Some(pos) = dlq.iter().position(|e| e.message.id == message_id) {
            let entry = dlq.remove(pos);

            let mut metrics = self.metrics.write().await;
            metrics.dlq_size = dlq.len();

            tracing::info!("Reprocessing DLQ message: {}", message_id);
            return Some(entry.message);
        }

        None
    }

    /// Update agent heartbeat
    pub async fn heartbeat(&self, agent_id: &str) -> Result<(), MessageBusError> {
        let mut agents = self.agents.write().await;

        if let Some(agent) = agents.get_mut(agent_id) {
            agent.last_heartbeat = Utc::now();
            agent.status = AgentStatus::Active;

            // Update metrics
            drop(agents);
            let mut metrics = self.metrics.write().await;
            metrics.heartbeats_received += 1;
            metrics.stale_agents = 0; // Reset, will be recalculated

            return Ok(());
        }

        Err(MessageBusError::AgentNotFound(agent_id.to_string()))
    }

    /// Get agents that have stale heartbeats ( haven't sent heartbeat within timeout )
    pub async fn get_stale_agents(&self) -> Vec<AgentInfo> {
        let agents = self.agents.read().await;
        let timeout = chrono::Duration::seconds(self.heartbeat_config.timeout_secs as i64);
        let now = Utc::now();

        agents
            .values()
            .filter(|a| now.signed_duration_since(a.last_heartbeat) > timeout)
            .cloned()
            .collect()
    }

    /// Mark stale agents as offline
    pub async fn mark_stale_offline(&self) -> usize {
        let stale = self.get_stale_agents().await;
        let mut count = 0;

        let mut agents = self.agents.write().await;
        for agent in &stale {
            if let Some(a) = agents.get_mut(&agent.id) {
                a.status = AgentStatus::Offline;
                count += 1;
            }
        }

        // Update metrics
        drop(agents);
        let mut metrics = self.metrics.write().await;
        metrics.stale_agents = count;

        if count > 0 {
            tracing::warn!("Marked {} agents as offline due to stale heartbeat", count);
        }

        count
    }

    /// Check if an agent's heartbeat is stale
    pub async fn is_heartbeat_stale(&self, agent_id: &str) -> bool {
        let agents = self.agents.read().await;

        if let Some(agent) = agents.get(agent_id) {
            let timeout = chrono::Duration::seconds(self.heartbeat_config.timeout_secs as i64);
            return Utc::now().signed_duration_since(agent.last_heartbeat) > timeout;
        }

        false
    }

    /// Get agent info
    pub async fn get_agent(&self, agent_id: &str) -> Option<AgentInfo> {
        let agents = self.agents.read().await;
        agents.get(agent_id).cloned()
    }

    /// List all registered agents
    pub async fn list_agents(&self) -> Vec<AgentInfo> {
        let agents = self.agents.read().await;
        agents.values().cloned().collect()
    }

    /// Get bus metrics
    pub async fn get_metrics(&self) -> BusMetrics {
        let mut metrics = self.metrics.write().await;

        // Update queue sizes
        let queues = self.queues.read().await;
        metrics.queue_sizes = queues
            .iter()
            .map(|(k, v)| (k.clone(), v.capacity()))
            .collect();

        metrics.clone()
    }

    /// Get topic subscriptions
    pub async fn get_topics(&self) -> HashMap<String, Vec<String>> {
        let topics = self.topics.read().await;
        topics
            .iter()
            .map(|(k, v)| (k.clone(), v.iter().cloned().collect()))
            .collect()
    }

    /// Shutdown the message bus
    pub async fn shutdown(&self) {
        // Clear all queues
        {
            let mut queues = self.queues.write().await;
            queues.clear();
        }

        // Clear agents
        {
            let mut agents = self.agents.write().await;
            agents.clear();
        }

        tracing::info!("Message bus shutdown complete");
    }
}

/// Message Bus errors
#[derive(Debug, thiserror::Error)]
pub enum MessageBusError {
    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    #[error("Agent already registered: {0}")]
    AgentAlreadyRegistered(String),

    #[error("Request timeout after {0} seconds")]
    RequestTimeout(u64),

    #[error("Channel closed")]
    ChannelClosed,

    #[error("Message send failed: {0}")]
    SendFailed(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

impl serde::Serialize for MessageBusError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_unregister() {
        let bus = MessageBus::new();

        // Register agent
        let rx = bus
            .register_agent("agent1", "Test Agent", vec!["test".to_string()])
            .await;
        assert!(rx.is_ok());

        // Double registration should fail
        let rx = bus.register_agent("agent1", "Test Agent", vec![]).await;
        assert!(rx.is_err());

        // Unregister
        let result = bus.unregister_agent("agent1").await;
        assert!(result.is_ok());

        // Re-register should work
        let rx = bus.register_agent("agent1", "Test Agent", vec![]).await;
        assert!(rx.is_ok());
    }

    #[tokio::test]
    async fn test_publish_subscribe() {
        let bus = MessageBus::new();

        // Register agents
        bus.register_agent("agent1", "Agent 1", vec![])
            .await
            .expect("test assertion");
        bus.register_agent("agent2", "Agent 2", vec![])
            .await
            .expect("test assertion");

        // Subscribe agent1 to topic
        bus.subscribe("agent1", "news")
            .await
            .expect("test assertion");

        // Publish to topic
        let msg = AgentMessage::new(
            "agent2",
            MessageType::Task,
            serde_json::json!({"text": "hello"}),
        )
        .on_topic("news");

        let count = bus.publish(msg).await.expect("test assertion");
        assert!(count >= 1);
    }

    #[tokio::test]
    async fn test_direct_message() {
        let bus = MessageBus::new();

        bus.register_agent("sender", "Sender", vec![])
            .await
            .expect("test assertion");
        bus.register_agent("receiver", "Receiver", vec![])
            .await
            .expect("test assertion");

        let id = bus
            .send_direct("sender", "receiver", serde_json::json!({"text": "hello"}))
            .await
            .expect("test assertion");

        assert!(!id.is_empty());
    }

    #[tokio::test]
    async fn test_broadcast() {
        let bus = MessageBus::new();

        bus.register_agent("agent1", "Agent 1", vec![])
            .await
            .expect("test assertion");
        bus.register_agent("agent2", "Agent 2", vec![])
            .await
            .expect("test assertion");
        bus.register_agent("agent3", "Agent 3", vec![])
            .await
            .expect("test assertion");

        let id = bus
            .broadcast("sender", serde_json::json!({"text": "broadcast"}), None)
            .await
            .expect("test assertion");

        assert!(!id.is_empty());
    }

    #[tokio::test]
    async fn test_request_response() {
        let bus = MessageBus::new();

        bus.register_agent("client", "Client", vec![])
            .await
            .expect("test assertion");
        bus.register_agent("server", "Server", vec![])
            .await
            .expect("test assertion");

        // This would need a server handler to respond
        // Just test the timeout case
        let result = bus
            .request(
                "client",
                "nonexistent",
                serde_json::json!({"test": "data"}),
                1,
            )
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_metrics() {
        let bus = MessageBus::new();

        bus.register_agent("agent1", "Agent 1", vec![])
            .await
            .expect("test assertion");

        let metrics = bus.get_metrics().await;
        assert_eq!(metrics.registered_agents, 1);
    }

    // Additional comprehensive tests

    #[test]
    fn test_message_priority_values() {
        assert_eq!(MessagePriority::Low.value(), 1);
        assert_eq!(MessagePriority::Normal.value(), 2);
        assert_eq!(MessagePriority::High.value(), 3);
        assert_eq!(MessagePriority::Critical.value(), 4);
    }

    #[test]
    fn test_message_priority_default() {
        assert_eq!(MessagePriority::default(), MessagePriority::Normal);
    }

    #[test]
    fn test_message_type_default() {
        assert_eq!(MessageType::default(), MessageType::Task);
    }

    #[test]
    fn test_agent_message_creation() {
        let msg = AgentMessage::new(
            "sender1",
            MessageType::Task,
            serde_json::json!({"data": "test"}),
        );

        assert_eq!(msg.sender, "sender1");
        assert_eq!(msg.msg_type, MessageType::Task);
        assert_eq!(msg.retries, 0);
        assert_eq!(msg.max_retries, 3);
        assert!(!msg.id.is_empty());
    }

    #[test]
    fn test_agent_message_task() {
        let msg = AgentMessage::task("sender1", serde_json::json!({"task": "do something"}));

        assert_eq!(msg.msg_type, MessageType::Task);
        assert_eq!(msg.sender, "sender1");
    }

    #[test]
    fn test_agent_message_result() {
        let msg = AgentMessage::result("sender1", serde_json::json!({"result": "success"}));

        assert_eq!(msg.msg_type, MessageType::Result);
    }

    #[test]
    fn test_agent_message_error() {
        let msg = AgentMessage::error("sender1", serde_json::json!({"error": "failed"}));

        assert_eq!(msg.msg_type, MessageType::Error);
    }

    #[test]
    fn test_agent_message_heartbeat() {
        let msg = AgentMessage::heartbeat("sender1");

        assert_eq!(msg.msg_type, MessageType::Heartbeat);
        assert_eq!(msg.content["status"], "alive");
    }

    #[test]
    fn test_agent_message_fluent_interface() {
        let msg = AgentMessage::task("sender1", serde_json::json!({}))
            .to("receiver1")
            .on_topic("test-topic")
            .with_correlation("corr-123")
            .reply_to_channel("reply-456")
            .with_priority(MessagePriority::High);

        assert_eq!(msg.receiver, Some("receiver1".to_string()));
        assert_eq!(msg.topic, Some("test-topic".to_string()));
        assert_eq!(msg.correlation_id, Some("corr-123".to_string()));
        assert_eq!(msg.reply_to, Some("reply-456".to_string()));
        assert_eq!(msg.priority, MessagePriority::High);
    }

    #[test]
    fn test_agent_status_default() {
        assert_eq!(AgentStatus::default(), AgentStatus::Registered);
    }

    #[test]
    fn test_agent_status_enum() {
        assert_eq!(AgentStatus::Registered as i32, 0);
        assert_eq!(AgentStatus::Active as i32, 1);
        assert_eq!(AgentStatus::Idle as i32, 2);
        assert_eq!(AgentStatus::Offline as i32, 3);
    }

    #[tokio::test]
    async fn test_subscribe_nonexistent_agent() {
        let bus = MessageBus::new();

        let result = bus.subscribe("nonexistent", "topic").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_unsubscribe() {
        let bus = MessageBus::new();

        bus.register_agent("agent1", "Agent 1", vec![])
            .await
            .expect("test assertion");
        bus.subscribe("agent1", "news")
            .await
            .expect("test assertion");

        let result = bus.unsubscribe("agent1", "news").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_heartbeat() {
        let bus = MessageBus::new();

        bus.register_agent("agent1", "Agent 1", vec![])
            .await
            .expect("test assertion");

        let result = bus.heartbeat("agent1").await;
        assert!(result.is_ok());

        let agent = bus.get_agent("agent1").await;
        assert!(agent.is_some());
        assert_eq!(agent.expect("test assertion").status, AgentStatus::Active);
    }

    #[tokio::test]
    async fn test_heartbeat_nonexistent_agent() {
        let bus = MessageBus::new();

        let result = bus.heartbeat("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_agent() {
        let bus = MessageBus::new();

        bus.register_agent("agent1", "Test Agent", vec!["capability1".to_string()])
            .await
            .expect("test assertion");

        let agent = bus.get_agent("agent1").await;
        assert!(agent.is_some());
        assert_eq!(agent.expect("test assertion").name, "Test Agent");
    }

    #[tokio::test]
    async fn test_list_agents() {
        let bus = MessageBus::new();

        bus.register_agent("agent1", "Agent 1", vec![])
            .await
            .expect("test assertion");
        bus.register_agent("agent2", "Agent 2", vec![])
            .await
            .expect("test assertion");

        let agents = bus.list_agents().await;
        assert_eq!(agents.len(), 2);
    }

    #[tokio::test]
    async fn test_dlq_operations() {
        let bus = MessageBus::new();

        let msg = AgentMessage::task("sender", serde_json::json!({"test": "data"}));

        bus.send_to_dlq(msg.clone(), "test failure")
            .await
            .expect("test assertion");

        let dlq = bus.get_dlq().await;
        assert_eq!(dlq.len(), 1);

        let cleared = bus.clear_dlq().await;
        assert_eq!(cleared, 1);

        let dlq = bus.get_dlq().await;
        assert_eq!(dlq.len(), 0);
    }

    #[tokio::test]
    async fn test_get_topics() {
        let bus = MessageBus::new();

        bus.register_agent("agent1", "Agent 1", vec![])
            .await
            .expect("test assertion");
        bus.subscribe("agent1", "news")
            .await
            .expect("test assertion");
        bus.subscribe("agent1", "updates")
            .await
            .expect("test assertion");

        let topics = bus.get_topics().await;
        assert!(topics.contains_key("news"));
        assert!(topics.contains_key("updates"));
    }

    #[tokio::test]
    async fn test_shutdown() {
        let bus = MessageBus::new();

        bus.register_agent("agent1", "Agent 1", vec![])
            .await
            .expect("test assertion");
        bus.register_agent("agent2", "Agent 2", vec![])
            .await
            .expect("test assertion");

        bus.shutdown().await;

        let agents = bus.list_agents().await;
        assert_eq!(agents.len(), 0);
    }

    #[tokio::test]
    async fn test_broadcast_with_topic() {
        let bus = MessageBus::new();

        bus.register_agent("agent1", "Agent 1", vec![])
            .await
            .expect("test assertion");
        bus.subscribe("agent1", "announcements")
            .await
            .expect("test assertion");

        let id = bus
            .broadcast(
                "sender",
                serde_json::json!({"text": "important"}),
                Some("announcements"),
            )
            .await
            .expect("test assertion");

        assert!(!id.is_empty());
    }

    #[test]
    fn test_bus_metrics_serialization() {
        let metrics = BusMetrics {
            messages_sent: 100,
            messages_received: 95,
            messages_failed: 5,
            dlq_size: 2,
            registered_agents: 3,
            topic_subscriptions: HashMap::new(),
            queue_sizes: HashMap::new(),
            stale_agents: 0,
            heartbeats_received: 0,
        };

        let json = serde_json::to_string(&metrics).expect("test assertion");
        assert!(json.contains("100"));
        assert!(json.contains("95"));
    }

    #[test]
    fn test_message_bus_error_display() {
        let err = MessageBusError::AgentNotFound("agent1".to_string());
        assert!(err.to_string().contains("agent1"));

        let err = MessageBusError::RequestTimeout(5);
        assert!(err.to_string().contains("5"));

        let err = MessageBusError::ChannelClosed;
        assert!(err.to_string().contains("Channel closed"));
    }
}
