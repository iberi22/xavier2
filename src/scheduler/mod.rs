pub mod job;

pub use job::{RecoveryConfig, ScheduledJob};

use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::sync::Mutex;

const DEFAULT_SCHEDULER_STATE_PATH: &str = "scheduler/jobs.json";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct CronSchedule {
    expression: String,
}

impl CronSchedule {
    pub fn parse(expression: &str) -> std::result::Result<Self, cron::error::Error> {
        <Self as FromStr>::from_str(expression)
    }

    pub fn is_valid(&self) -> bool {
        cron::Schedule::from_str(&self.expression).is_ok()
    }

    pub fn next_run(&self) -> Option<chrono::DateTime<Utc>> {
        cron::Schedule::from_str(&self.expression)
            .ok()?
            .upcoming(Utc)
            .next()
    }
}

impl FromStr for CronSchedule {
    type Err = cron::error::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let normalized = normalize_cron_expression(s);
        cron::Schedule::from_str(&normalized)?;
        Ok(Self {
            expression: normalized,
        })
    }
}

fn normalize_cron_expression(expression: &str) -> String {
    let parts: Vec<_> = expression.split_whitespace().collect();
    if parts.len() == 5 {
        format!("0 {}", expression)
    } else {
        expression.to_string()
    }
}

#[derive(Debug, Clone)]
pub struct Job {
    pub id: String,
    pub name: String,
    pub command: String,
    pub schedule: CronSchedule,
    pub status: JobStatus,
}

impl Job {
    pub fn new(name: String, command: String, schedule: CronSchedule) -> Self {
        Self {
            id: ulid::Ulid::new().to_string(),
            name,
            command,
            schedule,
            status: JobStatus::Pending,
        }
    }

    pub fn run(&mut self) {
        self.status = JobStatus::Running;
    }

    pub fn complete(&mut self) {
        self.status = JobStatus::Completed;
    }

    pub fn cancel(&mut self) {
        self.status = JobStatus::Cancelled;
    }
}

#[derive(Default)]
pub struct Scheduler {
    jobs: Mutex<Vec<Job>>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.jobs
            .try_lock()
            .map(|jobs| jobs.is_empty())
            .unwrap_or(false)
    }

    pub fn len(&self) -> usize {
        self.jobs.try_lock().map(|jobs| jobs.len()).unwrap_or(0)
    }

    pub async fn add_job(&self, job: Job) {
        self.jobs.lock().await.push(job);
    }

    pub async fn remove_job(&self, job_id: &str) {
        self.jobs.lock().await.retain(|job| job.id != job_id);
    }

    pub async fn get_next_jobs(&self, limit: usize) -> Vec<Job> {
        self.jobs.lock().await.iter().take(limit).cloned().collect()
    }
}

#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    pub storage_path: PathBuf,
    pub recovery: RecoveryConfig,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            storage_path: PathBuf::from(DEFAULT_SCHEDULER_STATE_PATH),
            recovery: RecoveryConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SchedulerState {
    jobs: Vec<ScheduledJob>,
}

/// JobScheduler — manages scheduled job execution.
pub struct JobScheduler {
    jobs: Vec<ScheduledJob>,
    config: SchedulerConfig,
}

impl JobScheduler {
    pub fn new(config: SchedulerConfig) -> Self {
        Self {
            jobs: Vec::new(),
            config,
        }
    }

    pub async fn load(config: SchedulerConfig) -> Result<Self> {
        let state = load_state(&config.storage_path).await?;
        Ok(Self {
            jobs: state.jobs,
            config,
        })
    }

    pub async fn load_or_default(config: SchedulerConfig) -> Result<Self> {
        if fs::try_exists(&config.storage_path)
            .await
            .with_context(|| {
                format!(
                    "failed to check scheduler state file {}",
                    config.storage_path.display()
                )
            })?
        {
            Self::load(config).await
        } else {
            Ok(Self::new(config))
        }
    }

    pub fn jobs(&self) -> &[ScheduledJob] {
        &self.jobs
    }

    pub fn jobs_mut(&mut self) -> &mut [ScheduledJob] {
        &mut self.jobs
    }

    pub async fn add_job(&mut self, job: ScheduledJob) -> Result<()> {
        self.jobs.push(job);
        self.persist().await
    }

    pub async fn upsert_job(&mut self, job: ScheduledJob) -> Result<()> {
        match self.jobs.iter_mut().find(|existing| existing.id == job.id) {
            Some(existing) => *existing = job,
            None => self.jobs.push(job),
        }

        self.persist().await
    }

    pub async fn persist(&self) -> Result<()> {
        persist_state(&self.config.storage_path, &self.jobs).await
    }

    pub async fn detect_missed_jobs(&mut self) -> Result<usize> {
        let missed = job::detect_missed_jobs(
            &mut self.jobs,
            Utc::now(),
            self.config.recovery.missed_window,
        );
        self.persist().await?;
        Ok(missed)
    }

    pub async fn recover_missed_jobs<F, Fut>(&mut self, executor: F) -> Result<Vec<String>>
    where
        F: FnMut(ScheduledJob) -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        let recovered =
            job::recover_missed_jobs(&mut self.jobs, &self.config.recovery, executor).await?;
        self.persist().await?;
        Ok(recovered)
    }
}

async fn load_state(path: &Path) -> Result<SchedulerState> {
    let payload = fs::read_to_string(path)
        .await
        .with_context(|| format!("failed to read scheduler state {}", path.display()))?;

    serde_json::from_str(&payload)
        .with_context(|| format!("failed to deserialize scheduler state {}", path.display()))
}

async fn persist_state(path: &Path, jobs: &[ScheduledJob]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await.with_context(|| {
            format!(
                "failed to create scheduler state directory {}",
                parent.display()
            )
        })?;
    }

    let payload = serde_json::to_vec_pretty(&SchedulerState {
        jobs: jobs.to_vec(),
    })
    .context("failed to serialize scheduler state")?;

    fs::write(path, payload)
        .await
        .with_context(|| format!("failed to write scheduler state {}", path.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduler::job::ScheduledJobStatus;
    use chrono::Duration;

    async fn temp_scheduler_path() -> PathBuf {
        let root = std::env::temp_dir()
            .join("xavier-scheduler-tests")
            .join(ulid::Ulid::new().to_string());
        fs::create_dir_all(&root).await.expect("test assertion");
        root.join("scheduler").join("jobs.json")
    }

    #[tokio::test]
    async fn persists_jobs_to_json_file() {
        let storage_path = temp_scheduler_path().await;
        let mut scheduler = JobScheduler::new(SchedulerConfig {
            storage_path: storage_path.clone(),
            recovery: RecoveryConfig::default(),
        });

        scheduler
            .add_job(
                ScheduledJob::from_schedule("job-a", "index", "0/30 * * * * * *")
                    .expect("test assertion"),
            )
            .await
            .expect("test assertion");

        assert!(fs::try_exists(&storage_path).await.expect("test assertion"));

        let restored = JobScheduler::load(SchedulerConfig {
            storage_path,
            recovery: RecoveryConfig::default(),
        })
        .await
        .expect("test assertion");

        assert_eq!(restored.jobs().len(), 1);
        assert_eq!(restored.jobs()[0].id, "job-a");
    }

    #[tokio::test]
    async fn detects_and_recovers_missed_jobs_through_scheduler() {
        let storage_path = temp_scheduler_path().await;
        let mut scheduler = JobScheduler::new(SchedulerConfig {
            storage_path,
            recovery: RecoveryConfig {
                missed_window: Duration::minutes(5),
                max_per_restart: 2,
                stagger_ms: 0,
            },
        });

        scheduler
            .add_job(ScheduledJob {
                id: "job-a".to_string(),
                name: "job-a".to_string(),
                schedule: "0/15 * * * * * *".to_string(),
                last_run: None,
                next_run: Utc::now() - Duration::minutes(10),
                status: ScheduledJobStatus::Pending,
            })
            .await
            .expect("test assertion");

        let missed = scheduler
            .detect_missed_jobs()
            .await
            .expect("test assertion");
        assert_eq!(missed, 1);
        assert_eq!(scheduler.jobs()[0].status, ScheduledJobStatus::Missed);

        let recovered = scheduler
            .recover_missed_jobs(|_| async { Ok(()) })
            .await
            .expect("test assertion");

        assert_eq!(recovered, vec!["job-a".to_string()]);
        assert_eq!(scheduler.jobs()[0].status, ScheduledJobStatus::Completed);
        assert!(scheduler.jobs()[0].last_run.is_some());
    }
}
