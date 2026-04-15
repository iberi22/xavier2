use anyhow::Result;
use chrono::Utc;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use crate::memory::qmd_memory::MemoryDocument;
use crate::utils::crypto::hex_encode;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChunkManifest {
    pub chunks: HashMap<String, ChunkMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    pub hash: String,
    pub document_ids: Vec<String>,
    pub created_at: i64,
}

pub fn export_to_chunk(
    sync_dir: &Path,
    documents: &[MemoryDocument],
    manifest: &mut ChunkManifest,
) -> Result<String> {
    let mut hasher = Sha256::new();
    let mut buffer = Vec::new();

    for doc in documents {
        let line = serde_json::to_string(doc)?;
        buffer.extend_from_slice(line.as_bytes());
        buffer.push(b'\n');
    }

    hasher.update(&buffer);
    let hash = hex_encode(&hasher.finalize());

    if manifest.chunks.contains_key(&hash) {
        return Ok(hash);
    }

    let chunks_dir = sync_dir.join("chunks");
    fs::create_dir_all(&chunks_dir)?;

    let chunk_filename = format!("{}.jsonl.gz", hash);
    let chunk_path = chunks_dir.join(&chunk_filename);

    let file = File::create(chunk_path)?;
    let mut encoder = GzEncoder::new(file, Compression::default());
    encoder.write_all(&buffer)?;
    encoder.finish()?;

    manifest.chunks.insert(
        hash.clone(),
        ChunkMetadata {
            hash: hash.clone(),
            document_ids: documents.iter().filter_map(|d| d.id.clone()).collect(),
            created_at: Utc::now().timestamp(),
        },
    );

    let manifest_path = sync_dir.join("manifest.json");
    let manifest_json = serde_json::to_string_pretty(manifest)?;
    fs::write(manifest_path, manifest_json)?;

    Ok(hash)
}

pub fn import_from_chunk(sync_dir: &Path, hash: &str) -> Result<Vec<MemoryDocument>> {
    let chunk_path = sync_dir.join("chunks").join(format!("{}.jsonl.gz", hash));
    let file = File::open(chunk_path)?;
    let decoder = GzDecoder::new(file);
    let reader = BufReader::new(decoder);

    let mut documents = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if !line.trim().is_empty() {
            let doc: MemoryDocument = serde_json::from_str(&line)?;
            documents.push(doc);
        }
    }

    Ok(documents)
}

pub fn load_manifest(sync_dir: &Path) -> Result<ChunkManifest> {
    let manifest_path = sync_dir.join("manifest.json");
    if !manifest_path.exists() {
        return Ok(ChunkManifest::default());
    }

    let manifest_json = fs::read_to_string(manifest_path)?;
    let manifest: ChunkManifest = serde_json::from_str(&manifest_json)?;
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_export_import_roundtrip() {
        let dir = tempdir().unwrap();
        let sync_dir = dir.path();

        let mut manifest = ChunkManifest::default();
        let docs = vec![
            MemoryDocument {
                id: Some("doc1".to_string()),
                path: "path1".to_string(),
                content: "content1".to_string(),
                metadata: serde_json::json!({}),
                content_vector: Some(vec![0.1, 0.2]),
                embedding: vec![0.1, 0.2],
            },
            MemoryDocument {
                id: Some("doc2".to_string()),
                path: "path2".to_string(),
                content: "content2".to_string(),
                metadata: serde_json::json!({"key": "value"}),
                content_vector: Some(vec![]),
                embedding: vec![],
            },
        ];

        let hash = export_to_chunk(sync_dir, &docs, &mut manifest).unwrap();
        assert!(!hash.is_empty());
        assert!(manifest.chunks.contains_key(&hash));

        let imported = import_from_chunk(sync_dir, &hash).unwrap();
        assert_eq!(docs, imported);

        let reloaded_manifest = load_manifest(sync_dir).unwrap();
        assert_eq!(manifest.chunks.len(), reloaded_manifest.chunks.len());
    }
}
