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
    let temp_dir = TempDir::new().unwrap();
    let workspace_path = temp_dir.path();

    // 1. Initialize a git repo
    let repo = git2::Repository::init(workspace_path).unwrap();
    let mut index = repo.index().unwrap();

    let file_path = workspace_path.join("src/main.rs");
    fs::create_dir_all(file_path.parent().unwrap()).unwrap();
    fs::write(&file_path, "fn main() {}").unwrap();

    index.add_path(std::path::Path::new("src/main.rs")).unwrap();
    let id = index.write_tree().unwrap();
    let tree = repo.find_tree(id).unwrap();
    let sig = git2::Signature::now("Xavier CI", "xavier-ci@example.invalid").unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "initial commit", &tree, &[])
        .unwrap();

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
    let code_db = Arc::new(CodeGraphDB::in_memory().unwrap());
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
        .unwrap();

    // 4. Run Harvester
    let harvester = Harvester::new(workspace_path.to_path_buf(), memory, code_db);
    let since = Utc::now() - Duration::days(1);

    let output_path = harvester.run(since).await.unwrap();

    // 5. Verify Output
    assert!(output_path.exists());
    let content = fs::read_to_string(output_path).unwrap();
    let output: serde_json::Value = serde_json::from_str(&content).unwrap();

    assert_eq!(output["decisions"][0]["path"], "decisions/arch.md");
    assert_eq!(output["commits"][0]["message"], "initial commit");
    assert_eq!(output["code_changes"][0]["file"], "src/main.rs");
    assert_eq!(output["code_changes"][0]["symbols"][0], "main");
}
