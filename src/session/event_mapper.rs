use crate::session::types::SessionEvent;
use chrono::{DateTime, Utc};

/// A single entry in a panel/thread conversation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PanelThreadEntry {
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub session_id: String,
    pub event_type: String,
}

impl PanelThreadEntry {
    pub fn from_session_event(event: &SessionEvent) -> Option<Self> {
        let role = match event.event_type {
            crate::session::types::SessionEventType::Message => "user",
            crate::session::types::SessionEventType::ToolCall => "tool",
            crate::session::types::SessionEventType::ToolResult => "assistant",
            crate::session::types::SessionEventType::SessionStart => return None,
            crate::session::types::SessionEventType::SessionEnd => return None,
            crate::session::types::SessionEventType::Error => "system",
        };

        let content = event.content.clone().unwrap_or_default();
        if content.is_empty() {
            return None;
        }

        Some(Self {
            role: role.to_string(),
            content,
            timestamp: event.timestamp,
            session_id: event.session_id.clone(),
            event_type: serde_json::to_string(&event.event_type).unwrap_or_default(),
        })
    }
}
