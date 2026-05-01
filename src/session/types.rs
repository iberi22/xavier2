use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Incoming session event from OpenClaw webhook
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionEventType {
    SessionStart,
    SessionEnd,
    Message,
    ToolCall,
    ToolResult,
    Error,
}

/// Raw session event payload from OpenClaw
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEvent {
    pub session_id: String,
    pub event_type: SessionEventType,
    pub timestamp: DateTime<Utc>,
    pub content: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

impl SessionEvent {
    pub fn content_preview(&self) -> String {
        self.content
            .as_ref()
            .map(|c| {
                if c.len() > 200 {
                    format!("{}...", &c[..200])
                } else {
                    c.clone()
                }
            })
            .unwrap_or_default()
    }
}
