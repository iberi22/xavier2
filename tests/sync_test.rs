use anyhow::Result;
use tempfile::tempdir;
use xavier::memory::qmd_memory::MemoryDocument;
use xavier::sync::chunks::{export_to_chunk, import_from_chunk, load_manifest, ChunkManifest};

#[test]
fn test_sync_protocol_integration() -> Result<()> {
    let dir = tempdir()?;
    let sync_dir = dir.path();

    let docs = vec![
        MemoryDocument {
            id: Some("doc-integration-1".to_string()),
            path: "test/sync/1".to_string(),
            content: "First document for sync test".to_string(),
            metadata: serde_json::json!({"test": true}),
            content_vector: None,
            embedding: vec![0.1; 128],
        },
        MemoryDocument {
            id: Some("doc-integration-2".to_string()),
            path: "test/sync/2".to_string(),
            content: "Second document for sync test".to_string(),
            metadata: serde_json::json!({"priority": "high"}),
            content_vector: None,
            embedding: vec![0.2; 128],
        },
    ];

    let mut manifest = ChunkManifest::default();

    // 1. Export chunks
    let hash = export_to_chunk(sync_dir, &docs, &mut manifest)?;
    assert!(!hash.is_empty(), "Hash should not be empty");
    assert!(
        sync_dir
            .join("chunks")
            .join(format!("{}.jsonl.gz", hash))
            .exists(),
        "Chunk file should exist"
    );
    assert!(
        sync_dir.join("manifest.json").exists(),
        "Manifest file should exist"
    );

    // 2. Load manifest from disk
    let loaded_manifest = load_manifest(sync_dir)?;
    assert!(
        loaded_manifest.chunks.contains_key(&hash),
        "Loaded manifest should contain the hash"
    );

    // 3. Import chunks back
    let imported_docs = import_from_chunk(sync_dir, &hash)?;
    assert_eq!(imported_docs.len(), 2, "Should import 2 documents");
    assert_eq!(imported_docs[0].id, docs[0].id);
    assert_eq!(imported_docs[1].content, docs[1].content);

    Ok(())
}

#[test]
fn test_sync_no_duplicate_chunks() -> Result<()> {
    let dir = tempdir()?;
    let sync_dir = dir.path();
    let mut manifest = ChunkManifest::default();

    let docs = vec![MemoryDocument {
        id: Some("unique-1".to_string()),
        path: "p1".to_string(),
        content: "constant content".to_string(),
        metadata: serde_json::json!({}),
        content_vector: None,
        embedding: vec![],
    }];

    let hash1 = export_to_chunk(sync_dir, &docs, &mut manifest)?;
    let hash2 = export_to_chunk(sync_dir, &docs, &mut manifest)?;

    assert_eq!(
        hash1, hash2,
        "Identical content should produce identical hashes"
    );
    assert_eq!(
        manifest.chunks.len(),
        1,
        "Should not create duplicate chunk metadata entries"
    );

    Ok(())
}
