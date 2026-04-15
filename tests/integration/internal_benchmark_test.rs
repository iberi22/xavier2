use std::sync::Arc;

use tokio::sync::RwLock;
use xavier2::agents::{runtime::System3Mode, AgentRuntime, RuntimeConfig};
use xavier2::memory::{
    qmd_memory::QmdMemory,
    schema::{
        EvidenceKind, MemoryNamespace, MemoryProvenance, MemoryQueryFilters, TypedMemoryPayload,
    },
};

#[tokio::test]
async fn internal_benchmark_smoke_covers_filters_and_optional_system3() {
    let memory = Arc::new(QmdMemory::new_with_workspace(
        Arc::new(RwLock::new(Vec::new())),
        "ws-bench",
    ));

    memory
        .add_document_typed(
            "decision/system3".to_string(),
            "Decision: keep System3 optional and answer factual queries directly from typed evidence."
                .to_string(),
            serde_json::json!({}),
            Some(TypedMemoryPayload {
                kind: Some(xavier2::memory::schema::MemoryKind::Decision),
                evidence_kind: Some(EvidenceKind::Observation),
                namespace: Some(MemoryNamespace {
                    project: Some("xavier2".to_string()),
                    session_id: Some("session-wave-92".to_string()),
                    ..MemoryNamespace::default()
                }),
                provenance: Some(MemoryProvenance {
                    source_app: Some("engram".to_string()),
                    source_type: Some("observation".to_string()),
                    ..MemoryProvenance::default()
                }),
            }),
        )
        .await
        .unwrap();

    let filtered = memory
        .search_filtered(
            "System3 optional",
            5,
            Some(&MemoryQueryFilters {
                session_id: Some("session-wave-92".to_string()),
                project: Some("xavier2".to_string()),
                ..MemoryQueryFilters::default()
            }),
        )
        .await
        .unwrap();
    assert_eq!(filtered.len(), 1);

    let runtime = AgentRuntime::new(Arc::clone(&memory), None, RuntimeConfig::default()).unwrap();
    let trace = runtime
        .run_with_trace_filtered(
            "What was the decision about System3?",
            None,
            Some("1".to_string()),
            Some(MemoryQueryFilters {
                project: Some("xavier2".to_string()),
                session_id: Some("session-wave-92".to_string()),
                ..MemoryQueryFilters::default()
            }),
            System3Mode::Disabled,
        )
        .await
        .unwrap();

    assert!(trace.agent.response.to_lowercase().contains("system3"));
    assert!(!trace.action.llm_used);
}
