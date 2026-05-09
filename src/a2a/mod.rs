//! Agent-to-Agent (A2A) protocol primitives used by Xavier.
//!
//! The module provides:
//! - JSON-RPC 2.0 request/response types tailored to the A2A task flow.
//! - A small HTTP client for calling remote A2A agents.
//! - A server-side dispatcher that routes incoming RPC methods to a [`TaskHandler`].
//!
//! The streaming client currently exposes a placeholder stream and does not yet
//! implement full SSE parsing.

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::Stream;

/// Protocol types exchanged between A2A peers.
pub mod types {
    use super::*;

    /// JSON-RPC 2.0 request envelope.
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub struct JsonRpcRequest {
        /// Protocol version. A2A uses JSON-RPC `2.0`.
        pub jsonrpc: String,
        /// Caller-provided request identifier.
        pub id: String,
        /// Remote method name, such as `tasks/send`.
        pub method: String,
        /// Method-specific parameters.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub params: Option<serde_json::Value>,
    }

    /// JSON-RPC 2.0 response envelope.
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub struct JsonRpcResponse {
        /// Protocol version. A2A uses JSON-RPC `2.0`.
        pub jsonrpc: String,
        /// Identifier copied from the request.
        pub id: String,
        /// Successful response payload.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub result: Option<serde_json::Value>,
        /// Error payload when the request fails.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub error: Option<JsonRpcError>,
    }

    /// JSON-RPC error object.
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub struct JsonRpcError {
        /// JSON-RPC error code.
        pub code: i32,
        /// Human-readable error message.
        pub message: String,
        /// Optional structured error metadata.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub data: Option<serde_json::Value>,
    }

    /// Metadata published by an agent for discovery.
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub struct AgentCard {
        pub name: String,
        pub description: String,
        pub url: String,
        pub version: String,
        pub capabilities: AgentCapabilities,
        pub skills: Vec<Skill>,
        pub provider: Option<AgentProvider>,
        pub authentication: Option<Authentication>,
        pub tags: Vec<String>,
    }

    /// Provider metadata for an agent implementation.
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub struct AgentProvider {
        pub organization: String,
        pub url: Option<String>,
    }

    /// Authentication schemes expected by the agent endpoint.
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub struct Authentication {
        pub schemes: Vec<String>,
        pub credentials: Option<String>,
    }

    /// Feature flags advertised by an agent card.
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub struct AgentCapabilities {
        pub streaming: bool,
        pub push_notifications: bool,
        pub state_transitions: bool,
        pub attachments: bool,
        pub data_streaming: bool,
    }

    /// Capability-level unit of work that an agent can perform.
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub struct Skill {
        pub id: String,
        pub name: String,
        pub description: String,
    }

    /// A task exchanged between A2A agents.
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub struct Task {
        pub id: String,
        pub status: TaskStatus,
        pub messages: Vec<Message>,
        pub artifacts: Vec<Artifact>,
    }

    /// Lifecycle state of an A2A task.
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    #[serde(rename_all = "camelCase")]
    pub enum TaskStatus {
        Submitted,
        Working,
        InputRequired,
        Completed,
        Failed,
        Canceled,
    }

    /// Message exchanged as part of a task conversation.
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub struct Message {
        pub role: String,
        pub content: MessageContent,
    }

    /// Content payload supported by the A2A message model.
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    #[serde(untagged)]
    pub enum MessageContent {
        Text(String),
        Structured { parts: Vec<Part> },
    }

    /// Structured content fragment within a message.
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub struct Part {
        #[serde(rename = "type")]
        pub r#type: String,
        pub text: Option<String>,
    }

    /// Artifact emitted as task output.
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub struct Artifact {
        pub id: String,
        #[serde(rename = "type")]
        pub r#type: String,
    }

    /// Streaming event emitted by an agent while a task is running.
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    #[serde(tag = "type")]
    pub enum TaskStreamEvent {
        #[serde(rename = "taskStatusUpdate")]
        TaskStatusUpdate {
            task_id: String,
            status: TaskStatus,
            message: Option<Message>,
        },
        #[serde(rename = "taskArtifactUpdate")]
        TaskArtifactUpdate { task_id: String, artifact: Artifact },
        #[serde(rename = "taskMessage")]
        TaskMessage { task_id: String, message: Message },
        #[serde(rename = "error")]
        Error {
            task_id: String,
            error: JsonRpcError,
        },
    }

    /// Parameters used to subscribe to task streaming updates.
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub struct StreamingParams {
        pub task_id: String,
        #[serde(default)]
        pub include_messages: bool,
        #[serde(default)]
        pub include_artifacts: bool,
    }
}

/// Minimal A2A HTTP client used to talk to remote agents.
pub struct A2AClient {
    http_client: reqwest::Client,
    agent_cards: RwLock<HashMap<String, types::AgentCard>>,
}

impl A2AClient {
    /// Creates a client with an empty in-memory discovery registry.
    pub fn new() -> Self {
        Self {
            http_client: reqwest::Client::new(),
            agent_cards: RwLock::new(HashMap::new()),
        }
    }

    /// Registers an agent card in the local discovery cache.
    pub fn register_agent(&self, agent_id: &str, card: types::AgentCard) {
        self.agent_cards.write().insert(agent_id.to_string(), card);
    }

    /// Returns a locally registered agent card if present.
    pub fn get_registered_agent(&self, agent_id: &str) -> Option<types::AgentCard> {
        self.agent_cards.read().get(agent_id).cloned()
    }

    /// Fetches an agent card from `/.well-known/agent.json`.
    pub async fn discover_agent(&self, url: &str) -> Result<types::AgentCard, String> {
        let response = self
            .http_client
            .get(format!("{url}/.well-known/agent.json"))
            .send()
            .await
            .map_err(|e| format!("Failed to discover agent: {e}"))?;

        response
            .json::<types::AgentCard>()
            .await
            .map_err(|e| format!("Failed to parse agent card: {e}"))
    }

    /// Sends a `tasks/send` RPC call to a remote agent.
    pub async fn send_task(
        &self,
        agent_url: &str,
        task_id: &str,
        message: types::Message,
    ) -> Result<types::Task, String> {
        let request = types::JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: task_id.to_string(),
            method: "tasks/send".to_string(),
            params: Some(serde_json::json!({
                "taskId": task_id,
                "message": message
            })),
        };

        let response = self
            .http_client
            .post(format!("{agent_url}/rpc"))
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Failed to send task: {e}"))?;

        let rpc_response: types::JsonRpcResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {e}"))?;

        parse_task_response(rpc_response)
    }

    /// Sends a `tasks/get` RPC call to fetch the latest task state.
    pub async fn get_task(&self, agent_url: &str, task_id: &str) -> Result<types::Task, String> {
        let request = types::JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: task_id.to_string(),
            method: "tasks/get".to_string(),
            params: Some(serde_json::json!({ "taskId": task_id })),
        };

        let response = self
            .http_client
            .post(format!("{agent_url}/rpc"))
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Failed to get task: {e}"))?;

        let rpc_response: types::JsonRpcResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {e}"))?;

        parse_task_response(rpc_response)
    }

    /// Sends a `tasks/cancel` RPC call to a remote agent.
    pub async fn cancel_task(&self, agent_url: &str, task_id: &str) -> Result<(), String> {
        let request = types::JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: task_id.to_string(),
            method: "tasks/cancel".to_string(),
            params: Some(serde_json::json!({ "taskId": task_id })),
        };

        let response = self
            .http_client
            .post(format!("{agent_url}/rpc"))
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Failed to cancel task: {e}"))?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(format!("Cancel failed: {}", response.status()))
        }
    }

    /// Starts a task and returns a placeholder event stream.
    ///
    /// The server call is real, but the stream currently emits a single
    /// `Working` update locally until SSE support is implemented.
    pub async fn send_task_streaming(
        &self,
        agent_url: &str,
        task_id: &str,
        message: types::Message,
    ) -> Result<Pin<Box<dyn Stream<Item = types::TaskStreamEvent> + Send>>, String> {
        let request = types::JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: task_id.to_string(),
            method: "tasks/sendSubscribe".to_string(),
            params: Some(serde_json::json!({
                "taskId": task_id,
                "message": message
            })),
        };

        self.http_client
            .post(format!("{agent_url}/rpc"))
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Failed to start streaming: {e}"))?;

        let (tx, rx) = mpsc::channel(1);
        let task_id = task_id.to_string();

        tokio::spawn(async move {
            let _ = tx
                .send(types::TaskStreamEvent::TaskStatusUpdate {
                    task_id,
                    status: types::TaskStatus::Working,
                    message: None,
                })
                .await;
        });

        Ok(Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx)))
    }
}

impl Default for A2AClient {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_task_response(rpc_response: types::JsonRpcResponse) -> Result<types::Task, String> {
    match rpc_response.result {
        Some(result) => {
            serde_json::from_value(result).map_err(|e| format!("Failed to parse task: {e}"))
        }
        None => Err(rpc_response
            .error
            .map(|e| e.message)
            .unwrap_or_else(|| "Unknown error".to_string())),
    }
}

/// Server-side abstraction for handling A2A task RPCs.
pub trait TaskHandler: Send + Sync {
    /// Accepts a new task from a remote caller.
    fn handle_task<'a>(
        &'a self,
        task_id: String,
        message: types::Message,
    ) -> Pin<Box<dyn Future<Output = Result<types::Task, String>> + Send + 'a>>;

    /// Retrieves the latest known state for a task.
    fn get_task<'a>(
        &'a self,
        task_id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<types::Task, String>> + Send + 'a>>;

    /// Requests task cancellation.
    fn cancel_task<'a>(
        &'a self,
        task_id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;
}

/// JSON-RPC dispatcher for the A2A task methods supported by Xavier.
pub struct A2AServer {
    task_handler: Arc<dyn TaskHandler>,
    agent_card: types::AgentCard,
}

impl A2AServer {
    /// Creates a new server dispatcher bound to a task handler.
    pub fn new(agent_card: types::AgentCard, handler: Arc<dyn TaskHandler>) -> Self {
        Self {
            task_handler: handler,
            agent_card,
        }
    }

    /// Routes an incoming JSON-RPC request to the matching task operation.
    pub async fn handle_request(&self, request: types::JsonRpcRequest) -> types::JsonRpcResponse {
        match request.method.as_str() {
            "tasks/send" => self.handle_send(request).await,
            "tasks/get" => self.handle_get(request).await,
            "tasks/cancel" => self.handle_cancel(request).await,
            _ => method_not_found(request.id, &request.method),
        }
    }

    /// Returns the advertised agent card used by discovery endpoints.
    pub fn get_agent_card(&self) -> &types::AgentCard {
        &self.agent_card
    }

    async fn handle_send(&self, request: types::JsonRpcRequest) -> types::JsonRpcResponse {
        let Some(params) = request.params else {
            return invalid_request(request.id, "Invalid request");
        };

        let Some(task_id) = params.get("taskId").and_then(|value| value.as_str()) else {
            return invalid_params(request.id, "Missing taskId or message");
        };
        let Some(message_value) = params.get("message") else {
            return invalid_params(request.id, "Missing taskId or message");
        };

        let message = match serde_json::from_value::<types::Message>(message_value.clone()) {
            Ok(message) => message,
            Err(error) => {
                return invalid_params(request.id, &format!("Invalid params: {error}"));
            }
        };

        match self
            .task_handler
            .handle_task(task_id.to_string(), message)
            .await
        {
            Ok(task) => success_response(request.id, serde_json::to_value(task).unwrap()),
            Err(error) => internal_error(request.id, error),
        }
    }

    async fn handle_get(&self, request: types::JsonRpcRequest) -> types::JsonRpcResponse {
        let Some(params) = request.params else {
            return invalid_request(request.id, "Invalid request");
        };

        let Some(task_id) = params.get("taskId").and_then(|value| value.as_str()) else {
            return invalid_params(request.id, "Missing taskId");
        };

        match self.task_handler.get_task(task_id).await {
            Ok(task) => success_response(request.id, serde_json::to_value(task).unwrap()),
            Err(error) => internal_error(request.id, error),
        }
    }

    async fn handle_cancel(&self, request: types::JsonRpcRequest) -> types::JsonRpcResponse {
        let Some(params) = request.params else {
            return invalid_request(request.id, "Invalid request");
        };

        let Some(task_id) = params.get("taskId").and_then(|value| value.as_str()) else {
            return invalid_params(request.id, "Missing taskId");
        };

        match self.task_handler.cancel_task(task_id).await {
            Ok(()) => success_response(request.id, serde_json::json!({ "success": true })),
            Err(error) => internal_error(request.id, error),
        }
    }
}

fn success_response(id: String, result: serde_json::Value) -> types::JsonRpcResponse {
    types::JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: Some(result),
        error: None,
    }
}

fn invalid_request(id: String, message: &str) -> types::JsonRpcResponse {
    error_response(id, -32600, message.to_string())
}

fn invalid_params(id: String, message: &str) -> types::JsonRpcResponse {
    error_response(id, -32602, message.to_string())
}

fn internal_error(id: String, message: String) -> types::JsonRpcResponse {
    error_response(id, -32000, message)
}

fn method_not_found(id: String, method: &str) -> types::JsonRpcResponse {
    error_response(id, -32601, format!("Method not found: {method}"))
}

fn error_response(id: String, code: i32, message: String) -> types::JsonRpcResponse {
    types::JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: None,
        error: Some(types::JsonRpcError {
            code,
            message,
            data: None,
        }),
    }
}

/// Builds Xavier's default A2A discovery card.
pub fn create_xavier_agent_card() -> types::AgentCard {
    types::AgentCard {
        name: "Xavier".to_string(),
        description: "Cognitive Memory System with A2A Protocol support".to_string(),
        url: "http://localhost:8003".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: types::AgentCapabilities {
            streaming: true,
            push_notifications: false,
            state_transitions: true,
            attachments: true,
            data_streaming: true,
        },
        skills: vec![
            types::Skill {
                id: "memory".to_string(),
                name: "Memory Management".to_string(),
                description: "Store and retrieve information from memory".to_string(),
            },
            types::Skill {
                id: "code".to_string(),
                name: "Code Analysis".to_string(),
                description: "Analyze and search codebases".to_string(),
            },
            types::Skill {
                id: "tasks".to_string(),
                name: "Task Management".to_string(),
                description: "Create and manage tasks".to_string(),
            },
        ],
        provider: Some(types::AgentProvider {
            organization: "Southwest AI Labs".to_string(),
            url: Some("https://github.com/southwest-ai-labs".to_string()),
        }),
        authentication: Some(types::Authentication {
            schemes: vec!["Bearer".to_string()],
            credentials: None,
        }),
        tags: vec![
            "memory".to_string(),
            "cognitive".to_string(),
            "a2a".to_string(),
        ],
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MessageType {
    Request,
    Response,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct A2AMessage {
    pub sender: String,
    pub receiver: String,
    pub message_type: MessageType,
    pub data: String,
    pub id: String,
}

impl A2AMessage {
    pub fn new(sender: String, receiver: String, message_type: MessageType, data: String) -> Self {
        Self {
            sender,
            receiver,
            message_type,
            data,
            id: ulid::Ulid::new().to_string(),
        }
    }
}

#[derive(Default)]
pub struct A2AProtocol;

impl A2AProtocol {
    pub fn new() -> Self {
        Self
    }

    pub fn is_valid(&self) -> bool {
        true
    }

    pub fn validate_message(&self, payload: &str) -> Result<(), String> {
        if payload.trim().is_empty() {
            Err("payload cannot be empty".to_string())
        } else {
            Ok(())
        }
    }

    pub async fn handle_request(
        &self,
        sender: String,
        payload: String,
    ) -> Result<A2AMessage, String> {
        self.validate_message(&payload)?;
        Ok(A2AMessage::new(
            sender,
            "xavier".to_string(),
            MessageType::Response,
            payload,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct TestTaskHandler {
        tasks: RwLock<HashMap<String, types::Task>>,
    }

    impl TestTaskHandler {
        fn new() -> Self {
            Self {
                tasks: RwLock::new(HashMap::new()),
            }
        }
    }

    impl TaskHandler for TestTaskHandler {
        fn handle_task<'a>(
            &'a self,
            task_id: String,
            message: types::Message,
        ) -> Pin<Box<dyn Future<Output = Result<types::Task, String>> + Send + 'a>> {
            Box::pin(async move {
                let task = types::Task {
                    id: task_id.clone(),
                    status: types::TaskStatus::Submitted,
                    messages: vec![message],
                    artifacts: vec![],
                };
                self.tasks.write().insert(task_id, task.clone());
                Ok(task)
            })
        }

        fn get_task<'a>(
            &'a self,
            task_id: &'a str,
        ) -> Pin<Box<dyn Future<Output = Result<types::Task, String>> + Send + 'a>> {
            Box::pin(async move {
                self.tasks
                    .read()
                    .get(task_id)
                    .cloned()
                    .ok_or_else(|| format!("Task not found: {task_id}"))
            })
        }

        fn cancel_task<'a>(
            &'a self,
            task_id: &'a str,
        ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
            Box::pin(async move {
                let mut tasks = self.tasks.write();
                let Some(task) = tasks.get_mut(task_id) else {
                    return Err(format!("Task not found: {task_id}"));
                };
                task.status = types::TaskStatus::Canceled;
                Ok(())
            })
        }
    }

    fn test_message() -> types::Message {
        types::Message {
            role: "user".to_string(),
            content: types::MessageContent::Text("hello xavier".to_string()),
        }
    }

    fn test_server() -> A2AServer {
        A2AServer::new(create_xavier_agent_card(), Arc::new(TestTaskHandler::new()))
    }

    #[test]
    fn create_xavier_agent_card_exposes_expected_metadata() {
        let card = create_xavier_agent_card();

        assert_eq!(card.name, "Xavier");
        assert_eq!(card.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(card.skills.len(), 3);
        assert!(card.capabilities.streaming);
        assert!(card.capabilities.attachments);
        assert!(card.tags.iter().any(|tag| tag == "a2a"));
    }

    #[test]
    fn task_status_uses_protocol_friendly_serialization() {
        let serialized = serde_json::to_string(&types::TaskStatus::InputRequired).unwrap();
        assert_eq!(serialized, "\"inputRequired\"");

        let deserialized: types::TaskStatus = serde_json::from_str("\"completed\"").unwrap();
        assert_eq!(deserialized, types::TaskStatus::Completed);
    }

    #[test]
    fn type_fields_serialize_as_json_type() {
        let artifact = types::Artifact {
            id: "artifact-1".to_string(),
            r#type: "document".to_string(),
        };

        let json = serde_json::to_value(&artifact).unwrap();
        assert_eq!(json["type"], "document");
    }

    #[test]
    fn client_registry_returns_registered_agent() {
        let client = A2AClient::new();
        let card = create_xavier_agent_card();
        client.register_agent("xavier", card.clone());

        assert_eq!(client.get_registered_agent("xavier"), Some(card));
        assert_eq!(client.get_registered_agent("missing"), None);
    }

    #[tokio::test]
    async fn handle_send_returns_serialized_task() {
        let server = test_server();
        let request = types::JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: "req-1".to_string(),
            method: "tasks/send".to_string(),
            params: Some(serde_json::json!({
                "taskId": "task-1",
                "message": test_message()
            })),
        };

        let response = server.handle_request(request).await;
        let task: types::Task = serde_json::from_value(response.result.unwrap()).unwrap();

        assert_eq!(response.error, None);
        assert_eq!(task.id, "task-1");
        assert_eq!(task.status, types::TaskStatus::Submitted);
        assert_eq!(task.messages.len(), 1);
    }

    #[tokio::test]
    async fn handle_send_rejects_invalid_message_payload() {
        let server = test_server();
        let request = types::JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: "req-2".to_string(),
            method: "tasks/send".to_string(),
            params: Some(serde_json::json!({
                "taskId": "task-1",
                "message": { "role": 42 }
            })),
        };

        let response = server.handle_request(request).await;
        let error = response.error.unwrap();

        assert_eq!(error.code, -32602);
        assert!(error.message.contains("Invalid params"));
    }

    #[tokio::test]
    async fn handle_get_returns_missing_task_error() {
        let server = test_server();
        let request = types::JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: "req-3".to_string(),
            method: "tasks/get".to_string(),
            params: Some(serde_json::json!({ "taskId": "missing-task" })),
        };

        let response = server.handle_request(request).await;
        let error = response.error.unwrap();

        assert_eq!(error.code, -32000);
        assert!(error.message.contains("Task not found"));
    }

    #[tokio::test]
    async fn handle_cancel_acknowledges_successful_cancellation() {
        let server = test_server();
        let send_request = types::JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: "req-4".to_string(),
            method: "tasks/send".to_string(),
            params: Some(serde_json::json!({
                "taskId": "task-2",
                "message": test_message()
            })),
        };
        let cancel_request = types::JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: "req-5".to_string(),
            method: "tasks/cancel".to_string(),
            params: Some(serde_json::json!({ "taskId": "task-2" })),
        };

        let _ = server.handle_request(send_request).await;
        let response = server.handle_request(cancel_request).await;

        assert_eq!(response.error, None);
        assert_eq!(response.result.unwrap()["success"], true);
    }

    #[tokio::test]
    async fn handle_unknown_method_returns_method_not_found() {
        let server = test_server();
        let request = types::JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: "req-6".to_string(),
            method: "tasks/unknown".to_string(),
            params: None,
        };

        let response = server.handle_request(request).await;
        let error = response.error.unwrap();

        assert_eq!(error.code, -32601);
        assert_eq!(error.message, "Method not found: tasks/unknown");
    }
}
