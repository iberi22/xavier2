//! Pattern domain types for Xavier cognitive memory system.
//!
//! Patterns represent discovered code patterns, naming conventions, architectural
//! decisions, and workflow practices that have been verified through usage.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A verified pattern discovered in a codebase or project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiedPattern {
    /// Unique identifier for this pattern
    pub id: String,
    /// Category of the pattern
    pub category: PatternCategory,
    /// The actual pattern text or code snippet
    pub pattern: String,
    /// Which project this pattern belongs to (e.g., "xavier", "gestalt-rust")
    pub project: String,
    /// Agent or session that discovered this pattern
    pub discovered_by: String,
    /// Confidence score from 0.0 to 1.0 based on evidence
    pub confidence: f32,
    /// File path where the pattern was found
    pub source_file: String,
    /// Number of times this pattern was seen in the codebase
    pub source_occurrences: usize,
    /// Code/text snippet showing the pattern in context
    pub source_snippet: String,
    /// When this pattern was first discovered
    pub created_at: DateTime<Utc>,
    /// When this pattern was last updated
    pub updated_at: DateTime<Utc>,
    /// Number of times this pattern was retrieved or used
    pub usage_count: usize,
    /// Verification status of this pattern
    pub verification: PatternVerification,
}

/// Categories for pattern classification
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PatternCategory {
    Naming,
    Structure,
    Workflow,
    Convention,
    Architecture,
}

impl PatternCategory {
    /// Parse a string into a PatternCategory, case-insensitive
    pub fn from_category(s: &str) -> Self {
        match s.trim_matches('"').to_lowercase().as_str() {
            "naming" => PatternCategory::Naming,
            "structure" => PatternCategory::Structure,
            "workflow" => PatternCategory::Workflow,
            "convention" => PatternCategory::Convention,
            "architecture" => PatternCategory::Architecture,
            _ => PatternCategory::Convention,
        }
    }
}

/// Verification status for patterns
#[derive(Default, Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PatternVerification {
    #[default]
    Pending,
    Verified,
    Rejected,
    Expired,
}

impl PatternVerification {
    /// Parse a string into a PatternVerification, case-insensitive
    pub fn from_verification(s: &str) -> Self {
        match s.trim_matches('"').to_lowercase().as_str() {
            "pending" => PatternVerification::Pending,
            "verified" => PatternVerification::Verified,
            "rejected" => PatternVerification::Rejected,
            "expired" => PatternVerification::Expired,
            _ => PatternVerification::Pending,
        }
    }
}
