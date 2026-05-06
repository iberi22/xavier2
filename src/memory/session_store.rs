use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::{fs, sync::RwLock};
use ulid::Ulid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelMessage {
    pub id: String,
    pub role: String,
    pub plain_text: String,
    pub openui_lang: Option<String>,
    pub created_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelThread {
    pub id: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_preview: String,
    pub messages: Vec<PanelMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadSummary {
    pub id: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_preview: String,
    pub message_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadDetail {
    pub thread: ThreadSummary,
    pub messages: Vec<PanelMessage>,
}

#[derive(Clone)]
pub struct SessionStore {
    root: PathBuf,
    threads: Arc<RwLock<HashMap<String, PanelThread>>>,
}

impl SessionStore {
    pub async fn new(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        fs::create_dir_all(&root)
            .await
            .with_context(|| format!("failed to create session store root {}", root.display()))?;

        let mut threads = HashMap::new();
        let mut entries = fs::read_dir(&root)
            .await
            .with_context(|| format!("failed to read session store root {}", root.display()))?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if !is_thread_file(&path) {
                continue;
            }

            let payload = fs::read_to_string(&path)
                .await
                .with_context(|| format!("failed to read thread file {}", path.display()))?;

            let thread: PanelThread = serde_json::from_str(&payload)
                .with_context(|| format!("failed to parse thread file {}", path.display()))?;
            threads.insert(thread.id.clone(), thread);
        }

        Ok(Self {
            root,
            threads: Arc::new(RwLock::new(threads)),
        })
    }

    pub async fn list_threads(&self) -> Vec<ThreadSummary> {
        let threads = self.threads.read().await;
        let mut items: Vec<_> = threads.values().map(ThreadSummary::from).collect();
        items.sort_by_key(|item| std::cmp::Reverse(item.updated_at));
        items
    }

    pub async fn get_thread(&self, id: &str) -> Option<PanelThread> {
        self.threads.read().await.get(id).cloned()
    }

    pub async fn create_thread(&self, title_hint: &str) -> Result<PanelThread> {
        let now = Utc::now();
        let thread = PanelThread {
            id: Ulid::new().to_string(),
            title: summarize_title(title_hint),
            created_at: now,
            updated_at: now,
            last_preview: String::new(),
            messages: Vec::new(),
        };

        self.insert_thread(thread.clone()).await?;
        Ok(thread)
    }

    pub async fn delete_thread(&self, id: &str) -> Result<bool> {
        let removed = self.threads.write().await.remove(id);
        if removed.is_some() {
            let path = self.thread_path(id);
            if fs::try_exists(&path).await.unwrap_or(false) {
                fs::remove_file(&path)
                    .await
                    .with_context(|| format!("failed to delete thread file {}", path.display()))?;
            }
            return Ok(true);
        }

        Ok(false)
    }

    pub async fn append_message(
        &self,
        thread_id: &str,
        message: PanelMessage,
    ) -> Result<PanelThread> {
        let updated = {
            let mut threads = self.threads.write().await;
            let thread = threads
                .get_mut(thread_id)
                .with_context(|| format!("thread {thread_id} not found"))?;

            thread.updated_at = message.created_at;
            if !message.plain_text.trim().is_empty() {
                thread.last_preview = preview_text(&message.plain_text);
            }
            if thread.messages.is_empty() && thread.title == "New Thread" && message.role == "user"
            {
                thread.title = summarize_title(&message.plain_text);
            }
            thread.messages.push(message);
            thread.clone()
        };

        self.persist_thread(&updated).await?;
        Ok(updated)
    }

    pub async fn insert_thread(&self, thread: PanelThread) -> Result<()> {
        self.threads
            .write()
            .await
            .insert(thread.id.clone(), thread.clone());
        self.persist_thread(&thread).await
    }

    pub fn detail_from_thread(thread: PanelThread) -> ThreadDetail {
        ThreadDetail {
            thread: ThreadSummary::from(&thread),
            messages: thread.messages,
        }
    }

    async fn persist_thread(&self, thread: &PanelThread) -> Result<()> {
        let payload = serde_json::to_vec_pretty(thread).context("failed to serialize thread")?;
        fs::write(self.thread_path(&thread.id), payload)
            .await
            .with_context(|| format!("failed to persist thread {}", thread.id))?;
        Ok(())
    }

    fn thread_path(&self, id: &str) -> PathBuf {
        self.root.join(format!("{id}.json"))
    }
}

impl From<&PanelThread> for ThreadSummary {
    fn from(thread: &PanelThread) -> Self {
        Self {
            id: thread.id.clone(),
            title: thread.title.clone(),
            created_at: thread.created_at,
            updated_at: thread.updated_at,
            last_preview: thread.last_preview.clone(),
            message_count: thread.messages.len(),
        }
    }
}

fn is_thread_file(path: &Path) -> bool {
    path.extension().and_then(|value| value.to_str()) == Some("json")
}

fn preview_text(value: &str) -> String {
    value.trim().chars().take(120).collect()
}

fn summarize_title(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return "New Thread".to_string();
    }

    trimmed
        .lines()
        .next()
        .unwrap_or(trimmed)
        .chars()
        .take(48)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store_root() -> PathBuf {
        std::env::temp_dir().join(format!("xavier2-session-store-{}", Ulid::new()))
    }

    #[tokio::test]
    async fn creates_and_loads_threads() {
        let root = temp_store_root();
        let store = SessionStore::new(&root).await.unwrap();
        let thread = store.create_thread("Thread title").await.unwrap();

        let message = PanelMessage {
            id: Ulid::new().to_string(),
            role: "user".to_string(),
            plain_text: "Hello from the panel".to_string(),
            openui_lang: None,
            created_at: Utc::now(),
            metadata: serde_json::json!({}),
        };

        store.append_message(&thread.id, message).await.unwrap();

        let reloaded = SessionStore::new(&root).await.unwrap();
        let loaded = reloaded.get_thread(&thread.id).await.unwrap();
        assert_eq!(loaded.messages.len(), 1);
        assert_eq!(loaded.last_preview, "Hello from the panel");
    }

    #[tokio::test]
    async fn deletes_threads() {
        let root = temp_store_root();
        let store = SessionStore::new(&root).await.unwrap();
        let thread = store.create_thread("Delete me").await.unwrap();

        assert!(store.delete_thread(&thread.id).await.unwrap());
        assert!(store.get_thread(&thread.id).await.is_none());
    }

    #[tokio::test]
    async fn first_user_message_renames_new_threads() {
        let root = temp_store_root();
        let store = SessionStore::new(&root).await.unwrap();
        let thread = store.create_thread("New Thread").await.unwrap();

        let message = PanelMessage {
            id: Ulid::new().to_string(),
            role: "user".to_string(),
            plain_text: "Explain xavier2 memory and show a structured UI.".to_string(),
            openui_lang: None,
            created_at: Utc::now(),
            metadata: serde_json::json!({}),
        };

        let updated = store.append_message(&thread.id, message).await.unwrap();

        assert_eq!(updated.messages.len(), 1);
        assert_ne!(updated.title, "New Thread");
        assert_eq!(
            updated.title,
            "Explain xavier2 memory and show a structured UI."
        );
    }
}
