use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::process::Command;
use crate::memory::qmd_memory::QmdMemory;
use code_graph::db::CodeGraphDB;

#[derive(Debug, Serialize, Deserialize)]
pub struct CommitInfo {
    pub hash: String,
    pub author: String,
    pub date: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryInsight {
    pub path: String,
    pub content: String,
    pub kind: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CodeChange {
    pub file_path: String,
    pub symbol_name: String,
    pub symbol_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HarvestOutput {
    pub commits: Vec<CommitInfo>,
    pub insights: Vec<MemoryInsight>,
    pub code_changes: Vec<CodeChange>,
}

pub struct Harvester<'a> {
    memory: &'a QmdMemory,
    code_db: &'a CodeGraphDB,
    workspace_dir: &'a Path,
}

impl<'a> Harvester<'a> {
    pub fn new(memory: &'a QmdMemory, code_db: &'a CodeGraphDB, workspace_dir: &'a Path) -> Self {
        Self {
            memory,
            code_db,
            workspace_dir,
        }
    }

    pub async fn harvest(&self, since_days: u32) -> Result<HarvestOutput> {
        let commits = self.harvest_git_log(since_days).await?;
        let insights = self.harvest_memory().await?;
        let code_changes = self.harvest_code_changes(since_days).await?;

        Ok(HarvestOutput {
            commits,
            insights,
            code_changes,
        })
    }

    async fn harvest_git_log(&self, since_days: u32) -> Result<Vec<CommitInfo>> {
        let since = format!("{} days ago", since_days);
        let output = Command::new("git")
            .arg("-C")
            .arg(self.workspace_dir)
            .arg("log")
            .arg(format!("--since={}", since))
            .arg("--pretty=format:%H|%an|%ad|%s")
            .arg("--date=short")
            .output()
            .await?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let commits = stdout
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() == 4 {
                    Some(CommitInfo {
                        hash: parts[0].to_string(),
                        author: parts[1].to_string(),
                        date: parts[2].to_string(),
                        message: parts[3].to_string(),
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(commits)
    }

    async fn harvest_memory(&self) -> Result<Vec<MemoryInsight>> {
        let mut insights = Vec::new();

        // Search for decisions, bugs, and sessions
        let categories = ["decisions/", "bugs/", "sessions/"];
        for category in categories {
            let docs = self.memory.search(category, 50).await?;
            for doc in docs {
                if doc.path.starts_with(category) {
                    insights.push(MemoryInsight {
                        path: doc.path.clone(),
                        content: doc.content.clone(),
                        kind: category.trim_end_matches('/').to_string(),
                    });
                }
            }
        }

        Ok(insights)
    }

    async fn harvest_code_changes(&self, since_days: u32) -> Result<Vec<CodeChange>> {
        let since = format!("{} days ago", since_days);
        // Get names of files changed in commits since X days ago
        let output = Command::new("git")
            .arg("-C")
            .arg(self.workspace_dir)
            .arg("log")
            .arg(format!("--since={}", since))
            .arg("--name-only")
            .arg("--pretty=format:")
            .output()
            .await?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut changed_files = std::collections::HashSet::new();
        for line in stdout.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                changed_files.insert(trimmed.to_string());
            }
        }

        let mut changes = Vec::new();
        // For each changed file, try to find associated symbols in CodeGraphDB
        for file_path in changed_files {
            // We search for symbols where file_path matches.
            // find_symbols currently only searches by name.
            // We might need a better way to filter by file_path in CodeGraphDB if available.
            // Looking at code-graph/src/db/mod.rs, find_symbols uses name LIKE %query%.

            // Temporary workaround: pull many symbols and filter.
            // Optimization: we could add find_by_file to CodeGraphDB.
            let query_result = self.code_db.find_symbols("", 1000)?;
            for s in query_result.symbols {
                if s.file_path == file_path {
                    changes.push(CodeChange {
                        file_path: s.file_path.clone(),
                        symbol_name: s.name.clone(),
                        symbol_type: format!("{:?}", s.kind),
                    });
                }
            }
        }

        // De-duplicate changes
        changes.sort_by(|a, b| a.file_path.cmp(&b.file_path).then(a.symbol_name.cmp(&b.symbol_name)));
        changes.dedup_by(|a, b| a.file_path == b.file_path && a.symbol_name == b.symbol_name);

        Ok(changes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::qmd_memory::QmdMemory;
    use tokio::sync::RwLock as AsyncRwLock;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_harvest_git_log_basic() {
        // This test requires a git repo, which should be present in the environment
        let memory = QmdMemory::new(Arc::new(AsyncRwLock::new(Vec::new())));
        let code_db = CodeGraphDB::in_memory().unwrap();
        let harvester = Harvester::new(&memory, &code_db, Path::new("."));

        let result = harvester.harvest_git_log(1).await;
        assert!(result.is_ok());
    }
}
