use criterion::{criterion_group, criterion_main, Criterion};
use sha2::{Digest, Sha256};
use tempfile::tempdir;
use tokio::runtime::Runtime;
use xavier::memory::belief_graph::BeliefRelation;
use xavier::memory::sqlite_vec_store::{VecSqliteMemoryStore, VecSqliteStoreConfig};
use xavier::memory::{HybridSearchMode, MemoryRecord, MemoryStore};

fn stable_key(kind: &str, parts: &[&str]) -> String {
    let mut digest = Sha256::new();
    digest.update(kind.as_bytes());
    for part in parts {
        digest.update([0u8]);
        digest.update(part.as_bytes());
    }
    hex::encode(digest.finalize())
}

fn bench_hybrid_search(c: &mut Criterion) {
    let runtime = Runtime::new().expect("tokio runtime");
    let temp_dir = tempdir().expect("temp dir");
    let workspace_id = "bench-hybrid";

    let store = runtime.block_on(async {
        VecSqliteMemoryStore::new(VecSqliteStoreConfig {
            path: temp_dir.path().join("hybrid-bench.db"),
            embedding_dimensions: 3,
        })
        .await
        .expect("vec store")
    });

    runtime.block_on(async {
        let docs = [
            (
                "memory/account-renewal",
                "Customer account ACCT-9F3A renewal approved by Alice Johnson.",
                vec![0.0, 1.0, 0.0],
            ),
            (
                "memory/account-summary",
                "Enterprise renewal planning notes for the customer account.",
                vec![1.0, 0.0, 0.0],
            ),
            (
                "memory/incident",
                "Incident INC-4821 escalated to OpenClaw runtime support.",
                vec![0.0, 0.0, 1.0],
            ),
            (
                "memory/runtime-notes",
                "Runtime support queue for infrastructure incidents and pager load.",
                vec![1.0, 0.0, 0.0],
            ),
            (
                "memory/repo-release",
                "Repository openclaw/xavier tagged release v0.4.1 for customer rollout.",
                vec![0.0, 1.0, 1.0],
            ),
            (
                "memory/release-summary",
                "Release planning notes for the next customer rollout.",
                vec![1.0, 1.0, 0.0],
            ),
        ];

        for (path, content, embedding) in docs {
            store
                .put(MemoryRecord {
                    id: stable_key("memory", &[workspace_id, path]),
                    workspace_id: workspace_id.to_string(),
                    path: path.to_string(),
                    content: content.to_string(),
                    metadata: serde_json::json!({}),
                    embedding,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                    revision: 1,
                    primary: true,
                    parent_id: None,
                    cluster_id: None,
                    level: xavier::memory::schema::MemoryLevel::Raw,
                    relation: None,
                    revisions: Vec::new(),
                })
                .await
                .expect("seed memory");
        }

        store
            .save_beliefs(
                workspace_id,
                vec![
                    BeliefRelation {
                        id: ulid::Ulid::new().to_string(),
                        source: "ACCT-9F3A".to_string(),
                        target: "Alice Johnson".to_string(),
                        relation_type: "approved_by".to_string(),
                        weight: 0.9,
                        confidence: 0.9,
                        source_memory_id: Some(stable_key(
                            "memory",
                            &[workspace_id, "memory/account-renewal"],
                        )),
                        valid_from: None,
                        valid_until: None,
                        superseded_by: None,
                        created_at: chrono::Utc::now(),
                        updated_at: chrono::Utc::now(),
                    },
                    BeliefRelation {
                        id: ulid::Ulid::new().to_string(),
                        source: "INC-4821".to_string(),
                        target: "OpenClaw".to_string(),
                        relation_type: "handled_by".to_string(),
                        weight: 0.8,
                        confidence: 0.8,
                        source_memory_id: Some(stable_key(
                            "memory",
                            &[workspace_id, "memory/incident"],
                        )),
                        valid_from: None,
                        valid_until: None,
                        superseded_by: None,
                        created_at: chrono::Utc::now(),
                        updated_at: chrono::Utc::now(),
                    },
                ],
            )
            .await
            .expect("seed beliefs");
    });

    let cases = [
        (
            "ACCT-9F3A renewal",
            [1.0, 0.0, 0.0],
            "memory/account-renewal",
        ),
        ("INC-4821 OpenClaw", [1.0, 0.0, 0.0], "memory/incident"),
        (
            "openclaw/xavier v0.4.1",
            [1.0, 1.0, 0.0],
            "memory/repo-release",
        ),
    ];

    let (vector_hits, hybrid_hits) = runtime.block_on(async {
        let mut vector_hits = 0usize;
        let mut hybrid_hits = 0usize;

        for (query, embedding, expected_path) in &cases {
            let vector_results = store
                .hybrid_search_with_embedding(
                    workspace_id,
                    query,
                    HybridSearchMode::Vector,
                    Some(embedding),
                    None,
                    3,
                )
                .await
                .expect("vector results");
            if vector_results
                .first()
                .is_some_and(|result| result.record.path == *expected_path)
            {
                vector_hits += 1;
            }

            let hybrid_results = store
                .hybrid_search_with_embedding(
                    workspace_id,
                    query,
                    HybridSearchMode::Both,
                    Some(embedding),
                    None,
                    3,
                )
                .await
                .expect("hybrid results");
            if hybrid_results
                .first()
                .is_some_and(|result| result.record.path == *expected_path)
            {
                hybrid_hits += 1;
            }
        }

        (vector_hits, hybrid_hits)
    });

    println!(
        "hybrid_search_hit_rate vector={}/{} hybrid={}/{}",
        vector_hits,
        cases.len(),
        hybrid_hits,
        cases.len()
    );

    c.bench_function("vector_search_exact_match_queries", |b| {
        b.iter(|| {
            runtime.block_on(async {
                for (query, embedding, _) in &cases {
                    let _ = store
                        .hybrid_search_with_embedding(
                            workspace_id,
                            query,
                            HybridSearchMode::Vector,
                            Some(embedding),
                            None,
                            3,
                        )
                        .await
                        .expect("vector search");
                }
            })
        });
    });

    c.bench_function("hybrid_search_exact_match_queries", |b| {
        b.iter(|| {
            runtime.block_on(async {
                for (query, embedding, _) in &cases {
                    let _ = store
                        .hybrid_search_with_embedding(
                            workspace_id,
                            query,
                            HybridSearchMode::Both,
                            Some(embedding),
                            None,
                            3,
                        )
                        .await
                        .expect("hybrid search");
                }
            })
        });
    });
}

criterion_group!(hybrid_search_benches, bench_hybrid_search);
criterion_main!(hybrid_search_benches);
