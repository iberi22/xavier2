use std::sync::Arc;
use xavier::domain::change_control::*;
use xavier::app::change_control::ChangeControlService;
use xavier::coordination::{AgentRegistry, LeaseRegistry, MessageBus};
use xavier::memory::store::InMemoryMemoryStore;
use xavier::ports::inbound::ChangeControlPort;

#[tokio::test]
async fn test_change_control_workflow() {
    let bus = MessageBus::new();
    let agent_registry = AgentRegistry::new(bus);
    let lease_registry = Arc::new(LeaseRegistry::new());
    let memory_store = Arc::new(InMemoryMemoryStore::new());

    let service = ChangeControlService::new(
        memory_store,
        agent_registry.clone(),
        lease_registry,
    );

    let agent_id = "test-agent".to_string();
    agent_registry.register(
        &agent_id,
        "Test Agent",
        vec!["coding".to_string()],
        None,
        None,
    ).await.unwrap();

    // Test claim lease
    let request = LeaseRequest {
        agent_id: agent_id.clone(),
        resource_path: "src/main.rs".to_string(),
        duration_seconds: 60,
    };

    let response = service.claim_lease(request).await.unwrap();
    assert!(!response.lease_id.is_empty());

    // Test active leases
    let leases = service.active_leases().await.unwrap();
    assert_eq!(leases.len(), 1);
    assert_eq!(leases[0].resource_path, "src/main.rs");

    // Test conflict
    let request2 = LeaseRequest {
        agent_id: "other-agent".to_string(),
        resource_path: "src/main.rs".to_string(),
        duration_seconds: 60,
    };

    // Register the other agent too
    agent_registry.register(
        "other-agent",
        "Other Agent",
        vec![],
        None,
        None,
    ).await.unwrap();

    let result = service.claim_lease(request2).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("conflict"));

    // Test release lease
    service.release_lease(&response.lease_id).await.unwrap();
    let leases = service.active_leases().await.unwrap();
    assert_eq!(leases.len(), 0);
}
