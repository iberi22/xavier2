//! Retention Regularizer - Memory coherence and semantic drift detection
//!
//! Implements retention regularization for detecting conflicts, verifying
//! entity consistency, checking temporal ordering, and scoring memory coherence.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::memory::entity_graph::EntityRecord;
use crate::memory::qmd_memory::MemoryDocument;

/// Threshold configuration for drift detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftThreshold {
    /// Minimum property difference to trigger drift alert (0.0-1.0)
    pub property_diff: f32,
    /// Minimum trust score change to trigger drift (0.0-1.0)
    pub trust_change: f32,
    /// Minimum occurrence count change to trigger drift
    pub occurrence_change: i32,
}

impl Default for DriftThreshold {
    fn default() -> Self {
        Self {
            property_diff: 0.2,
            trust_change: 0.15,
            occurrence_change: 5,
        }
    }
}

/// Configuration for retention regularizer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegularizerConfig {
    /// Drift detection thresholds
    pub drift_threshold: DriftThreshold,
    /// Minimum coherence score to pass (0.0-1.0)
    pub min_coherence_score: f32,
    /// Enable conflict detection
    pub conflict_detection_enabled: bool,
    /// Enable drift detection
    pub drift_detection_enabled: bool,
}

impl Default for RegularizerConfig {
    fn default() -> Self {
        Self {
            drift_threshold: DriftThreshold::default(),
            min_coherence_score: 0.7,
            conflict_detection_enabled: true,
            drift_detection_enabled: true,
        }
    }
}

/// Report on memory coherence issues
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoherenceReport {
    /// Overall coherence score (0.0-1.0)
    pub coherence_score: f32,
    /// List of detected conflicts
    pub conflicts: Vec<Conflict>,
    /// List of entity drift alerts
    pub drift_alerts: Vec<DriftAlert>,
    /// List of temporal ordering issues
    pub temporal_issues: Vec<TemporalIssue>,
    /// Whether coherence passes the threshold
    pub passes: bool,
    /// Detailed findings
    pub findings: Vec<String>,
}

/// A detected conflict between memories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    /// Type of conflict
    pub conflict_type: ConflictType,
    /// Memory IDs involved
    pub memory_ids: Vec<String>,
    /// Description of the conflict
    pub description: String,
    /// Severity (0.0-1.0, higher = more severe)
    pub severity: f32,
}

/// Types of conflicts that can be detected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictType {
    /// Two facts contradict each other
    FactualContradiction,
    /// Entity properties don't match across memories
    EntityPropertyMismatch,
    /// Temporal ordering is violated
    TemporalOrderingViolation,
    /// Same entity has different names/aliases
    NamingConflict,
    /// Duplicate memories with different content
    DuplicateInconsistency,
}

impl ConflictType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::FactualContradiction => "factual_contradiction",
            Self::EntityPropertyMismatch => "entity_property_mismatch",
            Self::TemporalOrderingViolation => "temporal_ordering_violation",
            Self::NamingConflict => "naming_conflict",
            Self::DuplicateInconsistency => "duplicate_inconsistency",
        }
    }
}

/// Alert for semantic drift in entity properties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftAlert {
    /// Entity ID that has drifted
    pub entity_id: String,
    /// Entity name
    pub entity_name: String,
    /// Type of drift detected
    pub drift_type: DriftType,
    /// Description of the drift
    pub description: String,
    /// Severity of drift (0.0-1.0)
    pub severity: f32,
}

/// Types of semantic drift
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DriftType {
    /// Trust score has significantly changed
    TrustScoreChange,
    /// Property values have diverged
    PropertyDivergence,
    /// Entity type may have changed
    EntityTypeShift,
    /// Occurrence count anomaly
    OccurrenceAnomaly,
    /// Description updated significantly
    DescriptionDrift,
}

impl DriftType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TrustScoreChange => "trust_score_change",
            Self::PropertyDivergence => "property_divergence",
            Self::EntityTypeShift => "entity_type_shift",
            Self::OccurrenceAnomaly => "occurrence_anomaly",
            Self::DescriptionDrift => "description_drift",
        }
    }
}

/// Issue with temporal ordering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalIssue {
    /// Memory IDs involved
    pub memory_ids: Vec<String>,
    /// Description of the issue
    pub description: String,
    /// Expected temporal order
    pub expected_order: String,
}

/// Retention regularizer for memory coherence
#[derive(Debug, Clone)]
pub struct RetentionRegularizer {
    config: RegularizerConfig,
}

impl RetentionRegularizer {
    pub fn new(config: RegularizerConfig) -> Self {
        Self { config }
    }

    pub fn with_defaults() -> Self {
        Self {
            config: RegularizerConfig::default(),
        }
    }

    /// Check coherence of a set of memories
    pub fn check_coherence(&self, memories: &[MemoryDocument]) -> CoherenceReport {
        let mut conflicts = Vec::new();
        let mut temporal_issues = Vec::new();
        let mut findings = Vec::new();

        if self.config.conflict_detection_enabled {
            // Detect factual contradictions
            let factual = self.detect_factual_conflicts(memories);
            conflicts.extend(factual);

            // Detect entity property mismatches
            let entity_conflicts = self.detect_entity_conflicts(memories);
            conflicts.extend(entity_conflicts);

            // Detect duplicate inconsistencies
            let duplicates = self.detect_duplicate_inconsistencies(memories);
            conflicts.extend(duplicates);
        }

        // Check temporal ordering
        let temporal = self.check_temporal_order(memories);
        temporal_issues.extend(temporal);

        // Calculate overall coherence score
        let coherence_score = self.calculate_coherence_score(&conflicts, &temporal_issues);

        let passes = coherence_score >= self.config.min_coherence_score;

        if passes {
            findings.push("Memory coherence check passed".to_string());
        } else {
            findings.push(format!(
                "Memory coherence check failed: score {} below threshold {}",
                coherence_score, self.config.min_coherence_score
            ));
        }

        CoherenceReport {
            coherence_score,
            conflicts,
            drift_alerts: Vec::new(), // Populated separately via detect_drift
            temporal_issues,
            passes,
            findings,
        }
    }

    /// Check coherence with entity drift detection
    pub fn check_coherence_with_entities(
        &self,
        memories: &[MemoryDocument],
        entities: &[EntityRecord],
    ) -> CoherenceReport {
        let mut report = self.check_coherence(memories);

        if self.config.drift_detection_enabled {
            let drift_alerts = self.detect_entity_drifts(entities);
            report.drift_alerts = drift_alerts;

            // Adjust coherence score based on drift severity
            let drift_penalty: f32 =
                report.drift_alerts.iter().map(|a| a.severity).sum::<f32>() * 0.1;
            report.coherence_score = (report.coherence_score - drift_penalty).max(0.0);
            report.passes = report.coherence_score >= self.config.min_coherence_score;
        }

        report
    }

    /// Detect semantic drift between old and new entity versions
    pub fn detect_drift(&self, old: &EntityRecord, new: &EntityRecord) -> Option<DriftAlert> {
        // Check occurrence count anomaly (primary drift indicator for EntityRecord)
        let occurrence_diff = (new.occurrence_count as i32) - (old.occurrence_count as i32);
        if occurrence_diff.abs() >= self.config.drift_threshold.occurrence_change {
            return Some(DriftAlert {
                entity_id: new.id.clone(),
                entity_name: new.name.clone(),
                drift_type: DriftType::OccurrenceAnomaly,
                description: format!(
                    "Occurrence count changed from {} to {}",
                    old.occurrence_count, new.occurrence_count
                ),
                severity: (occurrence_diff.abs() as f32 / 50.0).min(1.0),
            });
        }

        // Check trust score change
        let trust_diff = (old.trust_score - new.trust_score).abs();
        if trust_diff >= self.config.drift_threshold.trust_change {
            return Some(DriftAlert {
                entity_id: new.id.clone(),
                entity_name: new.name.clone(),
                drift_type: DriftType::TrustScoreChange,
                description: format!(
                    "Trust score changed from {:.2} to {:.2}",
                    old.trust_score, new.trust_score
                ),
                severity: trust_diff.min(1.0),
            });
        }

        // Check description drift
        if let (Some(old_desc), Some(new_desc)) = (&old.description, &new.description) {
            if old_desc != new_desc {
                // Calculate similarity between descriptions
                let similarity = self.string_similarity(old_desc, new_desc);
                if similarity < 0.5 {
                    return Some(DriftAlert {
                        entity_id: new.id.clone(),
                        entity_name: new.name.clone(),
                        drift_type: DriftType::DescriptionDrift,
                        description: format!(
                            "Description significantly changed (similarity: {:.2})",
                            similarity
                        ),
                        severity: (1.0 - similarity).min(1.0),
                    });
                }
            }
        }

        None
    }

    /// Detect drift across all entities in a collection
    pub fn detect_entity_drifts(&self, entities: &[EntityRecord]) -> Vec<DriftAlert> {
        let mut alerts = Vec::new();
        let mut seen: HashMap<String, &EntityRecord> = HashMap::new();

        // Group entities by normalized name to detect drift
        for entity in entities {
            let key = entity.normalized_name.clone();
            if let Some(old_entity) = seen.get(&key) {
                if let Some(alert) = self.detect_drift(old_entity, entity) {
                    alerts.push(alert);
                }
            } else {
                seen.insert(key, entity);
            }
        }

        // Check for rapid occurrence count changes
        for entity in entities {
            // Occurrence spike detection (more than 3x previous)
            if entity.occurrence_count > 30 && entity.memory_count == 0 {
                alerts.push(DriftAlert {
                    entity_id: entity.id.clone(),
                    entity_name: entity.name.clone(),
                    drift_type: DriftType::OccurrenceAnomaly,
                    description: format!(
                        "High occurrence count ({}) with no memory associations",
                        entity.occurrence_count
                    ),
                    severity: 0.6,
                });
            }
        }

        alerts
    }

    /// Detect factual contradictions in memories
    fn detect_factual_conflicts(&self, memories: &[MemoryDocument]) -> Vec<Conflict> {
        let mut conflicts = Vec::new();
        let mut fact_groups: HashMap<String, Vec<&MemoryDocument>> = HashMap::new();

        // Group memories by normalized content signature
        for memory in memories {
            let signature = self.normalize_fact_signature(&memory.content);
            if !signature.is_empty() {
                fact_groups.entry(signature).or_default().push(memory);
            }
        }

        // Check for conflicting facts within same group
        for (signature, group) in fact_groups {
            if group.len() > 1 {
                // Check if content is actually contradictory
                let contents: Vec<&str> = group.iter().map(|m| m.content.as_str()).collect();
                if self.has_negation_conflict(&contents) {
                    conflicts.push(Conflict {
                        conflict_type: ConflictType::FactualContradiction,
                        memory_ids: group.iter().filter_map(|m| m.id.clone()).collect(),
                        description: format!("Contradictory statements detected: {}", signature),
                        severity: 0.8,
                    });
                }
            }
        }

        conflicts
    }

    /// Detect entity property mismatches across memories
    fn detect_entity_conflicts(&self, memories: &[MemoryDocument]) -> Vec<Conflict> {
        let mut conflicts = Vec::new();
        let mut entity_properties: HashMap<String, HashMap<String, Vec<String>>> = HashMap::new();

        // Extract entity references from memories
        for memory in memories {
            let memory_id = memory.id.clone().unwrap_or_default();
            let entities = self.extract_entity_references(&memory.content);

            for entity in entities {
                entity_properties
                    .entry(entity.0)
                    .or_default()
                    .entry(entity.1)
                    .or_default()
                    .push(memory_id.clone());
            }
        }

        // Detect property mismatches
        for (entity_name, properties) in entity_properties {
            let mut property_values: HashMap<String, Vec<String>> = HashMap::new();
            for (prop, memory_ids) in properties {
                if memory_ids.len() > 1 {
                    property_values.insert(prop, memory_ids);
                }
            }

            // If same entity has conflicting property assignments
            if property_values.len() > 1 {
                let all_memory_ids: Vec<String> =
                    property_values.values().flatten().cloned().collect();
                conflicts.push(Conflict {
                    conflict_type: ConflictType::EntityPropertyMismatch,
                    memory_ids: all_memory_ids,
                    description: format!(
                        "Entity '{}' has conflicting property assignments",
                        entity_name
                    ),
                    severity: 0.5,
                });
            }
        }

        conflicts
    }

    /// Detect duplicate memories with inconsistent content
    fn detect_duplicate_inconsistencies(&self, memories: &[MemoryDocument]) -> Vec<Conflict> {
        let mut conflicts = Vec::new();
        let mut seen_signatures: HashMap<String, Vec<&MemoryDocument>> = HashMap::new();

        for memory in memories {
            let sig = self.duplicate_signature(&memory.content);
            if !sig.is_empty() {
                seen_signatures.entry(sig).or_default().push(memory);
            }
        }

        for (_, group) in seen_signatures {
            if group.len() > 1 {
                // Check if content diverged significantly
                let first_content = &group[0].content;
                for other in &group[1..] {
                    let similarity = self.string_similarity(first_content, &other.content);
                    if similarity < 0.7 {
                        conflicts.push(Conflict {
                            conflict_type: ConflictType::DuplicateInconsistency,
                            memory_ids: group.iter().filter_map(|m| m.id.clone()).collect(),
                            description: format!(
                                "Duplicates have diverged content (similarity: {:.2})",
                                similarity
                            ),
                            severity: 1.0 - similarity,
                        });
                        break;
                    }
                }
            }
        }

        conflicts
    }

    /// Check temporal ordering of memories
    fn check_temporal_order(&self, memories: &[MemoryDocument]) -> Vec<TemporalIssue> {
        let mut issues = Vec::new();
        let mut temporal_markers: Vec<(&str, &MemoryDocument)> = Vec::new();

        // Extract temporal markers from content
        for memory in memories {
            if let Some(temporal) = self.extract_temporal_marker(&memory.content) {
                temporal_markers.push((temporal, memory));
            }
        }

        // Check for ordering conflicts
        for i in 0..temporal_markers.len() {
            for j in (i + 1)..temporal_markers.len() {
                let (marker1, mem1) = temporal_markers[i];
                let (marker2, mem2) = temporal_markers[j];

                if marker1 != marker2 {
                    // Check if temporal relationship is violated
                    if let Some(violation) =
                        self.check_temporal_violation(marker1, mem1, marker2, mem2)
                    {
                        issues.push(violation);
                    }
                }
            }
        }

        issues
    }

    /// Calculate overall coherence score
    fn calculate_coherence_score(
        &self,
        conflicts: &[Conflict],
        temporal_issues: &[TemporalIssue],
    ) -> f32 {
        if conflicts.is_empty() && temporal_issues.is_empty() {
            return 1.0;
        }

        // Penalize based on conflict severity
        let conflict_penalty: f32 = conflicts.iter().map(|c| c.severity).sum::<f32>() * 0.15;
        let temporal_penalty = (temporal_issues.len() as f32) * 0.05;

        (1.0 - conflict_penalty - temporal_penalty).max(0.0)
    }

    /// Create a normalized signature for fact comparison
    fn normalize_fact_signature(&self, content: &str) -> String {
        content
            .to_lowercase()
            .split_whitespace()
            .take(10)
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Create a signature for duplicate detection
    fn duplicate_signature(&self, content: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let normalized: String = content
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect();

        let mut hasher = DefaultHasher::new();
        normalized.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Check if contents have negation-based conflict
    fn has_negation_conflict(&self, contents: &[&str]) -> bool {
        let negation_words = [
            "not", "no", "never", "neither", "none", "doesn't", "isn't", "wasn't",
        ];

        let mut has_affirmative = false;
        let mut has_negative = false;

        for content in contents {
            let lower = content.to_lowercase();
            let has_neg = negation_words.iter().any(|w| lower.contains(w));
            if has_neg {
                has_negative = true;
            } else {
                has_affirmative = true;
            }
        }

        has_affirmative && has_negative
    }

    /// Extract entity references from content
    fn extract_entity_references(&self, content: &str) -> Vec<(String, String)> {
        use regex::Regex;
        static ENTITY_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"\b([A-Z][a-z]+(?:\s+[A-Z][a-z]+)*)\b").expect("test assertion")
        });

        let mut refs = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for cap in ENTITY_RE.captures_iter(content) {
            if let Some(name) = cap.get(1) {
                let entity_name = name.as_str().to_string();
                if seen.insert(entity_name.clone()) {
                    refs.push((entity_name, "referenced".to_string()));
                }
            }
        }

        refs
    }

    /// Extract temporal markers from content
    fn extract_temporal_marker(&self, content: &str) -> Option<&'static str> {
        let lower = content.to_lowercase();
        if lower.contains("yesterday") {
            Some("yesterday")
        } else if lower.contains("today") {
            Some("today")
        } else if lower.contains("tomorrow") {
            Some("tomorrow")
        } else if lower.contains("last week") {
            Some("last_week")
        } else if lower.contains("next week") {
            Some("next_week")
        } else {
            None
        }
    }

    /// Check for temporal violation between two markers
    fn check_temporal_violation(
        &self,
        marker1: &str,
        mem1: &MemoryDocument,
        marker2: &str,
        mem2: &MemoryDocument,
    ) -> Option<TemporalIssue> {
        // Simple violation: "yesterday" should not come after "tomorrow"
        let order = |m: &str| -> i32 {
            match m {
                "yesterday" => 0,
                "today" => 1,
                "tomorrow" => 2,
                "last_week" => 0,
                "next_week" => 3,
                _ => 1,
            }
        };

        let _o1 = order(marker1);
        let _o2 = order(marker2);

        // If markers represent contradictory timeframes
        if (marker1 == "tomorrow" && marker2 == "yesterday")
            || (marker1 == "yesterday" && marker2 == "tomorrow")
        {
            return Some(TemporalIssue {
                memory_ids: vec![
                    mem1.id.clone().unwrap_or_default(),
                    mem2.id.clone().unwrap_or_default(),
                ],
                description: format!("Temporal contradiction: '{}' vs '{}'", marker1, marker2),
                expected_order: "Earlier event should come before later event".to_string(),
            });
        }

        None
    }

    /// Simple string similarity using Jaccard index on words
    fn string_similarity(&self, s1: &str, s2: &str) -> f32 {
        let s1_lower = s1.to_lowercase();
        let s2_lower = s2.to_lowercase();
        let words1: std::collections::HashSet<&str> = s1_lower.split_whitespace().collect();
        let words2: std::collections::HashSet<&str> = s2_lower.split_whitespace().collect();

        if words1.is_empty() && words2.is_empty() {
            return 1.0;
        }
        if words1.is_empty() || words2.is_empty() {
            return 0.0;
        }

        let intersection = words1.intersection(&words2).count() as f32;
        let union = words1.union(&words2).count() as f32;

        intersection / union
    }

    /// Get configuration reference
    pub fn config(&self) -> &RegularizerConfig {
        &self.config
    }

    /// Update drift threshold
    pub fn set_drift_threshold(&mut self, threshold: DriftThreshold) {
        self.config.drift_threshold = threshold;
    }

    /// Update minimum coherence score
    pub fn set_min_coherence(&mut self, score: f32) {
        self.config.min_coherence_score = score.clamp(0.0, 1.0);
    }
}

use std::sync::LazyLock;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_drift_threshold_default() {
        let threshold = DriftThreshold::default();
        assert!((threshold.property_diff - 0.2).abs() < 0.001);
        assert!((threshold.trust_change - 0.15).abs() < 0.001);
        assert_eq!(threshold.occurrence_change, 5);
    }

    #[test]
    fn test_detect_trust_score_drift() {
        let regularizer = RetentionRegularizer::with_defaults();
        let old = EntityRecord {
            id: "e1".to_string(),
            name: "Test".to_string(),
            normalized_name: "test".to_string(),
            entity_type: crate::memory::entity_graph::EntityType::Person,
            aliases: vec![],
            description: None,
            occurrence_count: 10,
            memory_count: 5,
            first_seen: Utc::now(),
            last_seen: Utc::now(),
            merged_from: vec![],
            trust_score: 0.0,
            trust_rank: 0,
        };

        let mut new = old.clone();
        new.trust_score = 0.3; // Large change from default 0.0

        let alert = regularizer.detect_drift(&old, &new);
        assert!(alert.is_some());
        let alert = alert.expect("test assertion");
        assert_eq!(alert.drift_type, DriftType::TrustScoreChange);
    }

    #[test]
    fn test_string_similarity() {
        let regularizer = RetentionRegularizer::with_defaults();
        let sim = regularizer.string_similarity("hello world", "hello world");
        assert!((sim - 1.0).abs() < 0.001);

        let sim2 = regularizer.string_similarity("hello world", "hello there");
        assert!(sim2 > 0.3 && sim2 < 0.8);

        let sim3 = regularizer.string_similarity("hello", "goodbye");
        assert!(sim3 < 0.3);
    }

    #[test]
    fn test_coherence_report_passes() {
        let regularizer = RetentionRegularizer::with_defaults();
        let memories = vec![MemoryDocument {
            id: Some("doc1".to_string()),
            path: "test".to_string(),
            content: "BELA works at SWAL today".to_string(),
            metadata: serde_json::json!({}),
            content_vector: None,
            embedding: vec![],
            ..Default::default()
        }];

        let report = regularizer.check_coherence(&memories);
        assert!(report.passes);
        assert!(report.coherence_score >= 0.7);
    }

    #[test]
    fn test_negation_conflict_detection() {
        let regularizer = RetentionRegularizer::with_defaults();
        let contents = vec!["BELA is at SWAL", "BELA is not at SWAL"];
        assert!(regularizer.has_negation_conflict(&contents));

        let contents2 = vec!["BELA is at SWAL", "BELA works at SWAL"];
        assert!(!regularizer.has_negation_conflict(&contents2));
    }

    #[test]
    fn test_duplicate_signature() {
        let regularizer = RetentionRegularizer::with_defaults();
        let sig1 = regularizer.duplicate_signature("Hello World!");
        let sig2 = regularizer.duplicate_signature("Hello World!");
        let sig3 = regularizer.duplicate_signature("Hello World");
        assert_eq!(sig1, sig2);
        assert_eq!(sig1, sig3);
    }
}
