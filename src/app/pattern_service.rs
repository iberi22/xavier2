use crate::domain::pattern::{PatternCategory, PatternVerification, VerifiedPattern};
use crate::memory::qmd_memory::MemoryDocument;
use crate::memory::store::{MemoryRecord, MemoryStore};
use crate::ports::inbound::PatternDiscoverPort;
use async_trait::async_trait;
use chrono::Utc;
use serde_json::json;
use std::sync::Arc;

/// Service for discovering, storing, and retrieving verified code patterns.
///
/// Uses MemoryStore internally, storing patterns with path `pattern:{id}`
/// under a reserved workspace (`__patterns__`).
pub struct PatternService {
    storage: Arc<dyn MemoryStore>,
    pattern_workspace: String,
}

impl PatternService {
    pub fn new(storage: Arc<dyn MemoryStore>) -> Self {
        Self {
            storage,
            pattern_workspace: "__patterns__".to_string(),
        }
    }

    fn pattern_path(id: &str) -> String {
        format!("pattern:{}", id)
    }
}

#[async_trait]
impl PatternDiscoverPort for PatternService {
    async fn discover(&self, pattern: VerifiedPattern) -> anyhow::Result<String> {
        let id = pattern.id.clone();
        let path = Self::pattern_path(&id);
        let doc = MemoryDocument {
            id: Some(id.clone()),
            path,
            content: serde_json::to_string_pretty(&pattern)?,
            metadata: json!({"kind": "pattern"}),
            content_vector: None,
            embedding: vec![],
        };
        let record = MemoryRecord::from_document(&self.pattern_workspace, &doc, true, None);
        self.storage.put(record).await?;
        Ok(id)
    }

    async fn query(
        &self,
        project: &str,
        category: Option<PatternCategory>,
        min_confidence: f32,
    ) -> anyhow::Result<Vec<VerifiedPattern>> {
        let all = self.storage.list(&self.pattern_workspace).await?;
        let mut results = Vec::new();

        for rec in all {
            if !rec.path.starts_with("pattern:") {
                continue;
            }
            if let Ok(p) = serde_json::from_str::<VerifiedPattern>(&rec.content) {
                if p.project != project {
                    continue;
                }
                if let Some(ref cat) = category {
                    if &p.category != cat {
                        continue;
                    }
                }
                if p.confidence < min_confidence {
                    continue;
                }
                results.push(p);
            }
        }

        results.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(results)
    }

    async fn verify(&self, id: &str, verified: bool) -> anyhow::Result<()> {
        let path = Self::pattern_path(id);
        let rec = self
            .storage
            .get(&self.pattern_workspace, &path)
            .await?
            .ok_or_else(|| anyhow::anyhow!("pattern {} not found", id))?;

        let mut pattern: VerifiedPattern = serde_json::from_str(&rec.content)?;
        pattern.verification = if verified {
            PatternVerification::Verified
        } else {
            PatternVerification::Rejected
        };
        pattern.updated_at = Utc::now();
        let updated = serde_json::to_string_pretty(&pattern)?;

        let doc = MemoryDocument {
            id: Some(id.to_string()),
            path,
            content: updated,
            metadata: json!({"kind": "pattern"}),
            content_vector: None,
            embedding: vec![],
        };
        let updated_rec = MemoryRecord::from_document(&self.pattern_workspace, &doc, true, None);
        self.storage.put(updated_rec).await?;
        Ok(())
    }

    async fn get(&self, id: &str) -> anyhow::Result<Option<VerifiedPattern>> {
        let path = Self::pattern_path(id);
        match self.storage.get(&self.pattern_workspace, &path).await? {
            Some(rec) => Ok(Some(serde_json::from_str(&rec.content)?)),
            None => Ok(None),
        }
    }

    async fn delete(&self, id: &str) -> anyhow::Result<Option<VerifiedPattern>> {
        let path = Self::pattern_path(id);
        let existing = match self
            .storage
            .get(&self.pattern_workspace, &path)
            .await?
        {
            Some(rec) => serde_json::from_str::<VerifiedPattern>(&rec.content).ok(),
            None => None,
        };
        self.storage
            .delete(&self.pattern_workspace, &path)
            .await?;
        Ok(existing)
    }

    async fn increment_usage(&self, id: &str) -> anyhow::Result<()> {
        let path = Self::pattern_path(id);
        let rec = self
            .storage
            .get(&self.pattern_workspace, &path)
            .await?
            .ok_or_else(|| anyhow::anyhow!("pattern {} not found", id))?;

        let mut pattern: VerifiedPattern = serde_json::from_str(&rec.content)?;
        pattern.usage_count += 1;
        pattern.updated_at = Utc::now();
        let updated = serde_json::to_string_pretty(&pattern)?;

        let doc = MemoryDocument {
            id: Some(id.to_string()),
            path,
            content: updated,
            metadata: json!({"kind": "pattern"}),
            content_vector: None,
            embedding: vec![],
        };
        let updated_rec = MemoryRecord::from_document(&self.pattern_workspace, &doc, true, None);
        self.storage.put(updated_rec).await?;
        Ok(())
    }
}
