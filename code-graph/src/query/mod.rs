//! Query engine for code graph

pub mod tests;

use crate::db::CodeGraphDB;
use crate::error::Result;
use crate::types::{QueryResult, Symbol, SymbolKind};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Simple in-memory cache for query results
pub struct QueryCache {
    cache: RwLock<HashMap<String, (Instant, QueryResult)>>,
    ttl: Duration,
    max_entries: usize,
}

impl QueryCache {
    pub fn new(ttl_secs: u64, max_entries: usize) -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            ttl: Duration::from_secs(ttl_secs),
            max_entries,
        }
    }

    /// Get cached result if still valid
    pub fn get(&self, query: &str) -> Option<QueryResult> {
        let cache = self.cache.read().unwrap();
        cache.get(query).and_then(|(time, result)| {
            if time.elapsed() < self.ttl {
                Some(result.clone())
            } else {
                None
            }
        })
    }

    /// Store result in cache
    pub fn set(&self, query: String, result: QueryResult) {
        let mut cache = self.cache.write().unwrap();

        // Evict old entries if at capacity
        if cache.len() >= self.max_entries {
            let now = Instant::now();
            cache.retain(|_, (time, _)| now.duration_since(*time) < self.ttl);

            // If still at capacity, remove oldest
            if cache.len() >= self.max_entries {
                if let Some(oldest) = cache
                    .iter()
                    .min_by_key(|(_, (time, _))| *time)
                    .map(|(k, _)| k.clone())
                {
                    cache.remove(&oldest);
                }
            }
        }

        cache.insert(query, (Instant::now(), result));
    }

    /// Clear all cached entries
    pub fn clear(&self) {
        self.cache.write().unwrap().clear();
    }

    /// Get cache statistics
    pub fn stats(&self) -> (usize, usize) {
        let cache = self.cache.read().unwrap();
        let valid = cache
            .iter()
            .filter(|(_, (time, _))| time.elapsed() < self.ttl)
            .count();
        (valid, cache.len())
    }
}

pub struct QueryEngine {
    db: Arc<CodeGraphDB>,
    cache: Option<Arc<QueryCache>>,
}

impl QueryEngine {
    pub fn new(db: Arc<CodeGraphDB>) -> Self {
        Self { db, cache: None }
    }

    /// Create with cache
    pub fn with_cache(db: Arc<CodeGraphDB>, ttl_secs: u64, max_entries: usize) -> Self {
        Self {
            db,
            cache: Some(Arc::new(QueryCache::new(ttl_secs, max_entries))),
        }
    }

    /// Search for symbols by name (with caching)
    pub fn search(&self, query: &str, limit: usize) -> Result<QueryResult> {
        // Try cache first
        if let Some(ref cache) = self.cache {
            if let Some(result) = cache.get(query) {
                return Ok(result);
            }
        }

        // Query database
        let result = self.db.find_symbols(query, limit)?;

        // Store in cache
        if let Some(ref cache) = self.cache {
            cache.set(query.to_string(), result.clone());
        }

        Ok(result)
    }

    /// Find all functions
    pub fn functions(&self, limit: usize) -> Result<Vec<Symbol>> {
        self.db.find_by_kind(SymbolKind::Function, limit)
    }

    /// Find all structs
    pub fn structs(&self, limit: usize) -> Result<Vec<Symbol>> {
        self.db.find_by_kind(SymbolKind::Struct, limit)
    }

    /// Find all classes
    pub fn classes(&self, limit: usize) -> Result<Vec<Symbol>> {
        self.db.find_by_kind(SymbolKind::Class, limit)
    }

    /// Search by AST pattern (tree-sitter based)
    /// Supported patterns: "function_call", "struct_definition", "import", "method"
    pub fn search_by_pattern(&self, pattern: &str, limit: usize) -> Result<Vec<Symbol>> {
        // Map AST patterns to symbol kinds
        let kind = match pattern {
            "function_call" | "function_definition" => SymbolKind::Function,
            "struct_definition" | "struct" => SymbolKind::Struct,
            "class_definition" | "class" => SymbolKind::Class,
            "enum_definition" | "enum" => SymbolKind::Enum,
            "module_definition" | "module" => SymbolKind::Module,
            "import" | "use_statement" => SymbolKind::Module, // Treat imports as modules
            _ => return Ok(vec![]),
        };

        self.db.find_by_kind(kind, limit)
    }

    /// Find all enums
    pub fn enums(&self, limit: usize) -> Result<Vec<Symbol>> {
        self.db.find_by_kind(SymbolKind::Enum, limit)
    }

    /// Find by file
    pub fn in_file(&self, file_path: &str) -> Result<Vec<Symbol>> {
        // This would need a new db method
        // For now, use search with file path
        self.db.find_symbols(file_path, 1000).map(|r| r.symbols)
    }

    /// Find all symbols in a specific file
    pub fn symbols_in_file(&self, file_path: &str) -> Result<Vec<Symbol>> {
        self.db.find_symbols_in_file(file_path)
    }

    /// Find files that depend on the given file
    pub fn reverse_dependencies(&self, file_path: &str) -> Result<Vec<String>> {
        self.db.find_reverse_dependencies(file_path)
    }

    /// Get all symbols of a specific language
    pub fn by_language(&self, _lang: crate::types::Language, _limit: usize) -> Result<Vec<Symbol>> {
        // Would need a new db method
        Ok(vec![])
    }
}
