use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

pub const MAX_SESSION_CHECKPOINT_BYTES: usize = 2048;
const MAX_SUMMARY_CHARS: usize = 320;
const MAX_ITEMS_PER_SECTION: usize = 10;
const MAX_ITEM_CHARS: usize = 120;

#[derive(Debug, Clone, Default)]
pub struct SessionCheckpointInput {
    pub summary: String,
    pub file_edits: Vec<String>,
    pub git_operations: Vec<String>,
    pub tasks: Vec<String>,
}

impl SessionCheckpointInput {
    pub fn new(
        summary: impl Into<String>,
        file_edits: Vec<String>,
        git_operations: Vec<String>,
        tasks: Vec<String>,
    ) -> Self {
        Self {
            summary: summary.into(),
            file_edits,
            git_operations,
            tasks,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionCheckpoint {
    pub session_id: String,
    pub timestamp: u64,
    pub summary: String,
    pub file_edits: Vec<String>,
    pub git_operations: Vec<String>,
    pub tasks: Vec<String>,
}

impl SessionCheckpoint {
    pub fn new(session_id: impl Into<String>, input: SessionCheckpointInput) -> Result<Self> {
        let session_id = session_id.into();
        validate_session_id(&session_id)?;

        let mut checkpoint = Self {
            session_id,
            timestamp: unix_timestamp(),
            summary: truncate(&input.summary, MAX_SUMMARY_CHARS),
            file_edits: normalize_items(input.file_edits),
            git_operations: normalize_items(input.git_operations),
            tasks: normalize_items(input.tasks),
        };

        checkpoint.compact_to_fit()?;
        Ok(checkpoint)
    }

    pub fn from_session(
        session_id: impl Into<String>,
        summary: impl Into<String>,
        file_edits: Vec<String>,
        git_operations: Vec<String>,
        tasks: Vec<String>,
    ) -> Result<Self> {
        Self::new(
            session_id,
            SessionCheckpointInput::new(summary, file_edits, git_operations, tasks),
        )
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(self).context("failed to serialize session checkpoint")
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let checkpoint: Self =
            serde_json::from_slice(bytes).context("failed to deserialize session checkpoint")?;
        validate_session_id(&checkpoint.session_id)?;
        if checkpoint.size_bytes()? > MAX_SESSION_CHECKPOINT_BYTES {
            bail!(
                "session checkpoint exceeds {} bytes",
                MAX_SESSION_CHECKPOINT_BYTES
            );
        }
        Ok(checkpoint)
    }

    pub fn size_bytes(&self) -> Result<usize> {
        Ok(self.to_bytes()?.len())
    }

    pub fn is_within_budget(&self) -> Result<bool> {
        Ok(self.size_bytes()? <= MAX_SESSION_CHECKPOINT_BYTES)
    }

    fn compact_to_fit(&mut self) -> Result<()> {
        if self.is_within_budget()? {
            return Ok(());
        }

        while self.size_bytes()? > MAX_SESSION_CHECKPOINT_BYTES {
            if pop_longest(&mut self.tasks)
                || pop_longest(&mut self.git_operations)
                || pop_longest(&mut self.file_edits)
            {
                continue;
            }

            if self.summary.len() > 48 {
                let next_len = (self.summary.len() * 3 / 4).max(48);
                self.summary = truncate(&self.summary, next_len);
                continue;
            }

            bail!(
                "unable to compact session checkpoint below {} bytes",
                MAX_SESSION_CHECKPOINT_BYTES
            );
        }

        Ok(())
    }
}

fn normalize_items(items: Vec<String>) -> Vec<String> {
    items
        .into_iter()
        .filter_map(|item| {
            let trimmed = item.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(truncate(trimmed, MAX_ITEM_CHARS))
            }
        })
        .take(MAX_ITEMS_PER_SECTION)
        .collect()
}

fn pop_longest(values: &mut Vec<String>) -> bool {
    let Some((index, _)) = values
        .iter()
        .enumerate()
        .max_by_key(|(_, value)| value.len())
    else {
        return false;
    };
    values.remove(index);
    true
}

fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }

    if max_chars <= 3 {
        return ".".repeat(max_chars);
    }

    let head: String = value.chars().take(max_chars - 3).collect();
    format!("{head}...")
}

fn unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn validate_session_id(session_id: &str) -> Result<()> {
    if session_id.is_empty() {
        bail!("session_id cannot be empty");
    }

    if !session_id
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
    {
        bail!("session_id contains unsupported characters");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_checkpoint_round_trips() {
        let checkpoint = SessionCheckpoint::from_session(
            "session_123",
            "Implemented session continuity checkpointing.",
            vec!["src/checkpoint/session.rs".to_string()],
            vec!["git add src/checkpoint/session.rs".to_string()],
            vec!["Implement Phase 3 checkpoint system".to_string()],
        )
        .expect("test assertion");

        let restored =
            SessionCheckpoint::from_bytes(&checkpoint.to_bytes().expect("test assertion"))
                .expect("test assertion");

        assert_eq!(restored.session_id, "session_123");
        assert_eq!(restored.file_edits.len(), 1);
        assert_eq!(restored.git_operations.len(), 1);
        assert_eq!(restored.tasks.len(), 1);
    }

    #[test]
    fn session_checkpoint_stays_under_budget() {
        let large_items = (0..20)
            .map(|idx| format!("item-{idx}-{}", "x".repeat(400)))
            .collect::<Vec<_>>();

        let checkpoint = SessionCheckpoint::from_session(
            "session_budget",
            "y".repeat(2_000),
            large_items.clone(),
            large_items.clone(),
            large_items,
        )
        .expect("test assertion");

        assert!(checkpoint.is_within_budget().expect("test assertion"));
        assert!(checkpoint.size_bytes().expect("test assertion") <= MAX_SESSION_CHECKPOINT_BYTES);
        assert!(!checkpoint.file_edits.is_empty());
    }

    #[test]
    fn invalid_session_id_is_rejected() {
        let err = SessionCheckpoint::from_session(
            "bad/session",
            "summary",
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
        .unwrap_err();

        assert!(err.to_string().contains("unsupported characters"));
    }
}
