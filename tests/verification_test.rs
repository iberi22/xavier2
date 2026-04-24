use std::sync::Arc;
use tokio::sync::RwLock;
use xavier2::memory::qmd_memory::{MemoryDocument, QmdMemory};
use xavier2::verification::verify_save;
use xavier2::workspace::{WorkspaceConfig, WorkspaceState};
use xavier2::agents::RuntimeConfig;

#[tokio::test]
async fn test_verify_save_success() {
    let memory = QmdMemory::new(Arc::new(RwLock::new(Vec::new())));
    let content = "This is a test memory for verification automation.";
    let path = "test/verify/1";

    // Add the document
    memory.add(MemoryDocument {
        id: Some("test-id".to_string()),
        path: path.to_string(),
        content: content.to_string(),
        metadata: serde_json::json!({}),
        content_vector: Some(Vec::new()),
        embedding: Vec::new(),
    }).await.unwrap();

    // Verify
    let result = verify_save(&memory, content).await;

    assert!(result.save_ok);
    assert!(result.content_identical);
    assert!(result.match_score > 0.9);
}

#[tokio::test]
async fn test_verify_save_failure_not_found() {
    let memory = QmdMemory::new(Arc::new(RwLock::new(Vec::new())));
    let content = "This document is not in memory.";

    // Verify without adding
    let result = verify_save(&memory, content).await;

    assert!(!result.save_ok);
    assert_eq!(result.match_score, 0.0);
}

#[tokio::test]
async fn test_process_verification_creates_files() {
    let root = std::env::temp_dir().join(format!("xavier2-test-verify-{}", ulid::Ulid::new()));
    let config = WorkspaceConfig {
        id: "test-verify".to_string(),
        token: "token".to_string(),
        plan: xavier2::workspace::PlanTier::Personal,
        memory_backend: xavier2::memory::surreal_store::MemoryBackend::Memory,
        storage_limit_bytes: None,
        request_limit: None,
        request_unit_limit: None,
        embedding_provider_mode: xavier2::workspace::EmbeddingProviderMode::BringYourOwn,
        managed_google_embeddings: false,
        sync_policy: xavier2::workspace::SyncPolicy::LocalOnly,
    };

    let workspace = WorkspaceState::new(config, RuntimeConfig::default(), &root)
        .await
        .unwrap();

    let workspace_ctx = xavier2::workspace::WorkspaceContext {
        workspace_id: "test-verify".to_string(),
        workspace: Arc::new(workspace),
    };

    let result = xavier2::verification::VerificationResult {
        save_ok: true,
        latency_ms: 10,
        match_score: 1.0,
        content_identical: true,
    };

    xavier2::verification::auto_verifier::process_verification(
        workspace_ctx,
        "test/path".to_string(),
        "test content".to_string(),
        result,
    ).await;

    // Check if log file exists
    let log_dir = std::path::Path::new("feedback/xavier2");
    assert!(log_dir.exists());

    let entries = std::fs::read_dir(log_dir).unwrap();
    let count = entries.filter_map(|e| e.ok()).count();
    assert!(count > 0);
}
