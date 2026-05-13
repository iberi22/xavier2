use chrono::{Duration, Utc};
use std::fs;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::RwLock as AsyncRwLock;

use code_graph::db::CodeGraphDB;
use code_graph::types::{Language, Symbol, SymbolKind};
use xavier::chronicle::harvest::Harvester;
use xavier::memory::qmd_memory::{MemoryDocument, QmdMemory};

#[tokio::test]
async fn test_full_harvest_workflow() {
    let temp_dir = TempDir::new().expect("test assertion");
    let workspace_path = temp_dir.path();

    // 1. Initialize a git repo
    let repo = git2::Repository::init(workspace_path).expect("test assertion");
    let mut index = repo.index().expect("test assertion");

    let file_path = workspace_path.join("src/main.rs");
    fs::create_dir_all(file_path.parent().expect("test assertion")).expect("test assertion");
    fs::write(&file_path, "fn main() {}").expect("test assertion");

    index.add_path(std::path::Path::new("src/main.rs")).expect("test assertion");
    let id = index.write_tree().expect("test assertion");
    let tree = repo.find_tree(id).expect("test assertion");
    let sig = git2::Signature::now("Xavier CI", "xavier-ci@example.invalid").expect("test assertion");
    repo.commit(Some("HEAD"), &sig, &sig, "initial commit", &tree, &[])
        .expect("test assertion");

    // 2. Setup QmdMemory
    let memory = Arc::new(QmdMemory::new(Arc::new(AsyncRwLock::new(vec![
        MemoryDocument {
            id: Some("d1".into()),
            path: "decisions/arch.md".into(),
            content: "Use SQLite".into(),
            metadata: serde_json::json!({"status": "accepted"}),
            content_vector: None,
            embedding: vec![],
        },
    ]))));

    // 3. Setup CodeGraphDB
    let code_db = Arc::new(CodeGraphDB::in_memory().expect("test assertion"));
    code_db
        .insert_symbol(&Symbol {
            id: None,
            name: "main".into(),
            kind: SymbolKind::Function,
            lang: Language::Rust,
            file_path: "src/main.rs".into(),
            start_line: 1,
            end_line: 1,
            start_col: 0,
            end_col: 10,
            signature: Some("fn main()".into()),
            parent: None,
        })
        .expect("test assertion");

    // 4. Run Harvester
    let harvester = Harvester::new(workspace_path.to_path_buf(), memory, code_db);
    let since = Utc::now() - Duration::days(1);

    let output_path = harvester.run(since).await.expect("test assertion");

    // 5. Verify Output
    assert!(output_path.exists());
    let content = fs::read_to_string(output_path).expect("test assertion");
    let output: serde_json::Value = serde_json::from_str(&content).expect("test assertion");

    assert_eq!(output["decisions"][0]["path"], "decisions/arch.md");
    assert_eq!(output["commits"][0]["message"], "initial commit");
    assert_eq!(output["code_changes"][0]["file"], "src/main.rs");
    assert_eq!(output["code_changes"][0]["symbols"][0], "main");
}
