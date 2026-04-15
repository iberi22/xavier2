use axum::{extract::Query, Extension, Json};
use criterion::{criterion_group, criterion_main, Criterion};
use std::sync::Arc;
use tokio::runtime::Runtime;
use xavier2::agents::RuntimeConfig;
use xavier2::server::v1_api::{
    v1_memories_add, v1_memories_list, v1_memories_search, V1AddMemoryRequest, V1PaginationParams,
    V1SearchRequest,
};
use xavier2::workspace::{WorkspaceConfig, WorkspaceContext, WorkspaceState};

fn bench_v1_api(c: &mut Criterion) {
    let runtime = Runtime::new().expect("tokio runtime");

    let temp_dir = std::env::temp_dir().join(format!("xavier2-bench-{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir).unwrap();

    let workspace = runtime.block_on(async {
        WorkspaceState::new(
            WorkspaceConfig {
                id: "bench".to_string(),
                token: "bench-token".to_string(),
                plan: xavier2::workspace::PlanTier::Pro,
                memory_backend: xavier2::memory::surreal_store::MemoryBackend::File,
                storage_limit_bytes: None,
                request_limit: None,
                request_unit_limit: None,
                embedding_provider_mode: xavier2::workspace::EmbeddingProviderMode::BringYourOwn,
                managed_google_embeddings: false,
                sync_policy: xavier2::workspace::SyncPolicy::LocalOnly,
            },
            RuntimeConfig::default(),
            temp_dir.join("threads"),
        )
        .await
        .unwrap()
    });

    let context = WorkspaceContext {
        workspace_id: "bench".to_string(),
        workspace: Arc::new(workspace),
    };

    // Pre-fill memory with some documents for search and list benchmarks
    runtime.block_on(async {
        for i in 0..100 {
            let _ = context.workspace.memory.add_document(
                format!("bench/doc/{}", i),
                format!("This is benchmark document number {} containing some content for semantic search testing.", i),
                serde_json::json!({"index": i})
            ).await;
        }
    });

    c.bench_function("v1_memories_add", |b| {
        b.iter(|| {
            runtime.block_on(async {
                let payload = V1AddMemoryRequest {
                    messages: None,
                    text: Some("new benchmark memory".to_string()),
                    metadata: None,
                    user_id: Some("bench-user".to_string()),
                    kind: None,
                    evidence_kind: None,
                    namespace: None,
                    provenance: None,
                };
                v1_memories_add(Extension(context.clone()), Json(payload)).await
            })
        });
    });

    c.bench_function("v1_memories_list_100", |b| {
        b.iter(|| {
            runtime.block_on(async {
                let params = V1PaginationParams {
                    limit: Some(100),
                    offset: Some(0),
                };
                v1_memories_list(Extension(context.clone()), Query(params)).await
            })
        });
    });

    c.bench_function("v1_memories_search", |b| {
        b.iter(|| {
            runtime.block_on(async {
                let payload = V1SearchRequest {
                    query: "benchmark document content".to_string(),
                    limit: Some(5),
                    filters: None,
                };
                v1_memories_search(Extension(context.clone()), Json(payload)).await
            })
        });
    });

    c.bench_function("sync_export_100_docs", |b| {
        let docs = runtime.block_on(async { context.workspace.memory.all_documents().await });
        b.iter(|| {
            let mut manifest = xavier2::sync::chunks::ChunkManifest::default();
            let _ = xavier2::sync::chunks::export_to_chunk(&temp_dir, &docs, &mut manifest);
        });
    });
}

criterion_group!(v1_api_benches, bench_v1_api);
criterion_main!(v1_api_benches);
