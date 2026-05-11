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
        let id = if task.id.is_empty() {
            Ulid::new().to_string()
        } else {
            task.id.clone()
        };
        let now = Utc::now().timestamp();
        task.id = id.clone();
        task.status = AgentTaskStatus::Draft;
        if task.created_at == 0 {
            task.created_at = now;
        }
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
        // Clone the task under write lock, then drop it before generating summary.
        // Holding a write lock while requesting a read lock on the same RwLock
        // would deadlock (tokio::sync::RwLock panics on this pattern).
        let task_clone = {
            let mut tasks = self.tasks.write().await;
            if let Some(task) = tasks.get_mut(task_id) {
                task.status = AgentTaskStatus::Completed;
                task.updated_at = Utc::now().timestamp();
                Some(task.clone())
            } else {
                None
            }
        }; // write lock dropped here — safe to touch tasks again

        let summary = if let Some(task) = task_clone {
            Self::generate_change_summary(&task, &result)
        } else {
            format!("Task '{}' completed", task_id)
        };

        Ok(TaskCompletionResult {
            task_id: task_id.to_string(),
            summary,
        })
    }

    async fn plan_merge(&self) -> anyhow::Result<MergePlan> {
        let tasks = self.tasks.read().await;

        // Collect all non-terminal tasks (Draft, Claimed, Active)
        let mut active_tasks: Vec<&AgentTask> = tasks
            .values()
            .filter(|t| {
                t.status != AgentTaskStatus::Completed
                    && t.status != AgentTaskStatus::Failed
                    && t.status != AgentTaskStatus::Cancelled
            })
            .collect();

        // Sort by priority (RiskLevel descending, then created_at ascending)
        active_tasks.sort_by(|a, b| {
            b.risk_level
                .cmp(&a.risk_level)
                .then_with(|| a.created_at.cmp(&b.created_at))
        });

        let mut safe_parallel_groups: Vec<Vec<String>> = Vec::new();
        let mut sequential: Vec<String> = Vec::new();
        let mut blocked: Vec<BlockedTask> = Vec::new();

        let mut scheduled_tasks: Vec<&AgentTask> = Vec::new();

        for task in active_tasks {
            // Check for unmet dependencies
            let unmet_deps: Vec<String> = task
                .dependencies
                .iter()
                .filter(|dep_id| {
                    tasks
                        .get(*dep_id)
                        .map_or(true, |dep| dep.status != AgentTaskStatus::Completed)
                })
                .cloned()
                .collect();

            if !unmet_deps.is_empty() {
                blocked.push(BlockedTask {
                    task: task.id.clone(),
                    reason: format!("Unmet dependencies: {}", unmet_deps.join(", ")),
                    blocked_by: unmet_deps,
                });
                continue;
            }

            // Check for scope conflicts with higher-priority tasks
            let mut conflict_found = None;
            for scheduled in &scheduled_tasks {
                if let Some(reason) = Self::get_scope_conflict_reason(&task.scope, &scheduled.scope) {
                    conflict_found = Some((scheduled.id.clone(), reason));
                    break;
                }
            }

            if let Some((conflicting_id, reason)) = conflict_found {
                blocked.push(BlockedTask {
                    task: task.id.clone(),
                    reason: format!("conflicts with {} on {}", conflicting_id, reason),
                    blocked_by: vec![conflicting_id],
                });
                continue;
            }

            // Task is ready to be scheduled
            scheduled_tasks.push(task);

            if task.risk_level >= RiskLevel::High {
                sequential.push(task.id.clone());
            } else {
                // Try to fit into existing parallel groups
                let mut added = false;
                for group in &mut safe_parallel_groups {
                    let mut group_conflict = false;
                    for member_id in group.iter() {
                        if let Some(member) = tasks.get(member_id) {
                            if Self::scopes_conflict(&task.scope, &member.scope) {
                                group_conflict = true;
                                break;
                            }
                        }
                    }
                    if !group_conflict {
                        group.push(task.id.clone());
                        added = true;
                        break;
                    }
                }
                if !added {
                    safe_parallel_groups.push(vec![task.id.clone()]);
                }
            }
        }

        Ok(MergePlan {
            safe_parallel_groups,
            sequential,
            blocked,
        })
    }
}

impl ChangeControlService {
    /// Check if two ChangeScopes have any conflict.
    fn scopes_conflict(a: &ChangeScope, b: &ChangeScope) -> bool {
        Self::get_scope_conflict_reason(a, b).is_some()
    }

    /// Check if two ChangeScopes have conflicting write patterns, contracts, or layers.
    /// Returns a descriptive reason if they conflict.
    fn get_scope_conflict_reason(a: &ChangeScope, b: &ChangeScope) -> Option<String> {
        // 1. Write-Write overlap
        for pattern_a in &a.allowed_write {
            for pattern_b in &b.allowed_write {
                if Self::files_overlap(pattern_a, pattern_b) {
                    return Some(format!("overlapping write paths: {} and {}", pattern_a, pattern_b));
                }
            }
        }
        // 2. Write-Blocked overlap (a writes to what b blocks, or vice versa)
        for write_a in &a.allowed_write {
            for blocked_b in &b.blocked {
                if Self::files_overlap(write_a, blocked_b) {
                    return Some(format!("write to blocked path: {}", write_a));
                }
            }
        }
        for write_b in &b.allowed_write {
            for blocked_a in &a.blocked {
                if Self::files_overlap(write_b, blocked_a) {
                    return Some(format!("higher priority task blocks path: {}", write_b));
                }
            }
        }
        // 3. Contracts overlap
        for contract_a in &a.contracts_affected {
            if b.contracts_affected.contains(contract_a) {
                return Some(format!("shared contract: {}", contract_a));
            }
        }
        // 4. Layers overlap
        for layer_a in &a.layers_affected {
            if b.layers_affected.contains(layer_a) {
                return Some(format!("shared layer: {}", layer_a));
            }
        }
        None
    }

    /// Check if two file patterns overlap (shared directory prefix).
    pub(crate) fn files_overlap(a: &str, b: &str) -> bool {
        if a == b {
            return true;
        }

        let a_parts: Vec<&str> = a.split('/').filter(|p| !p.is_empty()).collect();
        let b_parts: Vec<&str> = b.split('/').filter(|p| !p.is_empty()).collect();

        let common = a_parts
            .iter()
            .zip(b_parts.iter())
            .take_while(|(ap, bp)| ap == bp)
            .count();

        // Overlap if one is a prefix of the other (meaning one contains the other)
        if (common == a_parts.len() && b_parts.len() > a_parts.len()) ||
           (common == b_parts.len() && a_parts.len() > b_parts.len()) {
            return true;
        }

        false
    }
}

#[cfg(test)]
mod merge_planner_tests {
    use super::*;
    use crate::domain::change_control::{AgentTaskStatus, ChangeScope, RiskLevel};
    use crate::ports::inbound::change_control_port::ChangeControlPort;

    fn mock_scope(write: Vec<&str>, blocked: Vec<&str>) -> ChangeScope {
        ChangeScope {
            allowed_write: write.into_iter().map(String::from).collect(),
            read_only: vec![],
            blocked: blocked.into_iter().map(String::from).collect(),
            contracts_affected: vec![],
            layers_affected: vec![],
        }
    }

    fn mock_task(id: &str, title: &str, risk: RiskLevel, created_at: i64, scope: ChangeScope, deps: Vec<&str>) -> AgentTask {
        AgentTask {
            id: id.to_string(),
            title: title.to_string(),
            capability: "test".to_string(),
            agent_id: "agent-1".to_string(),
            status: AgentTaskStatus::Draft,
            intent: "test".to_string(),
            scope,
            risk_level: risk,
            dependencies: deps.into_iter().map(String::from).collect(),
            memory_refs: vec![],
            created_at,
            updated_at: created_at,
        }
    }

    #[tokio::test]
    async fn test_plan_merge_complex() {
        let service = ChangeControlService::new();

        // 1. Task A: High Risk, no deps, created at 100
        let task_a = mock_task("task-a", "High Risk A", RiskLevel::High, 100,
            mock_scope(vec!["src/core/"], vec![]), vec![]);
        service.create_task(task_a).await.unwrap();

        // 2. Task B: Low Risk, no deps, created at 110, conflicts with A
        let task_b = mock_task("task-b", "Low Risk B (conflicts A)", RiskLevel::Low, 110,
            mock_scope(vec!["src/core/utils.rs"], vec![]), vec![]);
        service.create_task(task_b).await.unwrap();

        // 3. Task C: Low Risk, no deps, created at 120, no conflicts
        let task_c = mock_task("task-c", "Low Risk C (safe)", RiskLevel::Low, 120,
            mock_scope(vec!["src/ui/"], vec![]), vec![]);
        service.create_task(task_c).await.unwrap();

        // 4. Task D: Medium Risk, deps on A, created at 130
        let task_d = mock_task("task-d", "Medium Risk D (deps A)", RiskLevel::Medium, 130,
            mock_scope(vec!["docs/"], vec![]), vec!["task-a"]);
        service.create_task(task_d).await.unwrap();

        // 5. Task E: Critical Risk, no deps, created at 140, conflicts with C
        let task_e = mock_task("task-e", "Critical Risk E (conflicts C)", RiskLevel::Critical, 140,
            mock_scope(vec!["src/ui/theme.rs"], vec![]), vec![]);
        service.create_task(task_e).await.unwrap();

        let plan = service.plan_merge().await.unwrap();

        // EXPECTATIONS:
        // Priority order: E (Critical), A (High), B (Low, 110), C (Low, 120), D (Medium, 130)
        // 1. task-e (Critical) -> Ready. Scheduled as sequential.
        // 2. task-a (High) -> Ready. Scheduled as sequential.
        // 3. task-b (Low, 110) -> Conflicts with task-a (src/core/). -> Blocked by task-a.
        // 4. task-c (Low, 120) -> Conflicts with task-e (src/ui/). -> Blocked by task-e.
        // 5. task-d (Medium, 130) -> Unmet dependency on task-a. -> Blocked by task-a.

        assert!(plan.sequential.contains(&"task-e".to_string()));
        assert!(plan.sequential.contains(&"task-a".to_string()));
        assert_eq!(plan.sequential.len(), 2);

        assert!(plan.safe_parallel_groups.is_empty());

        let blocked_ids: Vec<String> = plan.blocked.iter().map(|b| b.task.clone()).collect();
        assert!(blocked_ids.contains(&"task-b".to_string()));
        assert!(blocked_ids.contains(&"task-c".to_string()));
        assert!(blocked_ids.contains(&"task-d".to_string()));
        assert_eq!(plan.blocked.len(), 3);

        // Verify reasons
        let b_task = plan.blocked.iter().find(|b| b.task == "task-b").unwrap();
        assert!(b_task.reason.contains("conflicts with task-a"));
        assert_eq!(b_task.blocked_by, vec!["task-a".to_string()]);

        let c_task = plan.blocked.iter().find(|b| b.task == "task-c").unwrap();
        assert!(c_task.reason.contains("conflicts with task-e"));
        assert_eq!(c_task.blocked_by, vec!["task-e".to_string()]);

        let d_task = plan.blocked.iter().find(|b| b.task == "task-d").unwrap();
        assert!(d_task.reason.contains("Unmet dependencies: task-a"));
        assert_eq!(d_task.blocked_by, vec!["task-a".to_string()]);
    }

    #[tokio::test]
    async fn test_plan_merge_parallel() {
        let service = ChangeControlService::new();

        // task-1: Low Risk, created at 100
        service.create_task(mock_task("task-1", "P1", RiskLevel::Low, 100,
            mock_scope(vec!["src/a.rs"], vec![]), vec![])).await.unwrap();

        // task-2: Low Risk, created at 110, conflicts with task-1
        service.create_task(mock_task("task-2", "P2", RiskLevel::Low, 110,
            mock_scope(vec!["src/a.rs"], vec![]), vec![])).await.unwrap();

        // task-3: Low Risk, created at 120, no conflicts
        service.create_task(mock_task("task-3", "P3", RiskLevel::Low, 120,
            mock_scope(vec!["src/b.rs"], vec![]), vec![])).await.unwrap();

        let plan = service.plan_merge().await.unwrap();

        // Expectation:
        // task-1 and task-3 are safe to run. task-2 conflicts with task-1.
        // BUT task-1 and task-3 don't conflict with each other.
        // In my current implementation, task-2 is blocked by task-1 because task-1 has higher priority (earlier created_at).
        // task-1 and task-3 should be in a parallel group.

        assert_eq!(plan.safe_parallel_groups.len(), 1);
        assert!(plan.safe_parallel_groups[0].contains(&"task-1".to_string()));
        assert!(plan.safe_parallel_groups[0].contains(&"task-3".to_string()));
        assert_eq!(plan.safe_parallel_groups[0].len(), 2);

        assert_eq!(plan.blocked.len(), 1);
        assert_eq!(plan.blocked[0].task, "task-2");
        assert_eq!(plan.blocked[0].blocked_by, vec!["task-1".to_string()]);
    }
}
