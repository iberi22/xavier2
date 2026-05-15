use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use git2::{Repository, Sort};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use crate::memory::qmd_memory::QmdMemory;
use code_graph::db::CodeGraphDB;
use code_graph::types::Symbol;

#[derive(Debug, Serialize, Deserialize)]
pub struct HarvestOutput {
    pub date: String,
    pub commits: Vec<CommitInfo>,
    pub decisions: Vec<MemoryEntry>,
    pub bugs: Vec<MemoryEntry>,
    pub sessions: Vec<SessionInfo>,
    pub code_changes: Vec<CodeChangeInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommitInfo {
    pub hash: String,
    pub message: String,
    pub files: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub path: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub duration_ms: u64,
    pub tokens: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CodeChangeInfo {
    pub file: String,
    pub symbols: Vec<String>,
}

pub struct Harvester {
    workspace_path: PathBuf,
    memory: Arc<QmdMemory>,
    code_db: Arc<CodeGraphDB>,
}

impl Harvester {
    pub fn new(workspace_path: PathBuf, memory: Arc<QmdMemory>, code_db: Arc<CodeGraphDB>) -> Self {
        Self {
            workspace_path,
            memory,
            code_db,
        }
    }

    pub async fn run(&self, since: DateTime<Utc>) -> Result<PathBuf> {
        let date = Utc::now().format("%Y-%m-%d").to_string();

        let commits = self.harvest_commits(since)?;
        let decisions = self.harvest_memories("decisions/*").await?;
        let bugs = self.harvest_memories("bugs/*").await?;
        let sessions = self.harvest_sessions().await?;

        let modified_files: HashSet<String> =
            commits.iter().flat_map(|c| c.files.clone()).collect();

        let code_changes = self.harvest_code_changes(modified_files)?;

        let output = HarvestOutput {
            date: date.clone(),
            commits,
            decisions,
            bugs,
            sessions,
            code_changes,
        };

        let chronicle_dir = self.workspace_path.join(".chronicle");
        if !chronicle_dir.exists() {
            std::fs::create_dir_all(&chronicle_dir)?;
        }

        let file_path = chronicle_dir.join(format!("harvest-{}.json", date));
        let json = serde_json::to_string_pretty(&output)?;
        std::fs::write(&file_path, json)?;

        Ok(file_path)
    }

    fn harvest_commits(&self, since: DateTime<Utc>) -> Result<Vec<CommitInfo>> {
        let repo = Repository::open(&self.workspace_path)
            .map_err(|e| anyhow!("Failed to open repo at {:?}: {}", self.workspace_path, e))?;

        let mut revwalk = repo.revwalk()?;
        revwalk.set_sorting(Sort::TIME | Sort::REVERSE)?;
        revwalk.push_head()?;

        let mut commit_infos = Vec::new();

        for id in revwalk {
            let id = id?;
            let commit = repo.find_commit(id)?;
            let commit_time = DateTime::<Utc>::from_timestamp(commit.time().seconds(), 0)
                .ok_or_else(|| anyhow!("Invalid commit timestamp"))?;

            if commit_time >= since {
                let mut files = Vec::new();

                // Get diff to find modified files
                if let Ok(parent) = commit.parent(0) {
                    let diff =
                        repo.diff_tree_to_tree(Some(&parent.tree()?), Some(&commit.tree()?), None)?;
                    diff.foreach(
                        &mut |delta, _| {
                            if let Some(new_file) = delta.new_file().path() {
                                if let Some(path_str) = new_file.to_str() {
                                    files.push(path_str.to_string());
                                }
                            }
                            true
                        },
                        None,
                        None,
                        None,
                    )?;
                } else {
                    // Initial commit
                    let tree = commit.tree()?;
                    tree.walk(git2::TreeWalkMode::PreOrder, |root, entry| {
                        if let Some(name) = entry.name() {
                            files.push(format!("{}{}", root, name));
                        }
                        git2::TreeWalkResult::Ok
                    })?;
                }

                commit_infos.push(CommitInfo {
                    hash: id.to_string(),
                    message: commit.message().unwrap_or_default().trim().to_string(),
                    files,
                });
            }
        }

        Ok(commit_infos)
    }

    async fn harvest_memories(&self, pattern: &str) -> Result<Vec<MemoryEntry>> {
        let docs = self.memory.all_documents().await;
        let mut entries = Vec::new();

        // Simple glob-to-regex or prefix match for patterns like "decisions/*"
        let prefix = pattern.trim_end_matches('*');

        for doc in docs {
            if doc.path.starts_with(prefix) {
                entries.push(MemoryEntry {
                    path: doc.path.clone(),
                    content: doc.content.clone(),
                    status: doc
                        .metadata
                        .get("status")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                });
            }
        }

        Ok(entries)
    }

    async fn harvest_sessions(&self) -> Result<Vec<SessionInfo>> {
        let docs = self.memory.all_documents().await;
        let mut sessions = Vec::new();

        for doc in docs {
            if doc.path.starts_with("sessions/") {
                // Heuristic: only pick summaries or direct session docs, avoiding every single turn
                if doc.path.contains("/summary") || !doc.path.contains("/turns/") {
                    let duration_ms = doc
                        .metadata
                        .get("duration_ms")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    let tokens = doc
                        .metadata
                        .get("tokens")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as usize;

                    sessions.push(SessionInfo {
                        id: doc.path.clone(),
                        duration_ms,
                        tokens,
                    });
                }
            }
        }

        Ok(sessions)
    }

    fn harvest_code_changes(&self, modified_files: HashSet<String>) -> Result<Vec<CodeChangeInfo>> {
        let mut code_changes = Vec::new();

        for file in modified_files {
            // Find symbols in this file using CodeGraphDB
            let symbols = self.find_symbols_in_file(&file)?;
            if !symbols.is_empty() {
                code_changes.push(CodeChangeInfo {
                    file,
                    symbols: symbols.into_iter().map(|s| s.name).collect(),
                });
            }
        }

        Ok(code_changes)
    }

    fn find_symbols_in_file(&self, file_path: &str) -> Result<Vec<Symbol>> {
        self.code_db
            .find_by_file(file_path)
            .map_err(|e| anyhow!("DB error: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::qmd_memory::MemoryDocument;
    use tokio::sync::RwLock as AsyncRwLock;

    #[tokio::test]
    async fn test_harvest_output_serialization() {
        let output = HarvestOutput {
            date: "2026-05-07".to_string(),
            commits: vec![CommitInfo {
                hash: "abc".to_string(),
                message: "feat: something".to_string(),
                files: vec!["src/lib.rs".to_string()],
            }],
            decisions: vec![MemoryEntry {
                path: "decisions/001-arch.md".to_string(),
                content: "Use Rust".to_string(),
                status: Some("accepted".to_string()),
            }],
            bugs: vec![],
            sessions: vec![],
            code_changes: vec![],
        };

        let json = serde_json::to_string(&output).expect("test assertion");
        assert!(json.contains("2026-05-07"));
        assert!(json.contains("decisions/001-arch.md"));
    }

    #[tokio::test]
    async fn test_harvest_memories() {
        let memory = Arc::new(QmdMemory::new(Arc::new(AsyncRwLock::new(vec![
            MemoryDocument {
                id: Some("1".into()),
                path: "decisions/use-rust.md".into(),
                content: "Content".into(),
                metadata: serde_json::json!({"status": "accepted"}),
                content_vector: None,
                embedding: vec![],
                ..Default::default()
            },
            MemoryDocument {
                id: Some("2".into()),
                path: "bugs/fix-ui.md".into(),
                content: "Content".into(),
                metadata: serde_json::json!({"status": "resolved"}),
                content_vector: None,
                embedding: vec![],
                ..Default::default()
            },
            MemoryDocument {
                id: Some("3".into()),
                path: "other/note.md".into(),
                content: "Content".into(),
                metadata: serde_json::json!({}),
                content_vector: None,
                embedding: vec![],
                ..Default::default()
            },
        ]))));

        let code_db = Arc::new(CodeGraphDB::in_memory().expect("test assertion"));
        let harvester = Harvester::new(PathBuf::from("."), memory, code_db);

        let decisions = harvester
            .harvest_memories("decisions/*")
            .await
            .expect("test assertion");
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0].path, "decisions/use-rust.md");
        assert_eq!(decisions[0].status, Some("accepted".to_string()));

        let bugs = harvester
            .harvest_memories("bugs/*")
            .await
            .expect("test assertion");
        assert_eq!(bugs.len(), 1);
        assert_eq!(bugs[0].path, "bugs/fix-ui.md");
    }
}
