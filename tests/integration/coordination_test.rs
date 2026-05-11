//! Coordination Module Tests

#[cfg(test)]
mod coordination_tests {
    use xavier::coordination::{CoordinationMessage, Coordinator, Event};

    #[test]
    fn test_coordinator_creation() {
        let coordinator = Coordinator::new();
        assert!(coordinator.is_idle());
    }

    #[test]
    fn test_coordination_message() {
        let msg =
            CoordinationMessage::new("from".to_string(), "to".to_string(), Event::TaskAssigned);

        assert_eq!(msg.from, "from");
        assert!(matches!(msg.event, Event::TaskAssigned));
    }

    #[tokio::test]
    async fn test_broadcast_event() {
        let mut coordinator = Coordinator::new();

        coordinator.subscribe("agent1".to_string()).await;
        coordinator.subscribe("agent2".to_string()).await;

        coordinator.broadcast(Event::SystemShutdown).await;

        // Both agents should receive the event
        let events1 = coordinator.get_events("agent1").await;
        let events2 = coordinator.get_events("agent2").await;

        assert!(!events1.is_empty());
        assert!(!events2.is_empty());
    }

    #[tokio::test]
    async fn test_send_to_specific() {
        let mut coordinator = Coordinator::new();

        coordinator.subscribe("target".to_string()).await;

        coordinator
            .send_to(
                "sender".to_string(),
                "target".to_string(),
                Event::TaskCompleted,
            )
            .await;

        let events = coordinator.get_events("target").await;
        assert!(!events.is_empty());
    }

    #[tokio::test]
    async fn test_unsubscribe() {
        let mut coordinator = Coordinator::new();

        coordinator.subscribe("to_remove".to_string()).await;
        coordinator.unsubscribe("to_remove").await;

        coordinator.broadcast(Event::Test).await;

        let events = coordinator.get_events("to_remove").await;
        assert!(events.is_empty());
    }
}

#[cfg(test)]
mod distributed_lock_tests {
    use xavier::coordination::DistributedLock;

    #[tokio::test]
    async fn test_lock_acquisition() {
        let lock = DistributedLock::new("resource_1".to_string());

        let acquired = lock.try_acquire("agent1").await;
        assert!(acquired);

        // Second attempt should fail
        let acquired_again = lock.try_acquire("agent2").await;
        assert!(!acquired_again);
    }

    #[tokio::test]
    async fn test_lock_release() {
        let lock = DistributedLock::new("resource".to_string());

        lock.try_acquire("owner").await;
        lock.release("owner").await;

        // Should be available now
        let acquired = lock.try_acquire("new_owner").await;
        assert!(acquired);
    }

    #[tokio::test]
    #[ignore = "timeout behaviour not implemented in the lightweight compatibility lock"]
    async fn test_lock_timeout() {
        let lock = DistributedLock::new("timeout_test".to_string());

        lock.try_acquire("owner").await;

        // After timeout, should be available
        // Implementation depends on timeout configuration
        todo!("Test with actual timeout");
    }
}

#[cfg(test)]
mod coordination_service_impact_tests {
    use std::sync::Arc;
    use xavier::coordination::{CoordinationService, MessageBus};
    use xavier::security::ChangeControlService;
    use code_graph::db::CodeGraphDB;
    use code_graph::query::QueryEngine;
    use code_graph::types::{Symbol, SymbolKind, Language};

    #[tokio::test]
    async fn test_claim_lease_with_low_impact() {
        let bus = MessageBus::new();
        let db = CodeGraphDB::in_memory().unwrap();
        let engine = Arc::new(QueryEngine::new(Arc::new(db)));
        let change_control = Arc::new(ChangeControlService::new(engine));
        let service = CoordinationService::with_change_control(bus, change_control);

        let result = service.claim_lease("resource1", "agent1", &["src/utils.rs".to_string()]).await;

        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_claim_lease_blocks_critical_impact() {
        let bus = MessageBus::new();
        let db = Arc::new(CodeGraphDB::in_memory().unwrap());

        // Mock a critical scenario: many symbols affected in a critical file
        for i in 0..20 {
            db.insert_symbol(&Symbol {
                id: None,
                name: format!("func_{}", i),
                kind: SymbolKind::Function,
                lang: Language::Rust,
                file_path: "src/lib.rs".to_string(),
                start_line: i * 10,
                end_line: i * 10 + 5,
                start_col: 0,
                end_col: 10,
                signature: None,
                parent: None,
            }).unwrap();
        }

        let engine = Arc::new(QueryEngine::new(db));
        let change_control = Arc::new(ChangeControlService::new(engine));
        let service = CoordinationService::with_change_control(bus, change_control);

        let result = service.claim_lease("resource1", "agent1", &["src/lib.rs".to_string()]).await;

        assert!(result.is_ok());
        // Should be blocked because score will be > 0.8
        // 20 symbols * 0.05 = 1.0
        // + 0.2 for critical file = 1.2 -> 1.0
        assert!(!result.unwrap());
    }
}
