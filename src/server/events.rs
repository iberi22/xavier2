//! Real-time event types for Xavier2 WebSocket streaming.

use serde::{Deserialize, Serialize};

/// Internal event broadcasted across the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeEvent {
    pub workspace_id: String,
    pub event_id: String,
    pub agent_id: String,
    pub project_id: Option<String>,
    pub event_type: String,
    pub timestamp: String,
    pub payload: serde_json::Value,
}

/// Messages sent from the client to the server via WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsMessage {
    /// Subscribe to events matching the given filters.
    /// If multiple filters are provided, they are combined with AND logic.
    Subscribe {
        #[serde(default)]
        agent_id: Option<String>,
        #[serde(default)]
        project_id: Option<String>,
        #[serde(default)]
        event_type: Option<String>,
    },
    /// Unsubscribe from events matching the given filters.
    Unsubscribe {
        #[serde(default)]
        agent_id: Option<String>,
        #[serde(default)]
        project_id: Option<String>,
        #[serde(default)]
        event_type: Option<String>,
    },
}

/// Messages sent from the server to the client via WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsEvent {
    /// A real-time event matching the client's subscriptions.
    Event(RealtimeEvent),
    /// Confirmation that a subscription/unsubscription was successful.
    SubscriptionConfirmed,
    /// An error message.
    Error { message: String },
}
