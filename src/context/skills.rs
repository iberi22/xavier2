use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub content: String,
}

pub struct SkillLoader {
    base_path: PathBuf,
}

impl SkillLoader {
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
        }
    }

    pub async fn load_all(&self) -> Result<Vec<Skill>> {
        let mut skills = Vec::new();

        if !self.base_path.exists() {
            return Ok(skills);
        }

        for entry in WalkDir::new(&self.base_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("md"))
        {
            let content = fs::read_to_string(entry.path()).await?;
            let name = entry
                .path()
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();

            if self.validate_skill(&content) {
                skills.push(Skill { name, content });
            }
        }

        Ok(skills)
    }

    pub fn validate_skill(&self, content: &str) -> bool {
        // Simple validation: check for # Purpose or ## Purpose
        content.contains("# Purpose") || content.contains("## Purpose")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[tokio::test]
    async fn loads_valid_skills() {
        let dir = tempdir().unwrap();
        let skill_path = dir.path().join("test-skill.md");
        let mut file = File::create(skill_path).unwrap();
        writeln!(file, "# Purpose\nTest skill content").unwrap();

        let loader = SkillLoader::new(dir.path());
        let skills = loader.load_all().await.unwrap();

        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "test-skill");
        assert!(skills[0].content.contains("Test skill content"));
    }

    #[tokio::test]
    async fn ignores_invalid_skills() {
        let dir = tempdir().unwrap();
        let skill_path = dir.path().join("invalid.md");
        let mut file = File::create(skill_path).unwrap();
        writeln!(file, "No purpose here").unwrap();

        let loader = SkillLoader::new(dir.path());
        let skills = loader.load_all().await.unwrap();

        assert_eq!(skills.len(), 0);
    }
}
