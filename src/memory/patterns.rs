//! Pattern Protocol - Storage for verified patterns discovered by agents
//!
//! Patterns are code/text patterns discovered by agents (Codex, subagents) that have been
//! verified and can be reused across the codebase.

use chrono::Utc;
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// PatternStore is wired into AppState when HTTP endpoints are added

/// Pattern category types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PatternCategory {
    Naming,
    Structure,
    Workflow,
    Convention,
    Architecture,
}

impl PatternCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            PatternCategory::Naming => "naming",
            PatternCategory::Structure => "structure",
            PatternCategory::Workflow => "workflow",
            PatternCategory::Convention => "convention",
            PatternCategory::Architecture => "architecture",
        }
    }

    pub fn from_category(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "naming" => Some(PatternCategory::Naming),
            "structure" => Some(PatternCategory::Structure),
            "workflow" => Some(PatternCategory::Workflow),
            "convention" => Some(PatternCategory::Convention),
            "architecture" => Some(PatternCategory::Architecture),
            _ => None,
        }
    }
}

/// Verification status for patterns
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PatternVerification {
    Pending,
    Verified,
    Rejected,
    AutoVerified,
}

impl PatternVerification {
    pub fn as_str(&self) -> &'static str {
        match self {
            PatternVerification::Pending => "pending",
            PatternVerification::Verified => "verified",
            PatternVerification::Rejected => "rejected",
            PatternVerification::AutoVerified => "auto_verified",
        }
    }

    pub fn from_verification(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "verified" => PatternVerification::Verified,
            "rejected" => PatternVerification::Rejected,
            "auto_verified" => PatternVerification::AutoVerified,
            _ => PatternVerification::Pending,
        }
    }
}

/// A verified pattern record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiedPattern {
    pub id: String,
    pub category: String,
    pub pattern: String,
    pub project: String,
    pub discovered_by: String,
    pub confidence: f32,
    pub source_file: String,
    pub source_occurrences: usize,
    pub source_snippet: String,
    pub verification: String,
    pub usage_count: usize,
    pub created_at: String,
    pub updated_at: String,
}

/// Request to discover/register a new pattern
#[derive(Debug, Deserialize)]
pub struct PatternDiscoverRequest {
    pub category: String,
    pub pattern: String,
    pub project: String,
    pub discovered_by: String,
    pub confidence: f32,
    pub source_file: String,
    pub source_occurrences: usize,
    pub source_snippet: String,
}

/// Query parameters for pattern search
#[derive(Debug, Deserialize)]
pub struct PatternQueryParams {
    pub project: Option<String>,
    pub category: Option<String>,
    pub min_confidence: Option<f32>,
    pub verification: Option<String>,
    pub limit: Option<usize>,
}

/// Response for pattern queries
#[derive(Debug, Serialize)]
pub struct PatternQueryResponse {
    pub patterns: Vec<VerifiedPattern>,
    pub total: usize,
}

/// Pattern store for managing verified patterns
pub struct PatternStore {
    conn: Arc<Mutex<Connection>>,
}

impl PatternStore {
    /// Create a new PatternStore
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Initialize the patterns table
    pub fn init_schema(conn: &Connection) -> rusqlite::Result<()> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS patterns (
                id TEXT PRIMARY KEY,
                category TEXT NOT NULL,
                pattern TEXT NOT NULL,
                project TEXT NOT NULL,
                discovered_by TEXT NOT NULL,
                confidence REAL NOT NULL DEFAULT 0.5,
                source_file TEXT DEFAULT '',
                source_occurrences INTEGER DEFAULT 0,
                source_snippet TEXT DEFAULT '',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                usage_count INTEGER DEFAULT 0,
                verification TEXT DEFAULT 'pending'
            );

            CREATE INDEX IF NOT EXISTS idx_patterns_project ON patterns(project);
            CREATE INDEX IF NOT EXISTS idx_patterns_category ON patterns(category);
            CREATE INDEX IF NOT EXISTS idx_patterns_confidence ON patterns(confidence);
            "#,
        )?;
        Ok(())
    }

    /// Discover/register a new pattern
    pub fn discover(&self, req: &PatternDiscoverRequest) -> rusqlite::Result<VerifiedPattern> {
        let id = ulid::Ulid::new().to_string();
        let now = Utc::now().to_rfc3339();

        let conn = self.conn.lock();
        conn.execute(
            r#"INSERT INTO patterns
               (id, category, pattern, project, discovered_by, confidence,
                source_file, source_occurrences, source_snippet, created_at, updated_at,
                usage_count, verification)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0, 'pending')"#,
            params![
                &id,
                &req.category,
                &req.pattern,
                &req.project,
                &req.discovered_by,
                req.confidence,
                &req.source_file,
                req.source_occurrences,
                &req.source_snippet,
                &now,
                &now,
            ],
        )?;

        Ok(VerifiedPattern {
            id,
            category: req.category.clone(),
            pattern: req.pattern.clone(),
            project: req.project.clone(),
            discovered_by: req.discovered_by.clone(),
            confidence: req.confidence,
            source_file: req.source_file.clone(),
            source_occurrences: req.source_occurrences,
            source_snippet: req.source_snippet.clone(),
            verification: "pending".to_string(),
            usage_count: 0,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    /// Query patterns by filters
    pub fn query(&self, params: &PatternQueryParams) -> rusqlite::Result<Vec<VerifiedPattern>> {
        let conn = self.conn.lock();
        let limit = params.limit.unwrap_or(50);

        let mut sql = String::from("SELECT * FROM patterns WHERE 1=1");
        let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ref project) = params.project {
            sql.push_str(" AND project = ?");
            args.push(Box::new(project.clone()));
        }

        if let Some(ref category) = params.category {
            sql.push_str(" AND category = ?");
            args.push(Box::new(category.clone()));
        }

        if let Some(min_conf) = params.min_confidence {
            sql.push_str(" AND confidence >= ?");
            args.push(Box::new(min_conf));
        }

        if let Some(ref verification) = params.verification {
            sql.push_str(" AND verification = ?");
            args.push(Box::new(verification.clone()));
        }

        sql.push_str(" ORDER BY confidence DESC, usage_count DESC LIMIT ?");
        args.push(Box::new(limit));

        let mut stmt = conn.prepare(&sql)?;
        let args_refs: Vec<&dyn rusqlite::ToSql> = args.iter().map(|b| b.as_ref()).collect();
        let mut rows = stmt.query(args_refs.as_slice())?;

        let mut patterns = Vec::new();
        while let Some(row) = rows.next()? {
            patterns.push(VerifiedPattern {
                id: row.get(0)?,
                category: row.get(1)?,
                pattern: row.get(2)?,
                project: row.get(3)?,
                discovered_by: row.get(4)?,
                confidence: row.get(5)?,
                source_file: row.get(6)?,
                source_occurrences: row.get::<_, i64>(7)? as usize,
                source_snippet: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
                usage_count: row.get::<_, i64>(11)? as usize,
                verification: row.get(12)?,
            });
        }

        Ok(patterns)
    }

    /// Get a pattern by ID
    pub fn get(&self, id: &str) -> rusqlite::Result<Option<VerifiedPattern>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT * FROM patterns WHERE id = ?")?;

        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(VerifiedPattern {
                id: row.get(0)?,
                category: row.get(1)?,
                pattern: row.get(2)?,
                project: row.get(3)?,
                discovered_by: row.get(4)?,
                confidence: row.get(5)?,
                source_file: row.get(6)?,
                source_occurrences: row.get::<_, i64>(7)? as usize,
                source_snippet: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
                usage_count: row.get::<_, i64>(11)? as usize,
                verification: row.get(12)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Delete a pattern by ID
    pub fn delete(&self, id: &str) -> rusqlite::Result<bool> {
        let conn = self.conn.lock();
        let affected = conn.execute("DELETE FROM patterns WHERE id = ?", params![id])?;
        Ok(affected > 0)
    }

    /// Verify a pattern manually
    pub fn verify(&self, id: &str) -> rusqlite::Result<bool> {
        let conn = self.conn.lock();
        let now = Utc::now().to_rfc3339();
        let affected = conn.execute(
            "UPDATE patterns SET verification = 'verified', updated_at = ? WHERE id = ?",
            params![&now, id],
        )?;
        Ok(affected > 0)
    }

    /// Increment usage count and auto-verify if threshold met
    pub fn increment_usage(&self, id: &str) -> rusqlite::Result<bool> {
        let conn = self.conn.lock();
        let now = Utc::now().to_rfc3339();

        // Get current state
        let (usage_count, confidence): (i64, f32) = conn.query_row(
            "SELECT usage_count, confidence FROM patterns WHERE id = ?",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        let new_usage = usage_count + 1;

        // Auto-verify if usage >= 5 and confidence >= 0.7
        let new_verification = if new_usage >= 5 && confidence >= 0.7 {
            "auto_verified"
        } else {
            "pending"
        };

        let affected = conn.execute(
            "UPDATE patterns SET usage_count = ?, verification = ?, updated_at = ? WHERE id = ?",
            params![new_usage, new_verification, &now, id],
        )?;

        Ok(affected > 0)
    }

    /// Apply confidence decay for unused patterns (30+ days)
    pub fn apply_decay(&self) -> rusqlite::Result<usize> {
        let conn = self.conn.lock();
        let cutoff = Utc::now() - chrono::Duration::days(30);

        let affected = conn.execute(
            r#"UPDATE patterns
               SET confidence = MAX(0.0, confidence - 0.1)
               WHERE verification = 'pending'
               AND datetime(updated_at) < datetime(?)"#,
            params![cutoff.to_rfc3339()],
        )?;

        Ok(affected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_conversion() {
        assert_eq!(PatternCategory::Naming.as_str(), "naming");
        assert_eq!(
            PatternCategory::from_category("naming"),
            Some(PatternCategory::Naming)
        );
        assert_eq!(
            PatternCategory::from_category("NAMING"),
            Some(PatternCategory::Naming)
        );
    }

    #[test]
    fn test_verification_conversion() {
        assert_eq!(PatternVerification::Verified.as_str(), "verified");
        assert_eq!(
            PatternVerification::from_verification("verified"),
            PatternVerification::Verified
        );
        assert_eq!(
            PatternVerification::from_verification("unknown"),
            PatternVerification::Pending
        );
    }
}
