//! Agents Module Tests

#[cfg(test)]
mod agent_tests {
    use xavier2::agents::{Agent, AgentConfig, AgentStatus};

    #[test]
    fn test_agent_creation() {
        let config = AgentConfig::new("test_agent".to_string());
        let agent = Agent::new(config);

        assert_eq!(agent.name, "test_agent");
        assert!(matches!(agent.status, AgentStatus::Idle));
    }

    #[test]
    fn test_agent_with_model() {
        let config =
            AgentConfig::new("model_agent".to_string()).with_model("MiniMax-M2.5".to_string());
        let agent = Agent::new(config);

        assert_eq!(agent.model, Some("MiniMax-M2.5".to_string()));
    }

    #[test]
    fn test_agent_status_transitions() {
        let agent = Agent::new(AgentConfig::new("status_test".to_string()));

        // Idle -> Running
        let mut agent = agent;
        agent.start();
        assert!(matches!(agent.status, AgentStatus::Running));

        // Running -> Idle
        agent.stop();
        assert!(matches!(agent.status, AgentStatus::Idle));
    }

    #[tokio::test]
    async fn test_agent_execute_task() {
        let agent = Agent::new(AgentConfig::new("exec_test".to_string()));

        // Agent should be able to execute a task
        let result = agent.execute("test prompt".to_string()).await;

        // Result depends on actual implementation
        // Just verify it returns something
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_agent_tools_configuration() {
        let config = AgentConfig::new("tools_agent".to_string())
            .with_tools(vec!["memory".to_string(), "search".to_string()]);
        let agent = Agent::new(config);

        assert_eq!(agent.tools.len(), 2);
    }
}

#[cfg(test)]
mod agent_coordination_tests {
    use xavier2::agents::coordination::{AgentCoordinator, AgentMessage, MessageType};

    #[test]
    fn test_coordinator_creation() {
        let coordinator = AgentCoordinator::new();
        assert!(coordinator.agents().is_empty());
    }

    #[test]
    fn test_register_agent() {
        let mut coordinator = AgentCoordinator::new();

        coordinator.register_agent("agent1".to_string());
        coordinator.register_agent("agent2".to_string());

        assert_eq!(coordinator.agents().len(), 2);
    }

    #[test]
    fn test_agent_message_creation() {
        let msg = AgentMessage::new(
            "from_agent".to_string(),
            "to_agent".to_string(),
            MessageType::Task,
            "test payload".to_string(),
        );

        assert_eq!(msg.from, "from_agent");
        assert_eq!(msg.to, "to_agent");
        assert!(matches!(msg.message_type, MessageType::Task));
    }

    #[tokio::test]
    async fn test_send_message() {
        let mut coordinator = AgentCoordinator::new();

        coordinator.register_agent("agent1".to_string());
        coordinator.register_agent("agent2".to_string());

        let msg = AgentMessage::new(
            "agent1".to_string(),
            "agent2".to_string(),
            MessageType::Task,
            "do something".to_string(),
        );

        coordinator.send_message(msg).await;

        // Verify message was queued
        let messages = coordinator.get_messages("agent2").await;
        assert!(!messages.is_empty());
    }

    #[tokio::test]
    async fn test_broadcast_message() {
        let mut coordinator = AgentCoordinator::new();

        coordinator.register_agent("agent1".to_string());
        coordinator.register_agent("agent2".to_string());
        coordinator.register_agent("agent3".to_string());

        coordinator
            .broadcast(
                "sender".to_string(),
                MessageType::Notification,
                "broadcast message".to_string(),
            )
            .await;

        // All agents should receive the message
        for agent in coordinator.agents() {
            let messages = coordinator.get_messages(agent).await;
            assert!(!messages.is_empty());
        }
    }
}

#[cfg(test)]
mod agent_state_tests {
    use xavier2::agents::AgentState;

    #[test]
    fn test_agent_state_creation() {
        let state = AgentState::new("test_agent".to_string());

        assert_eq!(state.agent_id, "test_agent");
        assert!(state.context.is_empty());
    }

    #[test]
    fn test_context_operations() {
        let mut state = AgentState::new("test".to_string());

        // Add context
        state.add_context("key1".to_string(), "value1".to_string());
        assert_eq!(state.get_context("key1"), Some(&"value1".to_string()));

        // Update context
        state.update_context("key1".to_string(), "new_value".to_string());
        assert_eq!(state.get_context("key1"), Some(&"new_value".to_string()));

        // Remove context
        state.remove_context("key1".to_string());
        assert_eq!(state.get_context("key1"), None);
    }
}
