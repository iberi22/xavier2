use std::sync::Arc;
use tokio::sync::RwLock;
use xavier2::memory::qmd_memory::QmdMemory;

/// Minimal LoCoMo Benchmark struct
/// Validates Multi-hop, Single-hop, and Temporal Reasoning capabilities of Xavier2 Memory.
#[derive(Debug)]
struct LoCoMoQuestion {
    id: &'static str,
    query: &'static str,
    expected_doc_paths: Vec<&'static str>,
}

#[tokio::test]
async fn run_locomo_benchmark() {
    let raw_docs = Arc::new(RwLock::new(Vec::new()));
    let memory = QmdMemory::new(raw_docs);

    // 1. Seed the memory with test facts
    let facts = vec![
        (
            "docs/doc1",
            "Alice moved to Paris in 2020 to work as a software engineer.",
        ),
        (
            "docs/doc2",
            "Bob is a designer holding a master's degree from MIT.",
        ),
        (
            "docs/doc3",
            "Alice's favorite programming language is Rust, which she learned in 2021.",
        ),
        (
            "docs/doc4",
            "The new Xavier2 memory system was deployed by Alice and Bob together in 2023.",
        ),
    ];

    for (path, content) in facts {
        memory
            .add_document(path.to_string(), content.to_string(), serde_json::json!({}))
            .await
            .unwrap();
    }

    // 2. Define the benchmark questions
    let questions = vec![
        LoCoMoQuestion {
            id: "single-hop-01",
            query: "Where did Alice move in 2020?",
            expected_doc_paths: vec!["docs/doc1"],
        },
        LoCoMoQuestion {
            id: "multi-hop-01",
            query: "What language does the software engineer who moved to Paris use?",
            expected_doc_paths: vec!["docs/doc3", "docs/doc1"],
        },
        LoCoMoQuestion {
            id: "temporal-01",
            query: "Who deployed the Xavier2 memory system alongside Alice?",
            expected_doc_paths: vec!["docs/doc4"],
        },
    ];

    // 3. Execute and grade
    let mut passed = 0;

    for q in &questions {
        // We use the full Overdrive query pipeline (RRF)
        let results = xavier2::memory::qmd_memory::query_with_embedding(&memory, q.query, 10)
            .await
            .unwrap_or_default();
        let retrieved_paths: Vec<String> = results.iter().map(|d| d.path.clone()).collect();
        println!(
            "DEBUG for {}: {:?}",
            q.id,
            results
                .iter()
                .map(|d| d.content.clone())
                .collect::<Vec<_>>()
        );

        // Check if top documents cover the expected paths
        let mut is_correct = true;
        for expected in &q.expected_doc_paths {
            if !retrieved_paths.iter().take(3).any(|p| p == expected) {
                is_correct = false;
                break;
            }
        }

        if is_correct {
            passed += 1;
            println!("✅ PASS: {}", q.id);
        } else {
            println!(
                "❌ FAIL: {} (Got: {:?}, Expected: {:?})",
                q.id, retrieved_paths, q.expected_doc_paths
            );
        }
    }

    let accuracy = (passed as f32 / questions.len() as f32) * 100.0;
    println!("🏆 LoCoMo Benchmark Accuracy: {:.2}%", accuracy);

    // Assert high accuracy (e.g., >80%)
    assert!(
        accuracy >= 90.0,
        "Benchmark failed to meet minimum accuracy threshold"
    );
}
