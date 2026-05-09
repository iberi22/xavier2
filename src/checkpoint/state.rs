use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::fs;
use tracing::{debug, info, warn};

use crate::agents::runtime::ConversationMessage;

const CHECKPOINT_ROOT: &str = "checkpoints";
const RETAINED_CHECKPOINTS: usize = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointState {
    pub session_id: String,
    pub messages: Vec<ConversationMessage>,
    pub task_queue: Vec<String>,
    pub tools_state: HashMap<String, serde_json::Value>,
    pub checkpoint_timestamp: DateTime<Utc>,
}

impl CheckpointState {
    pub fn new(
        session_id: impl Into<String>,
        messages: Vec<ConversationMessage>,
        task_queue: Vec<String>,
        tools_state: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            messages,
            task_queue,
            tools_state,
            checkpoint_timestamp: Utc::now(),
        }
    }
}

pub async fn save_checkpoint(state: &CheckpointState) -> Result<PathBuf> {
    save_checkpoint_in_dir(Path::new(CHECKPOINT_ROOT), state).await
}

pub async fn load_latest_checkpoint(session_id: &str) -> Result<CheckpointState> {
    load_latest_checkpoint_in_dir(Path::new(CHECKPOINT_ROOT), session_id).await
}

pub async fn is_session_restorable(session_id: &str) -> Result<bool> {
    is_session_restorable_in_dir(Path::new(CHECKPOINT_ROOT), session_id).await
}

async fn save_checkpoint_in_dir(root: &Path, state: &CheckpointState) -> Result<PathBuf> {
    validate_session_id(&state.session_id)?;

    let session_dir = root.join(&state.session_id);
    fs::create_dir_all(&session_dir).await.with_context(|| {
        format!(
            "failed to create checkpoint directory {}",
            session_dir.display()
        )
    })?;

    let filename = format!("{}.json", format_timestamp(&state.checkpoint_timestamp));
    let checkpoint_path = session_dir.join(filename);
    let payload =
        serde_json::to_vec_pretty(state).context("failed to serialize checkpoint state")?;

    fs::write(&checkpoint_path, payload)
        .await
        .with_context(|| {
            format!(
                "failed to write checkpoint file {}",
                checkpoint_path.display()
            )
        })?;

    info!(
        session_id = %state.session_id,
        path = %checkpoint_path.display(),
        "checkpoint saved"
    );

    rotate_checkpoints(&session_dir).await?;

    Ok(checkpoint_path)
}

async fn load_latest_checkpoint_in_dir(root: &Path, session_id: &str) -> Result<CheckpointState> {
    validate_session_id(session_id)?;

    let session_dir = root.join(session_id);
    let latest_path = latest_checkpoint_path(&session_dir)
        .await?
        .ok_or_else(|| anyhow!("no checkpoints found for session {session_id}"))?;

    debug!(
        session_id = %session_id,
        path = %latest_path.display(),
        "loading latest checkpoint"
    );

    let payload = fs::read_to_string(&latest_path)
        .await
        .with_context(|| format!("failed to read checkpoint file {}", latest_path.display()))?;

    serde_json::from_str(&payload).with_context(|| {
        format!(
            "failed to deserialize checkpoint file {}",
            latest_path.display()
        )
    })
}

async fn is_session_restorable_in_dir(root: &Path, session_id: &str) -> Result<bool> {
    validate_session_id(session_id)?;

    let session_dir = root.join(session_id);
    Ok(latest_checkpoint_path(&session_dir).await?.is_some())
}

async fn rotate_checkpoints(session_dir: &Path) -> Result<()> {
    let checkpoint_files = checkpoint_files_sorted(session_dir).await?;

    for stale_path in checkpoint_files.iter().skip(RETAINED_CHECKPOINTS) {
        fs::remove_file(stale_path)
            .await
            .with_context(|| format!("failed to remove old checkpoint {}", stale_path.display()))?;
        warn!(path = %stale_path.display(), "removed stale checkpoint during rotation");
    }

    Ok(())
}

async fn latest_checkpoint_path(session_dir: &Path) -> Result<Option<PathBuf>> {
    Ok(checkpoint_files_sorted(session_dir)
        .await?
        .into_iter()
        .next())
}

async fn checkpoint_files_sorted(session_dir: &Path) -> Result<Vec<PathBuf>> {
    if !fs::try_exists(session_dir).await.with_context(|| {
        format!(
            "failed to check checkpoint directory {}",
            session_dir.display()
        )
    })? {
        return Ok(Vec::new());
    }

    let mut entries = fs::read_dir(session_dir).await.with_context(|| {
        format!(
            "failed to read checkpoint directory {}",
            session_dir.display()
        )
    })?;
    let mut checkpoint_files = Vec::new();

    while let Some(entry) = entries.next_entry().await.with_context(|| {
        format!(
            "failed to iterate checkpoint directory {}",
            session_dir.display()
        )
    })? {
        let path = entry.path();
        if !entry.file_type().await?.is_file() {
            continue;
        }

        if path.extension().and_then(|value| value.to_str()) == Some("json") {
            checkpoint_files.push(path);
        }
    }

    checkpoint_files.sort_by(|left, right| right.file_name().cmp(&left.file_name()));
    Ok(checkpoint_files)
}

fn validate_session_id(session_id: &str) -> Result<()> {
    if session_id.is_empty() {
        bail!("session_id cannot be empty");
    }

    let valid = session_id
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'));

    if !valid {
        bail!("session_id contains unsupported characters");
    }

    Ok(())
}

fn format_timestamp(timestamp: &DateTime<Utc>) -> String {
    timestamp.format("%Y%m%dT%H%M%S%.3fZ").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::runtime::{ConversationMessage, MessageRole};

    async fn temp_checkpoint_root() -> PathBuf {
        let root = std::env::temp_dir()
            .join("xavier-checkpoint-tests")
            .join(ulid::Ulid::new().to_string());
        fs::create_dir_all(&root).await.unwrap();
        root
    }

    fn sample_state(session_id: &str) -> CheckpointState {
        CheckpointState {
            session_id: session_id.to_string(),
            messages: vec![ConversationMessage {
                id: "msg-1".to_string(),
                role: MessageRole::User,
                content: "restore me".to_string(),
                timestamp: Utc::now(),
            }],
            task_queue: vec!["task-a".to_string(), "task-b".to_string()],
            tools_state: HashMap::from([(
                "search".to_string(),
                serde_json::json!({ "cursor": 2 }),
            )]),
            checkpoint_timestamp: Utc::now(),
        }
    }

    #[tokio::test]
    async fn saves_and_loads_latest_checkpoint() {
        let root = temp_checkpoint_root().await;
        let mut first = sample_state("session_a");
        first.checkpoint_timestamp = Utc::now() - chrono::Duration::seconds(1);
        save_checkpoint_in_dir(&root, &first).await.unwrap();

        let second = sample_state("session_a");
        save_checkpoint_in_dir(&root, &second).await.unwrap();

        let restored = load_latest_checkpoint_in_dir(&root, "session_a")
            .await
            .unwrap();
        assert_eq!(restored.session_id, "session_a");
        assert_eq!(restored.messages.len(), 1);
        assert_eq!(restored.task_queue.len(), 2);
    }

    #[tokio::test]
    async fn rotates_old_checkpoints() {
        let root = temp_checkpoint_root().await;

        for offset in 0..4 {
            let mut state = sample_state("session_b");
            state.checkpoint_timestamp = Utc::now() + chrono::Duration::milliseconds(offset);
            save_checkpoint_in_dir(&root, &state).await.unwrap();
        }

        let session_dir = root.join("session_b");
        let files = checkpoint_files_sorted(&session_dir).await.unwrap();
        assert_eq!(files.len(), RETAINED_CHECKPOINTS);
    }

    #[tokio::test]
    async fn reports_restorable_sessions() {
        let root = temp_checkpoint_root().await;
        assert!(!is_session_restorable_in_dir(&root, "session_c")
            .await
            .unwrap());

        let state = sample_state("session_c");
        save_checkpoint_in_dir(&root, &state).await.unwrap();

        assert!(is_session_restorable_in_dir(&root, "session_c")
            .await
            .unwrap());
    }
}
