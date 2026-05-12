use std::future::Future;
use std::str::FromStr;

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use cron::Schedule;
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Duration as TokioDuration};
use tracing::warn;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScheduledJobStatus {
    Pending,
    Running,
    Completed,
    Missed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledJob {
    pub id: String,
    pub name: String,
    pub schedule: String,
    pub last_run: Option<DateTime<Utc>>,
    pub next_run: DateTime<Utc>,
    pub status: ScheduledJobStatus,
}

impl ScheduledJob {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        schedule: impl Into<String>,
        next_run: DateTime<Utc>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            schedule: schedule.into(),
            last_run: None,
            next_run,
            status: ScheduledJobStatus::Pending,
        }
    }

    pub fn from_schedule(
        id: impl Into<String>,
        name: impl Into<String>,
        schedule: impl Into<String>,
    ) -> Result<Self> {
        let schedule = schedule.into();
        let next_run = compute_next_run(&schedule, Utc::now())?;
        Ok(Self::new(id, name, schedule, next_run))
    }
}

#[derive(Debug, Clone)]
pub struct RecoveryConfig {
    pub missed_window: Duration,
    pub max_per_restart: usize,
    pub stagger_ms: u64,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        Self {
            missed_window: Duration::minutes(5),
            max_per_restart: 5,
            stagger_ms: 500,
        }
    }
}

pub fn compute_next_run(schedule: &str, after: DateTime<Utc>) -> Result<DateTime<Utc>> {
    let parsed = Schedule::from_str(schedule)
        .with_context(|| format!("invalid cron expression: {schedule}"))?;

    parsed
        .after(&after)
        .next()
        .ok_or_else(|| anyhow::anyhow!("no future run found for cron expression"))
}

pub fn detect_missed_jobs(
    jobs: &mut [ScheduledJob],
    now: DateTime<Utc>,
    missed_window: Duration,
) -> usize {
    let mut missed = 0;

    for job in jobs.iter_mut() {
        let is_late = now.signed_duration_since(job.next_run) > missed_window;
        if is_late && job.status != ScheduledJobStatus::Running {
            if job.status != ScheduledJobStatus::Missed {
                missed += 1;
            }
            job.status = ScheduledJobStatus::Missed;
        }
    }

    missed
}

pub async fn recover_missed_jobs<F, Fut>(
    jobs: &mut [ScheduledJob],
    config: &RecoveryConfig,
    mut executor: F,
) -> Result<Vec<String>>
where
    F: FnMut(ScheduledJob) -> Fut,
    Fut: Future<Output = Result<()>>,
{
    let mut missed_indexes = jobs
        .iter()
        .enumerate()
        .filter(|(_, job)| job.status == ScheduledJobStatus::Missed)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();

    missed_indexes.sort_by_key(|index| jobs[*index].next_run);

    let mut recovered = Vec::new();

    for (position, index) in missed_indexes
        .into_iter()
        .take(config.max_per_restart)
        .enumerate()
    {
        if position > 0 && config.stagger_ms > 0 {
            sleep(TokioDuration::from_millis(config.stagger_ms)).await;
        }

        let job_snapshot = {
            let job = &mut jobs[index];
            job.status = ScheduledJobStatus::Running;
            job.clone()
        };

        match executor(job_snapshot.clone()).await {
            Ok(()) => {
                let completed_at = Utc::now();
                let job = &mut jobs[index];
                job.last_run = Some(completed_at);
                job.next_run = compute_next_run(&job.schedule, completed_at)?;
                job.status = ScheduledJobStatus::Completed;
                recovered.push(job.id.clone());
            }
            Err(error) => {
                let job = &mut jobs[index];
                job.status = ScheduledJobStatus::Missed;
                warn!(job_id = %job.id, error = %error, "failed to recover missed job");
            }
        }
    }

    Ok(recovered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    #[test]
    fn flags_jobs_outside_missed_window() {
        let now = Utc::now();
        let mut jobs = vec![
            ScheduledJob {
                id: "missed".to_string(),
                name: "missed".to_string(),
                schedule: "0/5 * * * * * *".to_string(),
                last_run: None,
                next_run: now - Duration::minutes(10),
                status: ScheduledJobStatus::Pending,
            },
            ScheduledJob {
                id: "fresh".to_string(),
                name: "fresh".to_string(),
                schedule: "0/5 * * * * * *".to_string(),
                last_run: None,
                next_run: now - Duration::minutes(1),
                status: ScheduledJobStatus::Pending,
            },
        ];

        let missed = detect_missed_jobs(&mut jobs, now, Duration::minutes(5));

        assert_eq!(missed, 1);
        assert_eq!(jobs[0].status, ScheduledJobStatus::Missed);
        assert_eq!(jobs[1].status, ScheduledJobStatus::Pending);
    }

    #[tokio::test]
    async fn recovers_only_configured_number_of_missed_jobs() {
        let now = Utc::now();
        let mut jobs = vec![
            ScheduledJob {
                id: "job-1".to_string(),
                name: "job-1".to_string(),
                schedule: "0/10 * * * * * *".to_string(),
                last_run: None,
                next_run: now - Duration::minutes(10),
                status: ScheduledJobStatus::Missed,
            },
            ScheduledJob {
                id: "job-2".to_string(),
                name: "job-2".to_string(),
                schedule: "0/10 * * * * * *".to_string(),
                last_run: None,
                next_run: now - Duration::minutes(9),
                status: ScheduledJobStatus::Missed,
            },
        ];
        let runs = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&runs);

        let recovered = recover_missed_jobs(
            &mut jobs,
            &RecoveryConfig {
                missed_window: Duration::minutes(5),
                max_per_restart: 1,
                stagger_ms: 0,
            },
            move |_| {
                let counter = Arc::clone(&counter);
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                }
            },
        )
        .await
        .expect("test assertion");

        assert_eq!(runs.load(Ordering::SeqCst), 1);
        assert_eq!(recovered, vec!["job-1".to_string()]);
        assert_eq!(jobs[0].status, ScheduledJobStatus::Completed);
        assert_eq!(jobs[1].status, ScheduledJobStatus::Missed);
        assert!(jobs[0].last_run.is_some());
        assert!(jobs[0].next_run > now);
    }
}
