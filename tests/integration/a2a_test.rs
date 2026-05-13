//! A2A Protocol Tests

#[cfg(test)]
mod a2a_message_tests {
    use xavier::a2a::{A2AMessage, MessageType};

    #[test]
    fn test_a2a_message_creation() {
        let msg = A2AMessage::new(
            "sender".to_string(),
            "receiver".to_string(),
            MessageType::Request,
            "test data".to_string(),
        );

        assert_eq!(msg.sender, "sender");
        assert_eq!(msg.receiver, "receiver");
    }

    #[test]
    fn test_message_serialization() {
        let msg = A2AMessage::new(
            "a".to_string(),
            "b".to_string(),
            MessageType::Response,
            "data".to_string(),
        );

        let json = serde_json::to_string(&msg).expect("test assertion");
        assert!(json.contains("a"));
        assert!(json.contains("b"));
    }

    #[test]
    fn test_message_deserialization() {
        let json = r#"{
            "sender": "agent1",
            "receiver": "agent2",
            "message_type": "Request",
            "data": "test",
            "id": "msg123"
        }"#;

        let msg: A2AMessage = serde_json::from_str(json).expect("test assertion");
        assert_eq!(msg.sender, "agent1");
        assert_eq!(msg.receiver, "agent2");
    }
}

#[cfg(test)]
mod a2a_protocol_tests {
    use xavier::a2a::A2AProtocol;

    #[test]
    fn test_protocol_creation() {
        let protocol = A2AProtocol::new();
        assert!(protocol.is_valid());
    }

    #[tokio::test]
    async fn test_validate_message() {
        let protocol = A2AProtocol::new();

        // Valid message
        let valid = protocol.validate_message("valid payload");
        assert!(valid.is_ok());
    }

    #[tokio::test]
    async fn test_handle_request() {
        let protocol = A2AProtocol::new();

        let result = protocol
            .handle_request("sender".to_string(), "test request".to_string())
            .await;

        assert!(result.is_ok());
    }
}
