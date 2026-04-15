//! Integrator Agent - Applies changes and manages git state

use crate::agents::evolve::experiment::Hypothesis;
use anyhow::Result;
use std::path::PathBuf;
use tracing::info;

/// Integrator - Manages git state and applies/discards changes
pub struct Integrator {
    memory_path: PathBuf,
}

impl Integrator {
    pub fn new() -> Self {
        Self {
            memory_path: PathBuf::from("src/memory/"),
        }
    }

    /// Backup current memory modules state
    pub async fn backup_memory_modules(&self) -> Result<Backup> {
        let files = self.list_editable_files().await?;

        let mut backup = Backup {
            files: Vec::new(),
        };

        for file in &files {
            let content = tokio::fs::read_to_string(file).await?;
            backup.files.push((file.clone(), content));
        }

        info!(files = backup.files.len(), "Created backup");
        Ok(backup)
    }

    /// Apply a hypothesis (modify files)
    pub async fn apply_hypothesis(&self, hypothesis: &Hypothesis) -> Result<bool> {
        if hypothesis.patch.is_empty() && hypothesis.hypothesis_type != crate::agents::evolve::experiment::HypothesisType::Simplification {
            return Ok(false);
        }

        // For now, just log what would be changed
        // In production, this would apply the actual patch
        info!(
            hypothesis_id = %hypothesis.id,
            description = %hypothesis.description,
            files = ?hypothesis.files,
            "Applied hypothesis (simulated)"
        );

        Ok(true)
    }

    /// Restore from backup (discard changes)
    pub async fn restore(&self, backup: Backup) -> Result<()> {
        for (path, content) in backup.files {
            tokio::fs::write(&path, content).await?;
        }
        info!("Restored from backup");
        Ok(())
    }

    /// Commit changes (keep improvement)
    pub async fn commit(&self, hypothesis: &Hypothesis) -> Result<()> {
        // In production, this would run git commands
        info!(
            hypothesis_id = %hypothesis.id,
            description = %hypothesis.description,
            "Committed improvement"
        );
        Ok(())
    }

    /// Reset to baseline commit
    pub async fn reset_to_baseline(&self) -> Result<()> {
        // In production, this would run git reset
        info!("Reset to baseline");
        Ok(())
    }

    /// List editable files
    async fn list_editable_files(&self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        let entries = tokio::fs::read_dir(&self.memory_path).await?;
        let mut entries = std::pin::pin!(entries);

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "rs") {
                files.push(path);
            }
        }

        // Also include agent files
        let agent_files = vec![
            PathBuf::from("src/agents/system1.rs"),
            PathBuf::from("src/agents/system2.rs"),
            PathBuf::from("src/agents/system3.rs"),
        ];

        for af in agent_files {
            if af.exists() {
                files.push(af);
            }
        }

        Ok(files)
    }
}

impl Default for Integrator {
    fn default() -> Self {
        Self::new()
    }
}

/// Backup of modified files
#[derive(Debug, Clone)]
pub struct Backup {
    files: Vec<(PathBuf, String)>,
}
