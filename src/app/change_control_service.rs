use crate::domain::change_control::{
    AgentTask, AgentTaskStatus, ChangeScope, ConflictReport, ConflictSeverity, ConflictType,
    FileLease, ImpactReport, LeaseMode, LeaseStatus, RiskLevel,
};
use crate::ports::inbound::change_control_port::{
    BlockedTask, ChangeControlPort, LeaseResponse, MergePlan, TaskCompletionResult,
    ValidationResult,
};
use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use ulid::Ulid;
use anyhow::Result;

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

pub struct ChangeControlService {
    tasks: RwLock<HashMap<String, AgentTask>>,
    leases: RwLock<HashMap<String, FileLease>>,
}

impl ChangeControlService {
    pub fn new() -> Self {
        Self {
            tasks: RwLock::new(HashMap::new()),
            leases: RwLock::new(HashMap::new()),
        }
    }

    /// Search for relevant architectural decisions affecting the given file patterns.
    /// Returns memory_context strings for the LeaseResponse.
    pub async fn search_decisions(&self, patterns: &[String]) -> Result<Vec<String>> {
        // For now, return decisions from known ADR paths
        // In Phase 4, this will query QmdMemory properly
        let mut context = Vec::new();

        // Check if patterns touch critical architecture files
        for pattern in patterns {
            if pattern.contains("memory/") || pattern.contains("qmd_memory") {
                context.push("ADR-001: QmdMemory como dominio central".to_string());
            }
            if pattern.contains("ports/") {
                context.push("ADR-002: Ports solo donde hay swappeo real".to_string());
            }
            if pattern.contains("domain/") {
                context.push("ADR-005: Domain purity - no adapters imports".to_string());
            }
        }

        Ok(context)
    }

    /// Calculate the risk impact of touching the given file patterns.
    /// Uses heuristics-based analysis. Phase 4 will use code-graph for real graph queries.
    pub async fn calculate_impact(&self, patterns: &[String]) -> ImpactReport {
        let mut score = 0.0f32;
        let mut affected = Vec::new();
        let mut contracts = Vec::new();

        for pattern in patterns {
            // Critical files -> high risk
            if pattern.contains("qmd_memory") || pattern.contains("memory/") {
                score = score.max(0.9);
                affected.push(pattern.clone());
                contracts.push("MemoryQueryPort".to_string());
            }
            if pattern.contains("domain/memory") {
                score = score.max(0.95);
                contracts.push("MemoryDomain".to_string());
            }
            if pattern.contains("ports/inbound") {
                score = score.max(0.8);
                contracts.push("PortContract".to_string());
            }
            if pattern.contains("settings.rs") {
                score = score.max(0.7);
            }

            // Directories with many dependents
            if pattern.contains("src/memory/") {
                score = score.max(0.6);
                affected.push(pattern.clone());
            }
        }

        // Fallback: if no heuristics matched, low risk
        if score == 0.0 {
            score = 0.2;
        }

        let risk_level = if score >= 0.9 {
            RiskLevel::Critical
        } else if score >= 0.7 {
            RiskLevel::High
        } else if score >= 0.4 {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        };

        ImpactReport {
            score,
            symbols_affected: affected.len(),
            dependent_files: affected,
            contracts_affected: contracts,
            risk_level,
            recommendation: if score >= 0.9 {
                "Blocked — requires architect approval".to_string()
            } else if score >= 0.7 {
                "Consider splitting into smaller tasks".to_string()
            } else if score >= 0.4 {
                "Proceed with caution — run full test suite".to_string()
            } else {
                "Safe to proceed".to_string()
            },
        }
    }

    /// Generate a reusable summary after task completion.
    pub fn generate_change_summary(task: &AgentTask, _result: &serde_json::Value) -> String {
        let files = task.scope.allowed_write.join(", ");
        let contracts = task.scope.contracts_affected.join(", ");
        format!(
            "Task '{}' (agent: {}) modified [{}]. Contracts affected: [{}]. Risk level: {:?}.",
            task.title, task.agent_id, files, contracts, task.risk_level
        )
    }
}

impl Default for ChangeControlService {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Lease helpers
// ---------------------------------------------------------------------------

/// Check whether two glob patterns may overlap by comparing directory prefixes.
fn patterns_overlap(a: &str, b: &str) -> bool {
    // Naive overlap check: if one pattern starts with the other's prefix (up to the last `/` or `\`),
    // or if the common prefix of both patterns is non-empty.
    let common_len = a
        .chars()
        .zip(b.chars())
        .take_while(|(ca, cb)| ca == cb)
        .count();

    if common_len == 0 {
        return false;
    }

    // If they share at least one full directory level, treat as overlapping.
    let prefix = &a[..common_len];
    // A single character (e.g. a file name) is not a directory overlap.
    if prefix.len() <= 1 {
        return false;
    }
    // If the common prefix ends at a path separator or is a complete path component it's a match.
    true
}

#[allow(dead_code)]
fn is_lease_expired(lease: &FileLease) -> bool {
    lease.status == LeaseStatus::Active && lease.expires_at < Utc::now().timestamp()
}

// ---------------------------------------------------------------------------
// Port implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl ChangeControlPort for ChangeControlService {
    async fn create_task(&self, mut task: AgentTask) -> anyhow::Result<String> {
        let id = Ulid::new().to_string();
        let now = Utc::now().timestamp();
        task.id = id.clone();
        task.status = AgentTaskStatus::Draft;
        task.created_at = now;
        task.updated_at = now;

        let mut tasks = self.tasks.write().await;
        tasks.insert(id.clone(), task);
        Ok(id)
    }

    async fn get_task(&self, id: &str) -> anyhow::Result<Option<AgentTask>> {
        let tasks = self.tasks.read().await;
        Ok(tasks.get(id).cloned())
    }

    async fn list_tasks(&self) -> anyhow::Result<Vec<AgentTask>> {
        let tasks = self.tasks.read().await;
        Ok(tasks.values().cloned().collect())
    }

    async fn claim_lease(
        &self,
        agent_id: &str,
        task_id: &str,
        patterns: Vec<String>,
        mode: LeaseMode,
        ttl_seconds: i64,
    ) -> anyhow::Result<LeaseResponse> {
        let now = Utc::now().timestamp();
        let mut leases = self.leases.write().await;

        // Collect conflicts with other active leases
        let mut conflicts: Vec<ConflictReport> = Vec::new();

        // Prune expired leases
        leases.retain(|_, lease| {
            if lease.status == LeaseStatus::Active && lease.expires_at < now {
                lease.status = LeaseStatus::Expired;
            }
            true // keep all entries even if expired
        });

        // Check for pattern overlaps with active leases from *other* tasks
        for lease in leases.values() {
            if lease.status != LeaseStatus::Active || lease.expires_at < now {
                continue;
            }
            if lease.task_id == task_id {
                continue;
            }

            for pat in &patterns {
                for existing_pat in &lease.patterns {
                    if patterns_overlap(pat, existing_pat) {
                        conflicts.push(ConflictReport {
                            task_id: task_id.to_string(),
                            conflicting_task_id: lease.task_id.clone(),
                            conflict_type: ConflictType::DirectFileOverlap,
                            files: vec![pat.clone(), existing_pat.clone()],
                            contracts: Vec::new(),
                            severity: ConflictSeverity::Blocking,
                            recommendation: format!(
                                "Pattern '{}' overlaps with lease '{}' on task '{}'",
                                pat, lease.id, lease.task_id
                            ),
                        });
                    }
                }
            }
        }

        // Search for relevant ADR decisions before moving patterns into the lease
        let memory_context = self.search_decisions(&patterns).await.unwrap_or_default();

        // Create the lease (even if conflicts exist — caller decides)
        let lease_id = Ulid::new().to_string();
        let expires_at = now + ttl_seconds;
        let lease = FileLease {
            id: lease_id.clone(),
            task_id: task_id.to_string(),
            agent_id: agent_id.to_string(),
            patterns,
            mode,
            expires_at,
            status: LeaseStatus::Active,
        };
        leases.insert(lease_id.clone(), lease);

        let has_blocking = conflicts
            .iter()
            .any(|c| c.severity == ConflictSeverity::Blocking || c.severity == ConflictSeverity::Critical);

        let status = if has_blocking {
            "conflict_detected".to_string()
        } else {
            "granted".to_string()
        };

        Ok(LeaseResponse {
            status,
            lease_id,
            conflicts,
            memory_context,
            required_checks: vec!["cargo test --lib".to_string(), "cargo clippy".to_string()],
        })
    }

    async fn release_lease(&self, lease_id: &str) -> anyhow::Result<()> {
        let mut leases = self.leases.write().await;
        if let Some(lease) = leases.get_mut(lease_id) {
            lease.status = LeaseStatus::Released;
        }
        Ok(())
    }

    async fn active_leases(&self) -> anyhow::Result<Vec<FileLease>> {
        let now = Utc::now().timestamp();
        let leases = self.leases.read().await;
        let active: Vec<FileLease> = leases
            .values()
            .filter(|l| l.status == LeaseStatus::Active && l.expires_at >= now)
            .cloned()
            .collect();
        Ok(active)
    }

    async fn check_conflicts(&self, task_id: &str) -> anyhow::Result<Vec<ConflictReport>> {
        let tasks = self.tasks.read().await;
        let task = match tasks.get(task_id) {
            Some(t) => t.clone(),
            None => return Ok(Vec::new()),
        };

        let mut reports: Vec<ConflictReport> = Vec::new();
        for (other_id, other) in tasks.iter() {
            if other_id == task_id {
                continue;
            }
            if other.status == AgentTaskStatus::Completed
                || other.status == AgentTaskStatus::Failed
                || other.status == AgentTaskStatus::Cancelled
            {
                continue;
            }

            // Check for scope overlap (write patterns intersecting)
            for our_pat in &task.scope.allowed_write {
                for their_pat in &other.scope.allowed_write {
                    if patterns_overlap(our_pat, their_pat) {
                        reports.push(ConflictReport {
                            task_id: task_id.to_string(),
                            conflicting_task_id: other_id.clone(),
                            conflict_type: ConflictType::DirectFileOverlap,
                            files: vec![our_pat.clone(), their_pat.clone()],
                            contracts: Vec::new(),
                            severity: ConflictSeverity::Warning,
                            recommendation: format!(
                                "Task '{}' and task '{}' may both write to overlapping paths",
                                task_id, other_id
                            ),
                        });
                    }
                }
            }
        }

        Ok(reports)
    }

    async fn validate_change(&self, _scope: &ChangeScope) -> anyhow::Result<ValidationResult> {
        // Stub: always pass
        Ok(ValidationResult {
            passed: true,
            violations: Vec::new(),
            warnings: Vec::new(),
        })
    }

    async fn complete_task(
        &self,
        task_id: &str,
        result: serde_json::Value,
    ) -> anyhow::Result<TaskCompletionResult> {
        let mut tasks = self.tasks.write().await;

        if let Some(task) = tasks.get_mut(task_id) {
            task.status = AgentTaskStatus::Completed;
            task.updated_at = Utc::now().timestamp();
        }

        // Retrieve a snapshot of the completed task for summary generation
        let tasks_snapshot = self.tasks.read().await;
        let summary = if let Some(task) = tasks_snapshot.get(task_id) {
            Self::generate_change_summary(task, &result)
        } else {
            format!("Task '{}' completed", task_id)
        };
        drop(tasks_snapshot);

        Ok(TaskCompletionResult {
            task_id: task_id.to_string(),
            summary,
        })
    }

    async fn merge_plan(&self) -> anyhow::Result<MergePlan> {
        let tasks = self.tasks.read().await;
        let leases = self.leases.read().await;
        let _ = leases; // keep lease info available for future conflict analysis

        // Collect active (non-completed, non-failed, non-cancelled) tasks
        let active_tasks: Vec<&AgentTask> = tasks
            .values()
            .filter(|t| {
                t.status != AgentTaskStatus::Completed
                    && t.status != AgentTaskStatus::Failed
                    && t.status != AgentTaskStatus::Cancelled
            })
            .collect();

        // Phase 1: Find safe parallel groups (tasks with no conflicting scopes)
        let mut safe_parallel_groups: Vec<Vec<String>> = Vec::new();
        let mut assigned: std::collections::HashSet<String> = std::collections::HashSet::new();

        for (i, task_a) in active_tasks.iter().enumerate() {
            if assigned.contains(&task_a.id) {
                continue;
            }

            let mut group = vec![task_a.id.clone()];
            assigned.insert(task_a.id.clone());

            for task_b in active_tasks.iter().skip(i + 1) {
                if assigned.contains(&task_b.id) {
                    continue;
                }

                // Check if task_a and task_b conflict
                let conflict = Self::scopes_conflict(&task_a.scope, &task_b.scope);
                if !conflict {
                    group.push(task_b.id.clone());
                    assigned.insert(task_b.id.clone());
                }
            }

            safe_parallel_groups.push(group);
        }

        // Phase 2: Find sequential tasks (those with explicit dependencies)
        let sequential: Vec<String> = active_tasks
            .iter()
            .filter(|t| !t.dependencies.is_empty())
            .map(|t| t.id.clone())
            .collect();

        // Phase 3: Find blocked tasks (unmet dependencies)
        let blocked: Vec<BlockedTask> = active_tasks
            .iter()
            .filter(|t| {
                t.dependencies.iter().any(|dep_id| {
                    // Blocked if dependency is missing or not yet completed
                    tasks.get(dep_id).map_or(true, |dep| {
                        dep.status != AgentTaskStatus::Completed
                    })
                })
            })
            .map(|t| BlockedTask {
                task: t.id.clone(),
                reason: format!(
                    "Dependencies not met: {}",
                    t.dependencies
                        .iter()
                        .filter(|dep_id| {
                            tasks.get(*dep_id).map_or(true, |dep| {
                                dep.status != AgentTaskStatus::Completed
                            })
                        })
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                blocked_by: t.dependencies.clone(),
            })
            .collect();

        Ok(MergePlan {
            safe_parallel_groups,
            sequential,
            blocked,
        })
    }
}

impl ChangeControlService {
    /// Check if two ChangeScopes have conflicting write patterns.
    fn scopes_conflict(a: &ChangeScope, b: &ChangeScope) -> bool {
        // If either has blocked patterns that the other writes to
        for pattern_a in &a.allowed_write {
            for pattern_b in &b.allowed_write {
                if Self::files_overlap(pattern_a, pattern_b) {
                    return true;
                }
            }
        }
        // Check blocked vs allowed_write (a's blocked patterns vs b's writes)
        for blocked_a in &a.blocked {
            for write_b in &b.allowed_write {
                if Self::files_overlap(blocked_a, write_b) {
                    return true;
                }
            }
        }
        // Check blocked vs allowed_write (b's blocked patterns vs a's writes)
        for blocked_b in &b.blocked {
            for write_a in &a.allowed_write {
                if Self::files_overlap(blocked_b, write_a) {
                    return true;
                }
            }
        }
        false
    }

    /// Check if two file patterns overlap (shared directory prefix).
    fn files_overlap(a: &str, b: &str) -> bool {
        let a_parts: Vec<&str> = a.split('/').collect();
        let b_parts: Vec<&str> = b.split('/').collect();

        // Remove wildcards and file extensions for comparison
        let a_dir: Vec<&str> = a_parts
            .iter()
            .take_while(|p| !p.contains('.'))
            .copied()
            .collect();
        let b_dir: Vec<&str> = b_parts
            .iter()
            .take_while(|p| !p.contains('.'))
            .copied()
            .collect();

        // Check if they share a common directory prefix
        let common = a_dir
            .iter()
            .zip(b_dir.iter())
            .take_while(|(a, b)| a == b)
            .count();

        common > 0 && common >= a_dir.len().min(b_dir.len())
    }
}
