//! Indexer - scans and indexes codebases.

use crate::db::CodeGraphDB;
use crate::error::{GraphError, Result};
use crate::parser::parse_source;
use crate::types::{CodeEdge, EdgeType, IndexStats, Language, Symbol, SymbolKind};
use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::WalkBuilder;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Semaphore;
use tracing::{debug, error, info, warn};

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

    /// Index a directory.
    pub async fn index(&self, root: &Path) -> Result<IndexStats> {
        let start = Instant::now();
        info!("Starting indexing of {:?}", root);

        let files = self.collect_files(root)?;
        info!("Found {} files to index", files.len());

        self.db.clear()?;

        let semaphore = Arc::new(Semaphore::new(self.max_concurrent));
        let mut handles = Vec::new();

        for file_path in files {
            let sem = semaphore.clone();
            let root = root.to_path_buf();
            let handle = tokio::spawn(async move {
                let _permit = sem.acquire().await.expect("semaphore should be open");
                tokio::task::spawn_blocking(move || parse_file(&root, &file_path))
                    .await
                    .map_err(|e| GraphError::Parser(e.to_string()))?
            });
            handles.push(handle);
        }

        let mut symbols = Vec::new();
        let mut sources = HashMap::new();
        for handle in handles {
            match handle.await {
                Ok(Ok(parsed)) => {
                    sources.insert(parsed.file_path.clone(), parsed.source);
                    symbols.extend(parsed.symbols);
                }
                Ok(Err(error)) => warn!("Failed to parse file: {}", error),
                Err(error) => error!("Task failed: {}", error),
            }
        }

        assign_stable_ids(&mut symbols);
        let edges = build_edges(&symbols, &sources);

        self.db.insert_symbols(&symbols)?;
        self.db.insert_edges(&edges)?;

        let mut stats = self.db.stats()?;
        stats.duration_ms = start.elapsed().as_millis() as u64;

        info!(
            "Indexed {} files, {} symbols, {} edges in {}ms",
            stats.total_files,
            stats.total_symbols,
            edges.len(),
            stats.duration_ms
        );

        Ok(stats)
    }

    /// Collect all relevant files in a directory using .gitignore/.ignore aware traversal.
    fn collect_files(&self, root: &Path) -> Result<Vec<PathBuf>> {
        let excludes = build_excludes(&[
            "**/target/**",
            "**/.git/**",
            "**/node_modules/**",
            "**/dist/**",
            "**/build/**",
            "**/.next/**",
            "**/.nuxt/**",
            "**/coverage/**",
            "**/__pycache__/**",
            "**/.pytest_cache/**",
            "**/.codegraph/**",
        ]);

        let mut files = Vec::new();
        let walker = WalkBuilder::new(root)
            .hidden(false)
            .git_ignore(true)
            .git_exclude(true)
            .ignore(true)
            .require_git(false)
            .build();

        for entry in walker {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    warn!("Error walking directory: {}", error);
                    continue;
                }
            };
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if excludes.as_ref().is_some_and(|set| set.is_match(path)) {
                continue;
            }
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if Language::from_extension(ext) == Language::Unknown {
                continue;
            }
            files.push(path.to_path_buf());
        }

        Ok(files)
    }
}

struct ParsedFile {
    file_path: String,
    source: String,
    symbols: Vec<Symbol>,
}

fn parse_file(root: &Path, file_path: &Path) -> Result<ParsedFile> {
    let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let lang = Language::from_extension(ext);
    if lang == Language::Unknown {
        return Ok(ParsedFile {
            file_path: file_path.to_string_lossy().to_string(),
            source: String::new(),
            symbols: Vec::new(),
        });
    }

    let source = std::fs::read_to_string(file_path).map_err(GraphError::Io)?;
    let relative_path = file_path
        .strip_prefix(root)
        .unwrap_or(file_path)
        .to_string_lossy()
        .replace('\\', "/");
    let symbols = parse_source(&source, &lang, &relative_path)?;
    if !symbols.is_empty() {
        debug!("Extracted {} symbols from {}", symbols.len(), relative_path);
    }

    Ok(ParsedFile {
        file_path: relative_path,
        source,
        symbols,
    })
}

fn assign_stable_ids(symbols: &mut [Symbol]) {
    for symbol in symbols {
        if symbol.stable_id.is_none() {
            symbol.stable_id = Some(symbol.deterministic_id("default"));
        }
    }
}

fn build_edges(symbols: &[Symbol], sources: &HashMap<String, String>) -> Vec<CodeEdge> {
    let mut edges = Vec::new();
    let callable_symbols: Vec<&Symbol> = symbols
        .iter()
        .filter(|symbol| matches!(symbol.kind, SymbolKind::Function | SymbolKind::Method))
        .collect();

    for symbol in symbols {
        let symbol_id = symbol
            .stable_id
            .clone()
            .unwrap_or_else(|| symbol.deterministic_id("default"));
        let file_node = format!("file:{}", symbol.file_path);

        edges.push(CodeEdge {
            id: None,
            from_symbol: file_node.clone(),
            to_symbol: symbol_id.clone(),
            edge_type: EdgeType::Contains,
            file_path: symbol.file_path.clone(),
            line: symbol.start_line,
            confidence: 1.0,
            metadata: None,
        });

        edges.push(CodeEdge {
            id: None,
            from_symbol: file_node.clone(),
            to_symbol: symbol_id.clone(),
            edge_type: EdgeType::Defines,
            file_path: symbol.file_path.clone(),
            line: symbol.start_line,
            confidence: 1.0,
            metadata: None,
        });

        if symbol.kind == SymbolKind::Import {
            edges.push(CodeEdge {
                id: None,
                from_symbol: file_node,
                to_symbol: format!("module:{}", symbol.name),
                edge_type: EdgeType::Imports,
                file_path: symbol.file_path.clone(),
                line: symbol.start_line,
                confidence: 0.8,
                metadata: None,
            });
        }
    }

    for caller in &callable_symbols {
        let Some(source) = sources.get(&caller.file_path) else {
            continue;
        };
        let caller_id = caller
            .stable_id
            .clone()
            .unwrap_or_else(|| caller.deterministic_id("default"));
        let body = symbol_source_slice(source, caller);
        for callee in &callable_symbols {
            if caller.stable_id == callee.stable_id || caller.name == callee.name {
                continue;
            }
            if contains_call(&body, &callee.name) {
                let callee_id = callee
                    .stable_id
                    .clone()
                    .unwrap_or_else(|| callee.deterministic_id("default"));
                edges.push(CodeEdge {
                    id: None,
                    from_symbol: caller_id.clone(),
                    to_symbol: callee_id.clone(),
                    edge_type: EdgeType::Calls,
                    file_path: caller.file_path.clone(),
                    line: caller.start_line,
                    confidence: 0.65,
                    metadata: Some(serde_json::json!({"callee": callee.name})),
                });
                edges.push(CodeEdge {
                    id: None,
                    from_symbol: caller_id.clone(),
                    to_symbol: callee_id,
                    edge_type: EdgeType::References,
                    file_path: caller.file_path.clone(),
                    line: caller.start_line,
                    confidence: 0.55,
                    metadata: Some(serde_json::json!({"reference": callee.name})),
                });
            }
        }
    }

    edges
}

fn symbol_source_slice(source: &str, symbol: &Symbol) -> String {
    let start = symbol.start_line.saturating_sub(1) as usize;
    let end = symbol.end_line as usize;
    source
        .lines()
        .skip(start)
        .take(end.saturating_sub(start).max(1))
        .collect::<Vec<_>>()
        .join("\n")
}

fn contains_call(source: &str, name: &str) -> bool {
    let needle = format!("{}(", name);
    let method_needle = format!(".{}(", name);
    source.contains(&needle) || source.contains(&method_needle)
}

fn build_excludes(patterns: &[&str]) -> Option<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    let mut added = false;
    for pattern in patterns {
        match Glob::new(pattern) {
            Ok(glob) => {
                builder.add(glob);
                added = true;
            }
            Err(error) => warn!("Invalid glob pattern '{}': {}", pattern, error),
        }
    }
    added.then(|| builder.build().ok()).flatten()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn indexes_multiple_languages_and_edges() {
        let dir = TempDir::new().expect("temp dir");
        std::fs::write(
            dir.path().join("main.rs"),
            "fn helper() {}\nfn main() { helper(); }\n",
        )
        .expect("write rust");
        std::fs::write(
            dir.path().join("app.py"),
            "import os\nclass Service:\n    def run(self):\n        return os.getcwd()\n",
        )
        .expect("write python");

        let db = Arc::new(CodeGraphDB::in_memory().expect("db"));
        let indexer = Indexer::new(db.clone());
        let stats = indexer.index(dir.path()).await.expect("index");

        assert_eq!(stats.total_files, 2);
        assert!(stats.total_symbols >= 5);
        assert!(stats.total_imports >= 1);
        assert!(!db.hub_nodes(1, 10).expect("hubs").is_empty());
    }

    #[test]
    fn collector_respects_gitignore_and_common_excludes() {
        let dir = TempDir::new().expect("temp dir");
        std::fs::write(dir.path().join(".gitignore"), "ignored.rs\n").expect("gitignore");
        std::fs::write(dir.path().join("main.rs"), "fn main() {}\n").expect("main");
        std::fs::write(dir.path().join("ignored.rs"), "fn ignored() {}\n").expect("ignored");
        std::fs::create_dir(dir.path().join("target")).expect("target");
        std::fs::write(dir.path().join("target").join("skip.rs"), "fn skip() {}\n").expect("skip");

        let db = Arc::new(CodeGraphDB::in_memory().expect("db"));
        let indexer = Indexer::new(db);
        let files = indexer.collect_files(dir.path()).expect("collect");

        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("main.rs"));
    }
}
