pub mod session;
pub mod state;

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::memory::store::MemoryStore;

pub use session::{SessionCheckpoint, SessionCheckpointInput, MAX_SESSION_CHECKPOINT_BYTES};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub task_id: String,
    pub name: String,
    pub data: serde_json::Value,
}

impl Checkpoint {
    pub fn new(task_id: String, name: String, data: serde_json::Value) -> Self {
        Self {
            task_id,
            name,
            data,
        }
    }
}

pub struct CheckpointManager {
    checkpoints: RwLock<HashMap<String, Checkpoint>>,
    workspace_id: Option<String>,
    store: Option<Arc<dyn MemoryStore>>,
}

impl Default for CheckpointManager {
    fn default() -> Self {
        Self::new()
    }
}

impl CheckpointManager {
    pub fn new() -> Self {
        Self {
            checkpoints: RwLock::new(HashMap::new()),
            workspace_id: None,
            store: None,
        }
    }

    pub fn with_store(workspace_id: impl Into<String>, store: Arc<dyn MemoryStore>) -> Self {
        Self {
            checkpoints: RwLock::new(HashMap::new()),
            workspace_id: Some(workspace_id.into()),
            store: Some(store),
        }
    }

    fn key(task_id: &str, name: &str) -> String {
        format!("{task_id}::{name}")
    }

    pub async fn save(&self, checkpoint: Checkpoint) -> Result<()> {
        self.checkpoints.write().await.insert(
            Self::key(&checkpoint.task_id, &checkpoint.name),
            checkpoint.clone(),
        );
        if let (Some(workspace_id), Some(store)) = (&self.workspace_id, &self.store) {
            store.save_checkpoint(workspace_id, checkpoint).await?;
        }
        Ok(())
    }

    pub async fn load(&self, task_id: String, name: String) -> Result<Option<Checkpoint>> {
        if let Some(checkpoint) = self
            .checkpoints
            .read()
            .await
            .get(&Self::key(&task_id, &name))
            .cloned()
        {
            return Ok(Some(checkpoint));
        }

        if let (Some(workspace_id), Some(store)) = (&self.workspace_id, &self.store) {
            let checkpoint = store.load_checkpoint(workspace_id, &task_id, &name).await?;
            if let Some(checkpoint) = checkpoint.clone() {
                self.checkpoints
                    .write()
                    .await
                    .insert(Self::key(&task_id, &name), checkpoint);
            }
            return Ok(checkpoint);
        }

        Ok(None)
    }

    pub async fn list(&self, task_id: String) -> Result<Vec<Checkpoint>> {
        if let (Some(workspace_id), Some(store)) = (&self.workspace_id, &self.store) {
            let checkpoints = store.list_checkpoints(workspace_id, &task_id).await?;
            let mut cache = self.checkpoints.write().await;
            for checkpoint in &checkpoints {
                cache.insert(
                    Self::key(&checkpoint.task_id, &checkpoint.name),
                    checkpoint.clone(),
                );
            }
            return Ok(checkpoints);
        }

        Ok(self
            .checkpoints
            .read()
            .await
            .values()
            .filter(|checkpoint| checkpoint.task_id == task_id)
            .cloned()
            .collect())
    }

    pub async fn delete(&self, task_id: String, name: String) -> Result<()> {
        self.checkpoints
            .write()
            .await
            .remove(&Self::key(&task_id, &name));
        if let (Some(workspace_id), Some(store)) = (&self.workspace_id, &self.store) {
            store
                .delete_checkpoint(workspace_id, &task_id, &name)
                .await?;
        }
        Ok(())
    }
}
