//! Integration test for Hierarchical Curation & Retrieval

use serde_json::json;
use xavier::agents::RuntimeConfig;
use xavier::workspace::{
    EmbeddingProviderMode, PlanTier, SyncPolicy, WorkspaceConfig, WorkspaceState,
};

#[tokio::test]
async fn test_hierarchical_curation_and_retrieval() {
    // Setup workspace
    let temp_dir = std::env::temp_dir().join(format!("xavier-test-{}", uuid::Uuid::new_v4()));
    let config = WorkspaceConfig {
        id: "test-curation".to_string(),
        token: "test-token".to_string(),
        plan: PlanTier::Personal,
        memory_backend: xavier::memory::store::MemoryBackend::File,
        storage_limit_bytes: Some(10 * 1024 * 1024),
        request_limit: Some(1000),
        request_unit_limit: Some(2000),
        embedding_provider_mode: EmbeddingProviderMode::BringYourOwn,
        managed_google_embeddings: false,
        sync_policy: SyncPolicy::LocalOnly,
    };

    let workspace = WorkspaceState::new(config, RuntimeConfig::default(), temp_dir)
        .await
        .expect("Failed to create workspace");

    // Enable a mock LLM or skip actual LLM calls if not configured
    // For this test, we want to verify the pipeline flow.
    // If no LLM is configured, curate will fail or return default.
    // We can manually populate metadata to test retrieval expansion.

    // 1. Ingest a document
    let content = "Rust is a systems programming language that provides memory safety without garbage collection.";
    let path = "rust/intro".to_string();
    let metadata = json!({"type": "language_intro"});

    let doc_id = workspace
        .ingest(path.clone(), content.to_string(), metadata, false)
        .await
        .expect("Failed to ingest document");

    // 2. Manually "curate" it since we might not have a live LLM in the test environment
    if let Some(mut doc) = workspace.memory.get(&doc_id).await.unwrap() {
        if let Some(meta) = doc.metadata.as_object_mut() {
            meta.insert("domain".to_string(), json!("Technology"));
            meta.insert("topic".to_string(), json!("Rust Programming"));
        }
        workspace.memory.update(doc).await.unwrap();
    }

    // 3. Add a related document in the same domain/topic
    workspace.ingest(
        "rust/ownership".to_string(),
        "Ownership is Rust's most unique feature, and it enables memory safety without a garbage collector.".to_string(),
        json!({"domain": "Technology", "topic": "Rust Programming"}),
        false
    ).await.unwrap();

    // 4. Test Retrieval Expansion
    // When we search for "Rust intro", it should find "rust/intro"
    // and then expand to "rust/ownership" because they share domain/topic.

    let results = workspace
        .runtime
        .run_with_trace("Rust intro", None, None)
        .await
        .expect("Failed to run agent");

    // Check if both documents are in retrieval results
    let doc_paths: Vec<_> = results
        .retrieval
        .documents
        .iter()
        .map(|d| d.path.as_str())
        .collect();

    assert!(doc_paths.contains(&"rust/intro"));
    // This part verifies that hierarchical expansion worked!
    assert!(
        doc_paths.contains(&"rust/ownership"),
        "Should have expanded to include related topic doc"
    );

    // 5. Test Belief Graph Augmentation
    {
        let graph = workspace.belief_graph.read().await;
        graph.add_node("Rust".to_string(), 0.9);
        graph.add_node("Memory Safety".to_string(), 0.9);
        graph
            .add_edge(
                "Rust".to_string(),
                "Memory Safety".to_string(),
                "provides".to_string(),
            )
            .await;
    }

    let results_with_graph = workspace
        .runtime
        .run_with_trace("What does Rust provide?", None, None)
        .await
        .unwrap();
    let graph_context = results_with_graph
        .retrieval
        .documents
        .iter()
        .find(|d| d.path == "belief_graph");

    assert!(
        graph_context.is_some(),
        "Should include belief graph context"
    );
    assert!(graph_context
        .unwrap()
        .content
        .contains("FACT: Rust provides Memory Safety"));
}
