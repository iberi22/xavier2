use std::sync::Arc;

use tokio::sync::RwLock;
use xavier::agents::{runtime::System3Mode, AgentRuntime, RuntimeConfig};
use xavier::memory::{
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
                kind: Some(xavier::memory::schema::MemoryKind::Decision),
                evidence_kind: Some(EvidenceKind::Observation),
                namespace: Some(MemoryNamespace {
                    project: Some("xavier".to_string()),
                    session_id: Some("session-wave-92".to_string()),
                    ..MemoryNamespace::default()
                }),
                provenance: Some(MemoryProvenance {
                    source_app: Some("engram".to_string()),
                    source_type: Some("observation".to_string()),
                    ..MemoryProvenance::default()
                }),
                ..Default::default()
            }),
        )
        .await
        .expect("test assertion");

    let filtered = memory
        .search_filtered(
            "System3 optional",
            5,
            Some(&MemoryQueryFilters {
                session_id: Some("session-wave-92".to_string()),
                project: Some("xavier".to_string()),
                ..MemoryQueryFilters::default()
            }),
        )
        .await
        .expect("test assertion");
    assert_eq!(filtered.len(), 1);

    let runtime = AgentRuntime::new(Arc::clone(&memory), None, RuntimeConfig::default())
        .expect("test assertion");
    let trace = runtime
        .run_with_trace_filtered(
            "What was the decision about System3?",
            None,
            Some("1".to_string()),
            Some(MemoryQueryFilters {
                project: Some("xavier".to_string()),
                session_id: Some("session-wave-92".to_string()),
                ..MemoryQueryFilters::default()
            }),
            System3Mode::Disabled,
        )
        .await
        .expect("test assertion");

    assert!(trace.agent.response.to_lowercase().contains("system3"));
    assert!(!trace.action.llm_used);
}
