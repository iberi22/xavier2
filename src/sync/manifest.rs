//! Manifest tracking for Git-Chunk sync

use crate::sync::chunks::Chunk;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;

/// Metadata for a single chunk in the manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMeta {
    pub hash: String,
    pub path: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub size_bytes: u64,
}

/// Manifest tracking all chunks in a sync directory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub version: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "chunks")]
    chunks: Vec<ChunkMeta>,
}

impl Manifest {
    /// Create a new empty manifest
    pub fn new() -> Self {
        let now = chrono::Utc::now();
        Self {
            version: "1.0".to_string(),
            created_at: now,
            updated_at: now,
            chunks: Vec::new(),
        }
    }

    /// Add a chunk to the manifest
    pub fn add_chunk(&mut self, chunk: &Chunk) {
        // Avoid duplicates based on hash
        if !self.chunks.iter().any(|c| c.hash == chunk.hash) {
            self.chunks.push(ChunkMeta {
                hash: chunk.hash.clone(),
                path: chunk.path.clone(),
                created_at: chunk.created_at,
                size_bytes: chunk.content.len() as u64,
            });
            self.updated_at = chrono::Utc::now();
        }
    }

    /// Get all chunk metadata
    pub fn chunks(&self) -> &[ChunkMeta] {
        &self.chunks
    }

    /// Write manifest to JSON file
    pub async fn write(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self).context("failed to serialize manifest")?;
        fs::write(path, json).await.context("failed to write manifest")?;
        Ok(())
    }

    /// Read manifest from JSON file
    pub async fn read(path: &Path) -> Result<Self> {
        let json = fs::read_to_string(path).await.context("failed to read manifest")?;
        let manifest: Manifest =
            serde_json::from_str(&json).context("failed to parse manifest")?;
        Ok(manifest)
    }
}

impl Default for Manifest {
    fn default() -> Self {
        Self::new()
    }
}
