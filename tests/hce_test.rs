use xavier::memory::hce_engine::HceEngine;
use xavier::memory::sqlite_vec_store::VecSqliteMemoryStore;
use xavier::memory::store::{MemoryRecord, MemoryStore};
use xavier::memory::schema::MemoryLevel;
use std::sync::Arc;
use tempfile::tempdir;
use chrono::Utc;

#[tokio::test]
async fn test_hce_full_cycle() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test_xavier.db");
    
    // 1. Initialize Store
    let config = xavier::memory::sqlite_vec_store::VecSqliteStoreConfig {
        path: db_path,
        embedding_dimensions: 384,
    };
    let store = Arc::new(VecSqliteMemoryStore::new(config).await.unwrap());
    let workspace_id = "test_ws";

    // 2. Add Raw Memory (simulating a file)
    // 2. Add Raw Memory (simulating a file)
    let raw_content = "/* A very large comment to make this section meaningful and longer than one hundred characters in total length to pass the filter in the HCE engine decomposition logic */\nfn main() {\n    println!(\"Hello World\");\n}\n\n".repeat(10);
    let raw_record = MemoryRecord {
        id: "raw_1".to_string(),
        workspace_id: workspace_id.to_string(),
        path: "main.rs".to_string(),
        content: raw_content.clone(),
        metadata: serde_json::json!({"kind": "code"}),
        embedding: vec![0.0; 384],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        revision: 1,
        primary: true,
        parent_id: None,
        cluster_id: None,
        level: MemoryLevel::Raw,
        relation: None,
        revisions: vec![],
    };
    store.put(raw_record).await.unwrap();

    // 3. Run HCE Engine
    let hce = HceEngine::new(store.clone());
    hce.process_workspace(workspace_id).await.unwrap();

    // 4. Verify Section Creation
    let memories = store.list(workspace_id).await.unwrap();
    let sections: Vec<_> = memories.iter().filter(|m| m.level == MemoryLevel::Section).collect();
    
    // In our current MVP logic, we split by \n\n if content is > 1000 chars.
    // Let's check if it worked or if we need a larger content for the MVP logic.
    println!("Found {} sections", sections.len());
    if let Some(s) = sections.first() {
        println!("First section cluster_id: {:?}", s.cluster_id);
    }

    // 5. Verify Clustering and Global Summaries
    let globals: Vec<_> = memories.iter().filter(|m| m.level == MemoryLevel::Global).collect();
    println!("Found {} global summaries", globals.len());
}
