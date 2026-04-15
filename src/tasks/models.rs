//! Task Management - Core abstraction layer for tasks
//! Independent of any backend - Planka is just a sync target

use std::convert::Infallible;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

/// Task priority
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Low,
    #[default]
    Medium,
    High,
    Urgent,
}

/// Task status
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    #[default]
    Backlog,
    InProgress,
    Done,
}

impl std::str::FromStr for TaskStatus {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "backlog" => Ok(TaskStatus::Backlog),
            "in_progress" | "inprogress" | "progress" | "working" => Ok(TaskStatus::InProgress),
            "done" | "completed" | "complete" => Ok(TaskStatus::Done),
            _ => Ok(TaskStatus::Backlog),
        }
    }
}

impl TaskStatus {
    pub fn to_planka_list(&self) -> &'static str {
        match self {
            TaskStatus::Backlog => "Backlog",
            TaskStatus::InProgress => "In Progress",
            TaskStatus::Done => "Done",
        }
    }
}

/// Core Task structure - backend agnostic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique identifier
    pub id: String,

    /// Task title
    pub title: String,

    /// Detailed description
    pub description: String,

    /// Project this task belongs to
    pub project: String,

    /// Current status
    pub status: TaskStatus,

    /// Priority level
    pub priority: Priority,

    /// Tags/labels
    pub labels: Vec<String>,

    /// Assignee (user ID or name)
    pub assignee: Option<String>,

    /// Who created this task
    pub created_by: String,

    /// Timestamps
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,

    /// External sync (Planka)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub planka_card_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub planka_list_id: Option<String>,
}

impl Task {
    /// Create a new task
    pub fn new(title: &str, project: &str, created_by: &str) -> Self {
        let now = Utc::now();
        Self {
            id: Ulid::new().to_string(),
            title: title.to_string(),
            description: String::new(),
            project: project.to_string(),
            status: TaskStatus::Backlog,
            priority: Priority::Medium,
            labels: Vec::new(),
            assignee: None,
            created_by: created_by.to_string(),
            created_at: now,
            updated_at: now,
            completed_at: None,
            planka_card_id: None,
            planka_list_id: None,
        }
    }

    /// Update status and handle completion
    pub fn set_status(&mut self, status: TaskStatus) {
        self.status = status;
        self.updated_at = Utc::now();

        if status == TaskStatus::Done && self.completed_at.is_none() {
            self.completed_at = Some(Utc::now());
        } else if status != TaskStatus::Done {
            self.completed_at = None;
        }
    }

    /// Check if task matches search query
    pub fn matches(&self, query: &str) -> bool {
        let q = query.to_lowercase();
        self.title.to_lowercase().contains(&q)
            || self.description.to_lowercase().contains(&q)
            || self.project.to_lowercase().contains(&q)
            || self.labels.iter().any(|l| l.to_lowercase().contains(&q))
    }
}

/// Project configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: String,
    pub color: String,
    pub default_labels: Vec<String>,

    /// Planka sync
    #[serde(skip_serializing_if = "Option::is_none")]
    pub planka_project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub planka_board_id: Option<String>,
}

impl Project {
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            id: Ulid::new().to_string(),
            name: name.to_string(),
            description: description.to_string(),
            color: "#3b82f6".to_string(),
            default_labels: Vec::new(),
            planka_project_id: None,
            planka_board_id: None,
        }
    }
}

/// Task filter for queries
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskFilter {
    pub project: Option<String>,
    pub status: Option<TaskStatus>,
    pub priority: Option<Priority>,
    pub assignee: Option<String>,
    pub labels: Option<Vec<String>>,
    pub search: Option<String>,
}

impl TaskFilter {
    pub fn matches(&self, task: &Task) -> bool {
        if let Some(ref p) = self.project {
            if &task.project != p {
                return false;
            }
        }
        if let Some(s) = self.status {
            if task.status != s {
                return false;
            }
        }
        if let Some(p) = self.priority {
            if task.priority != p {
                return false;
            }
        }
        if let Some(ref a) = self.assignee {
            if task.assignee.as_ref() != Some(a) {
                return false;
            }
        }
        if let Some(ref labels) = self.labels {
            if !labels.iter().any(|l| task.labels.contains(l)) {
                return false;
            }
        }
        if let Some(ref q) = self.search {
            if !task.matches(q) {
                return false;
            }
        }
        true
    }
}

/// Task statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskStats {
    pub total: usize,
    pub backlog: usize,
    pub in_progress: usize,
    pub done: usize,
    pub by_priority: std::collections::HashMap<String, usize>,
}

impl TaskStats {
    pub fn from_tasks(tasks: &[Task]) -> Self {
        let mut stats = TaskStats::default();

        for task in tasks {
            stats.total += 1;
            match task.status {
                TaskStatus::Backlog => stats.backlog += 1,
                TaskStatus::InProgress => stats.in_progress += 1,
                TaskStatus::Done => stats.done += 1,
            }

            let p = format!("{:?}", task.priority).to_lowercase();
            *stats.by_priority.entry(p).or_insert(0) += 1;
        }

        stats
    }
}
