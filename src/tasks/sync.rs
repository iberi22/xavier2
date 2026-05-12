//! Planka Sync Service - Bidirectional sync between internal tasks and Planka
//! Tasks are stored in Xavier, Planka is just a view/sync target

use crate::tasks::models::{Project, Task, TaskStatus};
use crate::tasks::store::{InMemoryTaskStore, TaskService, TaskStore};
use crate::tools::kanban::PlankaClient;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Sync status between Xavier and Planka
#[derive(Debug, Clone)]
pub enum SyncStatus {
    Synced,
    PendingUpload,
    PendingUpdate,
    PendingDelete,
    Conflict,
}

/// Planka sync configuration
#[derive(Debug, Clone)]
pub struct PlankaSyncConfig {
    pub enabled: bool,
    pub auto_sync: bool,
    pub sync_interval_secs: u64,
}

impl Default for PlankaSyncConfig {
    fn default() -> Self {
        Self {
            enabled: false, // Start disabled, enable after setup
            auto_sync: true,
            sync_interval_secs: 60,
        }
    }
}

/// Planka sync service
pub struct PlankaSyncService<S: TaskStore> {
    task_service: TaskService<S>,
    planka_client: Option<PlankaClient>,
    config: RwLock<PlankaSyncConfig>,
}

impl<S: TaskStore> PlankaSyncService<S> {
    pub fn new(task_store: Arc<S>, planka_client: Option<PlankaClient>) -> Self {
        let task_service = TaskService::new(task_store);
        Self {
            task_service,
            planka_client,
            config: RwLock::new(PlankaSyncConfig::default()),
        }
    }

    /// Enable Planka sync
    pub async fn enable(&self) {
        let mut config = self.config.write().await;
        config.enabled = true;
        info!("[OK] Planka sync enabled");
    }

    /// Disable Planka sync
    pub async fn disable(&self) {
        let mut config = self.config.write().await;
        config.enabled = false;
        info!("[OK] Planka sync disabled");
    }

    /// Check if sync is enabled
    pub async fn is_enabled(&self) -> bool {
        let config = self.config.read().await;
        config.enabled
    }

    /// Get Planka client (lazy init)
    pub fn with_planka<F, R>(&self, f: F) -> Result<R, String>
    where
        F: FnOnce(&PlankaClient) -> Result<R, String>,
    {
        match &self.planka_client {
            Some(client) => f(client),
            None => Err("Planka client not configured".to_string()),
        }
    }

    /// Sync single task to Planka
    pub async fn sync_task_to_planka(&self, task: &Task) -> Result<()> {
        if !self.is_enabled().await {
            return Ok(());
        }

        let client = self
            .planka_client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Planka not configured"))?;

        match (&task.planka_card_id, &task.planka_list_id) {
            // Task exists in Planka - update it
            (Some(card_id), Some(list_id)) => {
                // Move to correct list based on status
                client
                    .move_card(card_id, list_id, 0)
                    .await
                    .map_err(|e| anyhow::anyhow!(e))?;
                info!("[OK] Synced task update: {}", task.id);
            }
            // New task - create in Planka
            _ => {
                let list_name = task.status.to_planka_list();
                let card = client
                    .quick_create_task(&task.project, list_name, &task.title, &task.description)
                    .await
                    .map_err(|e| anyhow::anyhow!(e))?;

                info!("[OK] Created task in Planka: {}", card.id);
            }
        }

        Ok(())
    }

    /// Full sync - upload all tasks to Planka
    pub async fn full_sync_to_planka(&self) -> Result<SyncStats> {
        if !self.is_enabled().await {
            return Err(anyhow::anyhow!("Sync is disabled"));
        }

        let mut stats = SyncStats::default();

        // Get all projects
        let projects = self.task_service.store.list_projects().await?;

        for project in projects {
            // Get tasks for project
            let tasks = self.task_service.get_project_tasks(&project.name).await?;

            for task in tasks {
                match self.sync_task_to_planka(&task).await {
                    Ok(_) => stats.synced += 1,
                    Err(e) => {
                        stats.failed += 1;
                        warn!("[WARN] Failed to sync task {}: {}", task.id, e);
                    }
                }
            }
        }

        info!(
            "[OK] Full sync completed: {} synced, {} failed",
            stats.synced, stats.failed
        );
        Ok(stats)
    }

    /// Import tasks from Planka to Xavier
    pub async fn import_from_planka(&self, project_name: &str) -> Result<Vec<Task>> {
        let client = self
            .planka_client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Planka not configured"))?;

        // Get full board from Planka
        let (_project, _board, lists, cards) = client
            .get_full_board(project_name)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;

        let mut imported = Vec::new();

        for card in cards {
            let list = lists.iter().find(|l| l.id == card.list_id);
            let status = list
                .map(|l| match l.name.as_str() {
                    "Backlog" => TaskStatus::Backlog,
                    "In Progress" => TaskStatus::InProgress,
                    "Done" => TaskStatus::Done,
                    _ => TaskStatus::Backlog,
                })
                .unwrap_or(TaskStatus::Backlog);

            let mut task = Task::new(&card.name, project_name, "planka-import");
            task.description = card.description.unwrap_or_default();
            task.status = status;
            task.planka_card_id = Some(card.id);
            task.planka_list_id = Some(card.list_id);

            imported.push(task);
        }

        info!("[OK] Imported {} tasks from Planka", imported.len());
        Ok(imported)
    }

    /// Create project in both Xavier and Planka
    pub async fn create_project_full(&self, name: &str, description: &str) -> Result<Project> {
        // Create in Xavier first
        let project = self
            .task_service
            .get_or_create_project(name, description)
            .await?;

        // If Planka is enabled, create there too
        if self.is_enabled().await {
            if let Some(client) = &self.planka_client {
                // Create project in Planka
                let planka_project = client
                    .create_project(name, description)
                    .await
                    .map_err(|e| anyhow::anyhow!(e))?;

                // Create default board
                let board = client
                    .create_board(&planka_project.id, name)
                    .await
                    .map_err(|e| anyhow::anyhow!(e))?;

                // Create default lists
                for (list_name, pos) in [("Backlog", 0), ("In Progress", 1), ("Done", 2)] {
                    client
                        .create_list(&board.id, list_name, pos)
                        .await
                        .map_err(|e| anyhow::anyhow!(e))?;
                }

                info!("[OK] Created project in Planka: {}", name);
            }
        }

        Ok(project)
    }
}

/// Sync statistics
#[derive(Debug, Default)]
pub struct SyncStats {
    pub synced: usize,
    pub failed: usize,
    pub pending: usize,
}

/// Convenience function to create a full task system with Planka sync
pub fn create_task_system() -> (
    Arc<InMemoryTaskStore>,
    TaskService<InMemoryTaskStore>,
    PlankaSyncService<InMemoryTaskStore>,
) {
    let store = Arc::new(InMemoryTaskStore::new());
    let task_service = TaskService::new(store.clone());
    let sync_service = PlankaSyncService::new(store.clone(), None);

    (store, task_service, sync_service)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_task_creation() {
        let (_store, service, _sync) = create_task_system();

        let task = service
            .create_task("Test task", "Xavier", "test-user")
            .await
            .expect("test assertion");

        assert_eq!(task.title, "Test task");
        assert_eq!(task.project, "Xavier");
        assert_eq!(task.status, TaskStatus::Backlog);
    }

    #[tokio::test]
    async fn test_task_board() {
        let (_store, service, _sync) = create_task_system();

        // Create tasks in different statuses
        service
            .create_task("Task 1", "Xavier", "user")
            .await
            .expect("test assertion");

        let task2 = service
            .create_task("Task 2", "Xavier", "user")
            .await
            .expect("test assertion");
        service
            .move_task(&task2.id, TaskStatus::InProgress)
            .await
            .expect("test assertion");

        let board = service.get_task_board("Xavier").await.expect("test assertion");

        assert!(!board.get("Backlog").expect("test assertion").is_empty());
        assert!(!board.get("In Progress").expect("test assertion").is_empty());
    }
}
