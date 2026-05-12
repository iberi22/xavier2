//! Scheduler Module Tests

#[cfg(test)]
mod scheduler_tests {
    use xavier::scheduler::{CronSchedule, Job, JobStatus, Scheduler};

    #[test]
    fn test_scheduler_creation() {
        let scheduler = Scheduler::new();
        assert!(scheduler.is_empty());
    }

    #[test]
    fn test_job_creation() {
        let job = Job::new(
            "test_job".to_string(),
            "test command".to_string(),
            CronSchedule::parse("0 * * * *").expect("test assertion"), // Every hour
        );

        assert_eq!(job.name, "test_job");
        assert_eq!(job.command, "test command");
        assert!(matches!(job.status, JobStatus::Pending));
    }

    #[test]
    fn test_job_status_transitions() {
        let mut job = Job::new(
            "test".to_string(),
            "cmd".to_string(),
            CronSchedule::parse("0 * * * *").expect("test assertion"),
        );

        // Pending -> Running
        job.run();
        assert!(matches!(job.status, JobStatus::Running));

        // Running -> Completed
        job.complete();
        assert!(matches!(job.status, JobStatus::Completed));
    }

    #[test]
    fn test_job_cancellation() {
        let mut job = Job::new(
            "test".to_string(),
            "cmd".to_string(),
            CronSchedule::parse("0 * * * *").expect("test assertion"),
        );

        job.cancel();
        assert!(matches!(job.status, JobStatus::Cancelled));
    }

    #[tokio::test]
    async fn test_scheduler_add_job() {
        let scheduler = Scheduler::new();

        let job = Job::new(
            "scheduled_job".to_string(),
            "echo hello".to_string(),
            CronSchedule::parse("0 * * * *").expect("test assertion"),
        );

        scheduler.add_job(job).await;
        assert_eq!(scheduler.len(), 1);
    }

    #[tokio::test]
    async fn test_scheduler_remove_job() {
        let scheduler = Scheduler::new();

        let job = Job::new(
            "to_remove".to_string(),
            "cmd".to_string(),
            CronSchedule::parse("0 * * * *").expect("test assertion"),
        );

        let job_id = job.id.clone();
        scheduler.add_job(job).await;
        scheduler.remove_job(&job_id).await;

        assert!(scheduler.is_empty());
    }

    #[tokio::test]
    async fn test_scheduler_get_next_jobs() {
        let scheduler = Scheduler::new();

        // Add job that should run soon
        let job = Job::new(
            "next_job".to_string(),
            "cmd".to_string(),
            CronSchedule::parse("* * * * *").expect("test assertion"), // Every minute
        );

        scheduler.add_job(job).await;

        let next = scheduler.get_next_jobs(1).await;
        assert!(!next.is_empty());
    }
}

#[cfg(test)]
mod cron_schedule_tests {
    use xavier::scheduler::CronSchedule;

    #[test]
    fn test_cron_parsing() {
        let schedule = CronSchedule::parse("0 * * * *").expect("test assertion");
        assert!(schedule.is_valid());
    }

    #[test]
    fn test_cron_every_minute() {
        let schedule = CronSchedule::parse("* * * * *").expect("test assertion");
        assert!(schedule.is_valid());
    }

    #[test]
    fn test_cron_invalid() {
        let result = CronSchedule::parse("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_cron_next_run() {
        let schedule = CronSchedule::parse("0 0 * * *").expect("test assertion"); // Daily at midnight
        let next = schedule.next_run();
        assert!(next.is_some());
    }
}
