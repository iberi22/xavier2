use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::agents::runtime::ConversationMessage;

pub const MAX_CHECKPOINT_BYTES: usize = 2048;
const MAX_RECENT_MESSAGES: usize = 4;
const MAX_MESSAGE_CHARS: usize = 240;
const MAX_TASKS: usize = 8;
const MAX_TASK_CHARS: usize = 80;
const MAX_TOOLS: usize = 4;
const MAX_TOOL_NAME_CHARS: usize = 32;
const MAX_TOOL_STATE_CHARS: usize = 120;
const MAX_SUMMARY_CHARS: usize = 320;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCheckpoint {
    pub session_id: String,
    pub summary: String,
    pub recent_messages: Vec<CompactMessage>,
    pub pending_tasks: Vec<String>,
    pub tool_state: HashMap<String, String>,
    pub checkpoint_timestamp: chrono::DateTime<chrono::Utc>,
}

impl SessionCheckpoint {
    pub fn from_state(state: &crate::checkpoint::state::CheckpointState) -> Self {
        Self {
            session_id: state.session_id.clone(),
            summary: summarize_messages(&state.messages),
            recent_messages: state
                .messages
                .iter()
                .rev()
                .take(MAX_RECENT_MESSAGES)
                .cloned()
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .map(CompactMessage::from)
                .collect(),
            pending_tasks: state
                .task_queue
                .iter()
                .take(MAX_TASKS)
                .map(|task| truncate(task, MAX_TASK_CHARS))
                .collect(),
            tool_state: state
                .tools_state
                .iter()
                .take(MAX_TOOLS)
                .map(|(name, value)| {
                    (
                        truncate(name, MAX_TOOL_NAME_CHARS),
                        truncate_compact_json(value, MAX_TOOL_STATE_CHARS),
                    )
                })
                .collect(),
            checkpoint_timestamp: state.checkpoint_timestamp,
        }
    }

    pub fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        Ok(serde_json::to_vec(self)?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactMessage {
    pub role: String,
    pub content: String,
}

impl From<ConversationMessage> for CompactMessage {
    fn from(value: ConversationMessage) -> Self {
        Self {
            role: format!("{:?}", value.role).to_lowercase(),
            content: truncate(&value.content, MAX_MESSAGE_CHARS),
        }
    }
}

fn summarize_messages(messages: &[ConversationMessage]) -> String {
    let merged = messages
        .iter()
        .rev()
        .take(2)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|message| {
            format!(
                "{}: {}",
                format!("{:?}", message.role).to_lowercase(),
                truncate(&message.content, 120)
            )
        })
        .collect::<Vec<_>>()
        .join(" | ");

    truncate(&merged, MAX_SUMMARY_CHARS)
}

fn truncate_compact_json(value: &serde_json::Value, max_chars: usize) -> String {
    let compact = serde_json::to_string(value).unwrap_or_else(|_| "\"<invalid-json>\"".to_string());
    truncate(&compact, max_chars)
}

fn truncate(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let truncated: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() && max_chars > 1 {
        format!(
            "{}…",
            truncated.chars().take(max_chars - 1).collect::<String>()
        )
    } else {
        truncated
    }
}
