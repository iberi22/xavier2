use std::sync::Arc;
use xavier::domain::change_control::{ChangePolicy, ValidationStatus};
use xavier::app::change_control_service::ChangeControlService;
use xavier::coordination::DistributedLock;

#[tokio::test]
async fn test_policy_load_and_validation() {
    let yaml = r#"
layers:
  domain:
    path: "src/domain/**"
    risk: high
    may_import: []
  adapters:
    path: "src/adapters/**"
    risk: medium
    may_import: [ports]
critical_files:
  - "src/memory/qmd_memory.rs"
rules:
  - id: test_rule
    description: "test"
    severity: error
"#;
    let policy: ChangePolicy = serde_yaml::from_str(yaml).unwrap();

    // Test critical files
    assert!(policy.is_critical("src/memory/qmd_memory.rs"));
    assert!(!policy.is_critical("src/other.rs"));

    // Test layer detection
    let layer = policy.get_layer_for_path("src/domain/mod.rs");
    assert!(layer.is_some());
    assert_eq!(layer.unwrap().0, "domain");

    // Test import validation
    let status = policy.validate_import("src/domain/mod.rs", "src/adapters/mod.rs");
    match status {
        ValidationStatus::Rejected(reason) => assert!(reason.contains("is not allowed to import")),
        _ => panic!("Expected rejection"),
    }
}

#[tokio::test]
async fn test_distributed_lock_with_policy() {
    let yaml = r#"
layers:
  domain:
    path: "src/domain/**"
    risk: high
    may_import: []
critical_files:
  - "src/memory/qmd_memory.rs"
rules: []
"#;
    let policy: ChangePolicy = serde_yaml::from_str(yaml).unwrap();
    let service = Arc::new(ChangeControlService::new(policy));

    // Lock on critical file should fail (ApprovalRequired)
    let lock_critical = DistributedLock::with_policy("src/memory/qmd_memory.rs".to_string(), service.clone());
    assert!(!lock_critical.try_acquire("agent1").await);

    // Lock on high-risk layer should fail (ApprovalRequired)
    let lock_high_risk = DistributedLock::with_policy("src/domain/agent.rs".to_string(), service.clone());
    assert!(!lock_high_risk.try_acquire("agent1").await);

    // Lock on normal file should succeed
    let lock_normal = DistributedLock::with_policy("src/utils/mod.rs".to_string(), service.clone());
    assert!(lock_normal.try_acquire("agent1").await);
}
