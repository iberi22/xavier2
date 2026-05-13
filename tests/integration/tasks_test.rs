//! Tasks Module Tests

#[cfg(test)]
mod task_tests {
    use xavier::tasks::{Task, TaskPriority, TaskStatus};

    #[test]
    fn test_task_creation() {
        let task = Task::new("test_task".to_string(), "do something".to_string());

        assert_eq!(task.name, "test_task");
        assert_eq!(task.description, "do something");
        assert!(matches!(task.status, TaskStatus::Pending));
    }

    #[test]
    fn test_task_with_priority() {
        let task = Task::new("high_priority".to_string(), "important task".to_string())
            .with_priority(TaskPriority::High);

        assert!(matches!(task.priority, TaskPriority::High));
    }

    #[test]
    fn test_task_status_transitions() {
        let mut task = Task::new("test".to_string(), "desc".to_string());

        // Pending -> InProgress
        task.start();
        assert!(matches!(task.status, TaskStatus::InProgress));

        // InProgress -> Completed
        task.complete();
        assert!(matches!(task.status, TaskStatus::Completed));
    }

    #[test]
    fn test_task_failure() {
        let mut task = Task::new("test".to_string(), "desc".to_string());

        task.start();
        task.fail("test error".to_string());

        assert!(matches!(task.status, TaskStatus::Failed));
        assert_eq!(task.error_message, Some("test error".to_string()));
    }

    #[test]
    fn test_task_cancellation() {
        let mut task = Task::new("test".to_string(), "desc".to_string());

        task.cancel();
        assert!(matches!(task.status, TaskStatus::Cancelled));
    }

    #[tokio::test]
    async fn test_task_execution() {
        let task = Task::new("exec_test".to_string(), "description".to_string());

        // Task should be executable
        assert!(task.can_execute());
    }
}

#[cfg(test)]
mod task_queue_tests {
    use xavier::tasks::{Task, TaskPriority, TaskQueue};

    #[tokio::test]
    async fn test_task_queue_creation() {
        let queue = TaskQueue::new();
        assert!(queue.is_empty());
    }

    #[tokio::test]
    async fn test_enqueue_task() {
        let queue = TaskQueue::new();
        let task = Task::new("queued".to_string(), "desc".to_string());

        queue.enqueue(task).await;
        assert_eq!(queue.len(), 1);
    }

    #[tokio::test]
    async fn test_dequeue_task() {
        let queue = TaskQueue::new();
        let task = Task::new("dequeue_test".to_string(), "desc".to_string());

        queue.enqueue(task).await;
        let dequeued = queue.dequeue().await;

        assert!(dequeued.is_some());
        assert!(queue.is_empty());
    }

    #[tokio::test]
    async fn test_priority_ordering() {
        let queue = TaskQueue::new();

        // Add tasks with different priorities
        let low = Task::new("low".to_string(), "desc".to_string()).with_priority(TaskPriority::Low);
        let high =
            Task::new("high".to_string(), "desc".to_string()).with_priority(TaskPriority::High);
        let medium =
            Task::new("medium".to_string(), "desc".to_string()).with_priority(TaskPriority::Medium);

        queue.enqueue(low).await;
        queue.enqueue(high).await;
        queue.enqueue(medium).await;

        // First should be high priority
        let first = queue.dequeue().await.expect("test assertion");
        assert!(matches!(first.priority, TaskPriority::High));
    }

    #[tokio::test]
    async fn test_task_retry() {
        let queue = TaskQueue::new();
        let task = Task::new("retry_test".to_string(), "desc".to_string());

        queue.enqueue(task).await;

        // Simulate failure and retry
        let mut task = queue.dequeue().await.expect("test assertion");
        task.fail("temp error".to_string());

        queue.enqueue(task).await;
        assert_eq!(queue.len(), 1);
    }
}
