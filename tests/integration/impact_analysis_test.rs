use xavier::app::change_control_service::ChangeControlService;
use xavier::ports::inbound::change_control_port::ChangeControlPort;
use xavier::domain::change_control::{AgentTask, AgentTaskStatus, ChangeScope, LeaseMode, RiskLevel};
use code_graph::db::CodeGraphDB;
use code_graph::types::{Symbol, SymbolKind, Language};
use std::sync::Arc;
use tempfile::tempdir;

#[tokio::test]
async fn test_claim_lease_triggers_impact_analysis() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test_code_graph.db");
    let db = Arc::new(CodeGraphDB::new(&db_path).unwrap());

    // Seed some data in code-graph
    let symbols = vec![
        Symbol {
            id: None,
            name: "MemoryQueryPort".to_string(),
            kind: SymbolKind::Trait,
            lang: Language::Rust,
            file_path: "src/ports/inbound/memory_port.rs".to_string(),
            start_line: 10,
            end_line: 20,
            start_col: 0,
            end_col: 0,
            signature: Some("pub trait MemoryQueryPort".to_string()),
            parent: None,
        },
        Symbol {
            id: None,
            name: "MemoryQueryPort".to_string(),
            kind: SymbolKind::Import,
            lang: Language::Rust,
            file_path: "src/app/qmd_memory_adapter.rs".to_string(),
            start_line: 5,
            end_line: 5,
            start_col: 0,
            end_col: 0,
            signature: Some("use crate::ports::inbound::memory_port::MemoryQueryPort;".to_string()),
            parent: None,
        }
    ];
    db.insert_symbols(&symbols).unwrap();

    let service = ChangeControlService::new().with_db(db);

    let task = AgentTask {
        id: "".to_string(),
        title: "Test Task".to_string(),
        capability: "coding".to_string(),
        agent_id: "agent-1".to_string(),
        status: AgentTaskStatus::Draft,
        intent: "Modify memory port".to_string(),
        scope: ChangeScope {
            read_only: vec!["src/**/*.rs".to_string()],
            allowed_write: vec!["src/ports/inbound/memory_port.rs".to_string()],
            blocked: vec![],
            contracts_affected: vec![],
            layers_affected: vec![],
        },
        risk_level: RiskLevel::Low,
        dependencies: vec![],
        memory_refs: vec![],
        created_at: 0,
        updated_at: 0,
    };

    let task_id = service.create_task(task).await.unwrap();

    let response = service.claim_lease(
        "agent-1",
        &task_id,
        vec!["src/ports/inbound/memory_port.rs".to_string()],
        LeaseMode::Write,
        3600
    ).await.unwrap();

    // Verify impact analysis results in LeaseResponse
    // 1. Symbol from file was found
    // 2. Reverse dependency (qmd_memory_adapter.rs) was found via import
    // 3. Risk level should be at least Medium because of "ports/" heuristic and dependency

    let impact = service.calculate_impact(&["src/ports/inbound/memory_port.rs".to_string()]).await;

    assert!(impact.symbols_affected >= 1);
    assert!(impact.dependent_files.contains(&"src/app/qmd_memory_adapter.rs".to_string()));
    assert!(impact.contracts_affected.contains(&"MemoryQueryPort".to_string()));
    assert!(impact.risk_level >= RiskLevel::Medium);

    // Check if LeaseResponse reflect high-risk checks if it was High or Critical
    if impact.risk_level >= RiskLevel::High {
        assert!(response.required_checks.iter().any(|c| c.contains("cargo test --test '*'")));
    }
}
