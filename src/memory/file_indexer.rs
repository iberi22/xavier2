//! File Indexer - Indexa archivos markdown para memoria
//!
//! Lee archivos del Tier 1 (MEMORY.md, memory/*.md),
//! genera chunks y embeddings, almacena en el backend de memoria configurado.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, info, warn};

use crate::memory::qmd_memory::QmdMemory;

/// Configuración del indexer
#[derive(Debug, Clone)]
pub struct FileIndexerConfig {
    pub root_path: PathBuf,
    pub include_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
    pub chunk_size: usize,
    pub chunk_overlap: usize,
}

impl Default for FileIndexerConfig {
    fn default() -> Self {
        Self {
            root_path: PathBuf::from("/data/projects"),
            include_patterns: vec![".md".to_string()],
            exclude_patterns: vec![
                ".git/**".to_string(),
                "node_modules/**".to_string(),
                "__pycache__/**".to_string(),
                "target/**".to_string(),
                ".venv/**".to_string(),
                "venv/**".to_string(),
                ".xavier/**".to_string(),
                "dist/**".to_string(),
                "build/**".to_string(),
                ".next/**".to_string(),
                ".cache/**".to_string(),
            ],
            chunk_size: 400,   // tokens
            chunk_overlap: 80, // tokens
        }
    }
}

/// Archivo indexado
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedFile {
    pub path: String,
    pub content: String,
    pub chunks: Vec<FileChunk>,
    pub last_modified: String,
    pub size: usize,
}

/// Chunk de un archivo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChunk {
    pub index: usize,
    pub content: String,
    pub start_line: usize,
    pub end_line: usize,
}

/// Resultado de la indexación
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexResult {
    pub total_files: usize,
    pub total_chunks: usize,
    pub errors: Vec<String>,
    pub indexed_paths: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<IndexedFile>,
}

use std::sync::Arc;

/// File Indexer
#[derive(Clone)]
pub struct FileIndexer {
    config: FileIndexerConfig,
    pub code_indexer: Option<Arc<code_graph::indexer::Indexer>>,
}

impl FileIndexer {
    /// Crea un nuevo indexer
    pub fn new(
        config: FileIndexerConfig,
        code_indexer: Option<Arc<code_graph::indexer::Indexer>>,
    ) -> Self {
        Self {
            config,
            code_indexer,
        }
    }

    /// Guarda el índice en un archivo JSON dentro del proyecto
    pub async fn save_index(&self, result: &IndexResult) -> Result<PathBuf> {
        let index_path = self.config.root_path.join(".xavier").join("index.json");

        // Create .xavier directory
        fs::create_dir_all(index_path.parent().unwrap()).await?;

        // Write index file
        let json = serde_json::to_string_pretty(result)?;
        fs::write(&index_path, json).await?;

        info!("💾 Index saved to: {:?}", index_path);
        Ok(index_path)
    }

    /// Carga un índice existente desde el proyecto
    pub async fn load_index(&self) -> Result<Option<IndexResult>> {
        let index_path = self.config.root_path.join(".xavier").join("index.json");

        if !index_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&index_path).await?;
        let result: IndexResult = serde_json::from_str(&content)?;

        info!("📂 Loaded index from: {:?}", index_path);
        Ok(Some(result))
    }

    /// Indexa todos los archivos markdown en el path raíz
    pub async fn index_all(&self) -> Result<IndexResult> {
        info!("📂 Starting file indexing: {:?}", self.config.root_path);

        let mut result = IndexResult {
            total_files: 0,
            total_chunks: 0,
            errors: vec![],
            indexed_paths: vec![],
            files: vec![],
        };

        // Recursively find all markdown files
        let mut stack = vec![self.config.root_path.clone()];

        while let Some(path) = stack.pop() {
            // Skip if not a directory (only process directories)
            if !path.is_dir() {
                continue;
            }

            // Add subdirectories to stack FIRST
            if let Ok(mut entries) = fs::read_dir(&path).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    let entry_path = entry.path();
                    if entry_path.is_dir() {
                        // Skip excluded directories
                        if !self.should_exclude(&entry_path) {
                            stack.push(entry_path);
                        }
                    } else if entry_path.is_file() {
                        // Only index files that match the pattern
                        if self.should_include(&entry_path) {
                            match self.index_file(&entry_path).await {
                                Ok(file) => {
                                    let path_clone = file.path.clone();
                                    result.total_files += 1;
                                    result.total_chunks += file.chunks.len();
                                    result.indexed_paths.push(path_clone.clone());
                                    result.files.push(file);
                                    debug!("Indexed: {}", path_clone);
                                }
                                Err(e) => {
                                    warn!("Error indexing {:?}: {}", entry_path, e);
                                    result.errors.push(format!("{:?}: {}", entry_path, e));
                                }
                            }
                        }
                    }
                }
            }
        }

        // Save index to project
        if let Err(e) = self.save_index(&result).await {
            warn!("Failed to save index: {}", e);
        }

        info!(
            "✅ Indexing complete: {} files, {} chunks",
            result.total_files, result.total_chunks
        );

        Ok(result)
    }

    /// Indexa un archivo individual
    pub async fn index_file(&self, path: &Path) -> Result<IndexedFile> {
        let path_str = path.to_string_lossy().to_string();

        // If it's a code file and we have a code indexer, trigger it
        if let Some(ref code_indexer) = self.code_indexer {
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                let code_exts = ["rs", "py", "ts", "js", "go", "java"];
                if code_exts.contains(&ext) {
                    debug!("🚀 Triggering code indexing for: {:?}", path);
                    let _ = code_indexer.index(path).await;
                }
            }
        }

        let content = fs::read_to_string(path)
            .await
            .with_context(|| format!("Failed to read file: {:?}", path))?;

        let metadata = fs::metadata(path).await?;
        let last_modified = metadata
            .modified()
            .map(|t| {
                let datetime: chrono::DateTime<chrono::Utc> = t.into();
                datetime.to_rfc3339()
            })
            .unwrap_or_else(|_| "unknown".to_string());

        // Generate chunks
        let chunks = self.generate_chunks(&content);

        Ok(IndexedFile {
            path: path_str,
            content,
            chunks,
            last_modified,
            size: metadata.len() as usize,
        })
    }

    /// Genera chunks de un contenido
    fn generate_chunks(&self, content: &str) -> Vec<FileChunk> {
        let mut chunks = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        let mut current_chunk = String::new();
        let mut start_line = 0;
        let mut line_count = 0;

        for (i, line) in lines.iter().enumerate() {
            current_chunk.push_str(line);
            current_chunk.push('\n');
            line_count += 1;

            // Split by paragraphs or size limit
            if line_count >= 20 || line.is_empty() && current_chunk.len() > 200 {
                if !current_chunk.trim().is_empty() {
                    chunks.push(FileChunk {
                        index: chunks.len(),
                        content: current_chunk.clone(),
                        start_line,
                        end_line: i,
                    });
                }
                current_chunk.clear();
                start_line = i + 1;
                line_count = 0;
            }
        }

        // Add last chunk
        if !current_chunk.trim().is_empty() {
            chunks.push(FileChunk {
                index: chunks.len(),
                content: current_chunk,
                start_line,
                end_line: lines.len(),
            });
        }

        chunks
    }

    /// Verifica si un path debe ser excluido
    fn should_exclude(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        for pattern in &self.config.exclude_patterns {
            if pattern.contains("**") {
                // Handle wildcard patterns
                let base = pattern.trim_start_matches("**/").trim_end_matches("/**");
                if path_str.contains(base) {
                    return true;
                }
            } else if path_str.contains(pattern) {
                return true;
            }
        }

        false
    }

    /// Verifica si un archivo debe ser incluido según los patrones
    fn should_include(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        for pattern in &self.config.include_patterns {
            let pattern_clean = pattern.trim_start_matches("*");
            if path_str.ends_with(pattern_clean)
                || path_str
                    .to_lowercase()
                    .ends_with(pattern_clean.to_lowercase().as_str())
            {
                return true;
            }
        }

        false
    }

    /// Sincronización incremental - solo archivos modificados
    pub async fn sync_incremental(&self, _memory: &QmdMemory) -> Result<IndexResult> {
        info!("🔄 Starting incremental sync");

        // TODO: Check file modification times vs indexed times
        // Only re-index files that have changed

        self.index_all().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_generation() {
        let config = FileIndexerConfig::default();
        let indexer = FileIndexer::new(config, None);

        let content = "Line 1\nLine 2\n\nLine 4\nLine 5\n";
        let chunks = indexer.generate_chunks(content);

        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_exclude_patterns() {
        let config = FileIndexerConfig::default();
        let indexer = FileIndexer::new(config, None);

        assert!(indexer.should_exclude(Path::new("/foo/node_modules/bar")));
        assert!(indexer.should_exclude(Path::new("/foo/.git/config")));
        assert!(!indexer.should_exclude(Path::new("/foo/src/main.rs")));
    }
}
