//! Indexer - scans and indexes codebase

use crate::db::CodeGraphDB;
use crate::error::{GraphError, Result};
use crate::parser::rust::RustParser;
use crate::types::IndexStats;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Semaphore;
use tracing::{debug, error, info, warn};
use walkdir::WalkDir;

pub struct Indexer {
    db: Arc<CodeGraphDB>,
    max_concurrent: usize,
}

impl Indexer {
    pub fn new(db: Arc<CodeGraphDB>) -> Self {
        Self {
            db,
            max_concurrent: 8,
        }
    }

    /// Index a directory
    pub async fn index(&self, root: &Path) -> Result<IndexStats> {
        let start = Instant::now();
        info!("Starting indexing of {:?}", root);

        // Collect files
        let files = self.collect_files(root)?;
        info!("Found {} files to index", files.len());

        // Clear existing data
        self.db.clear()?;

        // Process files
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent));
        let mut handles = Vec::new();

        for file_path in files {
            let sem = semaphore.clone();
            let db = self.db.clone();

            let handle = tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();

                // Get extension
                let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");

                // Support multiple languages
                let supported = [
                    "rs", "ts", "tsx", "js", "jsx", "py", "go", "java", "c", "cpp", "h",
                ];
                if !supported.contains(&ext) {
                    return Ok(());
                }

                // Read file
                let source = match std::fs::read_to_string(&file_path) {
                    Ok(s) => s,
                    Err(e) => {
                        warn!("Failed to read {:?}: {}", file_path, e);
                        return Ok(());
                    }
                };

                let relative_path = file_path.to_str().unwrap_or("").to_string();

                // Parse with Rust parser
                let mut parser = RustParser::new();
                match parser.parse(&source, &relative_path) {
                    Ok(symbols) => {
                        if !symbols.is_empty() {
                            debug!("Extracted {} symbols from {}", symbols.len(), relative_path);
                            if let Err(e) = db.insert_symbols(&symbols) {
                                error!("Failed to insert symbols: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to parse {:?}: {}", file_path, e);
                    }
                }

                Ok::<(), GraphError>(())
            });

            handles.push(handle);
        }

        // Wait for all tasks
        for handle in handles {
            if let Err(e) = handle.await {
                error!("Task failed: {}", e);
            }
        }

        // Get stats
        let mut stats = self.db.stats()?;
        stats.duration_ms = start.elapsed().as_millis() as u64;

        info!(
            "Indexed {} files, {} symbols in {}ms",
            stats.total_files, stats.total_symbols, stats.duration_ms
        );

        Ok(stats)
    }

    /// Collect all relevant files in a directory
    fn collect_files(&self, root: &Path) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        for entry in WalkDir::new(root).into_iter() {
            match entry {
                Ok(entry) => {
                    let path = entry.path();
                    // Skip hidden and common non-code directories
                    let should_skip = path.components().any(|c| {
                        let s = c.as_os_str().to_str().unwrap_or("");
                        s == "node_modules"
                            || s == "target"
                            || s == "__pycache__"
                            || s == ".git"
                            || s == "dist"
                            || s == "build"
                    });

                    if entry.file_type().is_file() && !should_skip {
                        files.push(path.to_path_buf());
                    }
                }
                Err(e) => {
                    warn!("Error walking directory: {}", e);
                }
            }
        }

        Ok(files)
    }
}
