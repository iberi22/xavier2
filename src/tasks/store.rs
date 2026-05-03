//! Task Store - Persistence layer for tasks
//! Can use different backends (SQLite, file, in-memory)

use crate::tasks::models::{Priority, Project, Task, TaskFilter, TaskStats, TaskStatus};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Trait for task storage backends
#[allow(async_fn_in_trait)]
pub trait TaskStore: Send + Sync {
    // Tasks
    async fn create_task(&self, task: &Task) -> Result<()>;
    async fn get_task(&self, id: &str) -> Result<Option<Task>>;
    async fn update_task(&self, task: &Task) -> Result<()>;
    async fn delete_task(&self, id: &str) -> Result<()>;
    async fn list_tasks(&self, filter: &TaskFilter) -> Result<Vec<Task>>;
    async fn get_task_stats(&self, project: Option<&str>) -> Result<TaskStats>;

    // Projects
    async fn create_project(&self, project: &Project) -> Result<()>;
    async fn get_project(&self, id: &str) -> Result<Option<Project>>;
    async fn get_project_by_name(&self, name: &str) -> Result<Option<Project>>;
    async fn update_project(&self, project: &Project) -> Result<()>;
    async fn delete_project(&self, id: &str) -> Result<()>;
    async fn list_projects(&self) -> Result<Vec<Project>>;
}

/// In-memory task store (for development/testing)
pub struct InMemoryTaskStore {
    tasks: RwLock<Vec<Task>>,
    projects: RwLock<Vec<Project>>,
}

impl InMemoryTaskStore {
    pub fn new() -> Self {
        Self {
            tasks: RwLock::new(Vec::new()),
            projects: RwLock::new(Vec::new()),
        }
    }
}

impl TaskStore for InMemoryTaskStore {
    async fn create_task(&self, task: &Task) -> Result<()> {
        let mut tasks = self.tasks.write().await;
        tasks.push(task.clone());
        Ok(())
    }

    async fn get_task(&self, id: &str) -> Result<Option<Task>> {
        let tasks = self.tasks.read().await;
        Ok(tasks.iter().find(|t| t.id == id).cloned())
    }

    async fn update_task(&self, task: &Task) -> Result<()> {
        let mut tasks = self.tasks.write().await;
        if let Some(pos) = tasks.iter().position(|t| t.id == task.id) {
            tasks[pos] = task.clone();
            Ok(())
        } else {
            Err(anyhow::anyhow!("Task not found"))
        }
    }

    async fn delete_task(&self, id: &str) -> Result<()> {
        let mut tasks = self.tasks.write().await;
        tasks.retain(|t| t.id != id);
        Ok(())
    }

    async fn list_tasks(&self, filter: &TaskFilter) -> Result<Vec<Task>> {
        let tasks = self.tasks.read().await;
        Ok(tasks
            .iter()
            .filter(|t| filter.matches(t))
            .cloned()
            .collect())
    }

    async fn get_task_stats(&self, project: Option<&str>) -> Result<TaskStats> {
        let tasks = self.tasks.read().await;
        let filtered: Vec<Task> = match project {
            Some(p) => tasks.iter().filter(|t| t.project == p).cloned().collect(),
            None => tasks.clone(),
        };
        Ok(TaskStats::from_tasks(&filtered))
    }

    // Projects
    async fn create_project(&self, project: &Project) -> Result<()> {
        let mut projects = self.projects.write().await;
        projects.push(project.clone());
        Ok(())
    }

    async fn get_project(&self, id: &str) -> Result<Option<Project>> {
        let projects = self.projects.read().await;
        Ok(projects.iter().find(|p| p.id == id).cloned())
    }

    async fn get_project_by_name(&self, name: &str) -> Result<Option<Project>> {
        let projects = self.projects.read().await;
        Ok(projects
            .iter()
            .find(|p| p.name.to_lowercase() == name.to_lowercase())
            .cloned())
    }

    async fn update_project(&self, project: &Project) -> Result<()> {
        let mut projects = self.projects.write().await;
        if let Some(pos) = projects.iter().position(|p| p.id == project.id) {
            projects[pos] = project.clone();
            Ok(())
        } else {
            Err(anyhow::anyhow!("Project not found"))
        }
    }

    async fn delete_project(&self, id: &str) -> Result<()> {
        let mut projects = self.projects.write().await;
        projects.retain(|p| p.id != id);
        Ok(())
    }

    async fn list_projects(&self) -> Result<Vec<Project>> {
        let projects = self.projects.read().await;
        Ok(projects.clone())
    }
}

impl Default for InMemoryTaskStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Task service - business logic layer
pub struct TaskService<S: TaskStore> {
    pub store: Arc<S>,
}

impl<S: TaskStore> TaskService<S> {
    pub fn new(store: Arc<S>) -> Self {
        Self { store }
    }

    /// Create a new task
    pub async fn create_task(&self, title: &str, project: &str, created_by: &str) -> Result<Task> {
        let task = Task::new(title, project, created_by);
        self.store.create_task(&task).await?;
        Ok(task)
    }

    /// Quick create task with description
    pub async fn create_task_with_details(
        &self,
        title: &str,
        description: &str,
        project: &str,
        priority: Priority,
        labels: Vec<String>,
        created_by: &str,
    ) -> Result<Task> {
        let mut task = Task::new(title, project, created_by);
        task.description = description.to_string();
        task.priority = priority;
        task.labels = labels;

        self.store.create_task(&task).await?;
        Ok(task)
    }

    /// Move task to different status
    pub async fn move_task(&self, task_id: &str, new_status: TaskStatus) -> Result<Task> {
        let mut task = self
            .store
            .get_task(task_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Task not found"))?;

        task.set_status(new_status);
        self.store.update_task(&task).await?;

        Ok(task)
    }

    /// Get tasks for a project
    pub async fn get_project_tasks(&self, project: &str) -> Result<Vec<Task>> {
        let filter = TaskFilter {
            project: Some(project.to_string()),
            ..Default::default()
        };
        self.store.list_tasks(&filter).await
    }

    /// Get task board view (grouped by status)
    pub async fn get_task_board(
        &self,
        project: &str,
    ) -> Result<std::collections::HashMap<String, Vec<Task>>> {
        let tasks = self.get_project_tasks(project).await?;

        let mut board = std::collections::HashMap::new();
        board.insert("Backlog".to_string(), Vec::new());
        board.insert("In Progress".to_string(), Vec::new());
        board.insert("Done".to_string(), Vec::new());

        for task in tasks {
            let key = task.status.to_planka_list().to_string();
            board.entry(key).or_insert_with(Vec::new).push(task);
        }

        Ok(board)
    }

    /// Assign task to user
    pub async fn assign_task(&self, task_id: &str, assignee: &str) -> Result<Task> {
        let mut task = self
            .store
            .get_task(task_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Task not found"))?;

        task.assignee = Some(assignee.to_string());
        task.updated_at = chrono::Utc::now();
        self.store.update_task(&task).await?;

        Ok(task)
    }

    /// Get or create project
    pub async fn get_or_create_project(&self, name: &str, description: &str) -> Result<Project> {
        if let Some(p) = self.store.get_project_by_name(name).await? {
            return Ok(p);
        }

        let project = Project::new(name, description);
        self.store.create_project(&project).await?;
        Ok(project)
    }
}
