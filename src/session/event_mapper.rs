use crate::session::types::{SessionEvent, SessionEventType};
use chrono::{DateTime, Utc};
use tracing::info;

/// A single entry in a panel/thread conversation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PanelThreadEntry {
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub session_id: String,
    pub event_type: String,
}

/// Maps a SessionEvent to a PanelThreadEntry.
/// Returns None for SessionStart/SessionEnd events (which are metadata only).
pub fn map_to_panel_thread(event: SessionEvent) -> Option<PanelThreadEntry> {
    let role = match event.event_type {
        SessionEventType::Message => "user",
        SessionEventType::ToolCall => "tool",
        SessionEventType::ToolResult => "assistant",
        SessionEventType::SessionStart => return None,
        SessionEventType::SessionEnd => return None,
        SessionEventType::Error => "system",
    };

    let content = event.content.clone().unwrap_or_default();
    if content.is_empty() {
        return None;
    }

    info!(
        session_id = %event.session_id,
        role = %role,
        content_len = content.len(),
        "mapping session event to panel thread"
    );

    Some(PanelThreadEntry {
        role: role.to_string(),
        content,
        timestamp: event.timestamp,
        session_id: event.session_id,
        event_type: serde_json::to_string(&event.event_type).unwrap_or_default(),
    })
}

impl PanelThreadEntry {
    pub fn from_session_event(event: &SessionEvent) -> Option<Self> {
        let role = match event.event_type {
            SessionEventType::Message => "user",
            SessionEventType::ToolCall => "tool",
            SessionEventType::ToolResult => "assistant",
            SessionEventType::SessionStart => return None,
            SessionEventType::SessionEnd => return None,
            SessionEventType::Error => "system",
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
