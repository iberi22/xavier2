use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use tokio::runtime::Runtime;
use xavier2::memory::belief_graph::{Belief, BeliefGraph, Confidence};
use xavier2::security::SecurityManager;

fn bench_belief_graph_search(c: &mut Criterion) {
    let runtime = Runtime::new().expect("tokio runtime");
    let graph = BeliefGraph::new();

    runtime.block_on(async {
        for i in 0..100 {
            graph
                .add_belief(
                    Belief::new(
                        format!("subject-{i}"),
                        "relates_to".to_string(),
                        format!("topic-{}", i % 10),
                        Confidence::Medium,
                    ),
                    None,
                    None,
                )
                .await;
        }
    });

    c.bench_function("belief_graph_search_100_items", |b| {
        b.iter(|| {
            let results = runtime.block_on(graph.search("topic-5"));
            black_box(results);
        });
    });
}

fn bench_security_hash_password(c: &mut Criterion) {
    let security = SecurityManager::new();

    c.bench_function("security_hash_password", |b| {
        b.iter(|| {
            let hash = security
                .hash_password("benchmark-password")
                .expect("hash password");
            black_box(hash);
        });
    });
}

criterion_group!(
    xavier2_benches,
    bench_belief_graph_search,
    bench_security_hash_password
);
criterion_main!(xavier2_benches);
