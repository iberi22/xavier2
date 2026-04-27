use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize)]
pub struct HookResult<T> {
    pub status: String,
    pub data: T,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionSnapshot {
    pub session_id: String,
    pub timestamp: DateTime<Utc>,
    pub files_edited: Vec<FileEdit>,
    pub git_operations: Vec<GitOp>,
    pub tasks: Vec<TaskState>,
    pub decisions: Vec<Decision>,
    pub context_summary: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileEdit {
    pub path: String,
    pub change_summary: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitOp {
    pub command: String,
    pub branch: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskState {
    pub task_id: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Decision {
    pub decision: String,
    pub rationale: String,
}

#[async_trait]
pub trait Hooks: Send + Sync {
    /// Antes de ejecutar una tool/operación
    async fn pre_tool_use(&self, tool: &str, input: &Value) -> anyhow::Result<HookResult<()>>;

    /// Después de ejecutar una tool/operación
    async fn post_tool_use(&self, tool: &str, output: &Value) -> anyhow::Result<HookResult<Value>>;

    /// Antes de comprimir el contexto (session continuity)
    async fn pre_compact(&self, session_id: &str) -> anyhow::Result<HookResult<SessionSnapshot>>;

    /// Al iniciar una sesión nueva
    async fn session_start(&self, session_id: &str) -> anyhow::Result<HookResult<Option<SessionSnapshot>>>;
}
