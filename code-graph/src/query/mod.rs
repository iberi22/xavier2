//! Query engine for code graph

pub mod tests;

use crate::db::CodeGraphDB;
use crate::error::Result;
use crate::types::{
    CodeEdge, ComplexityHotspot, EdgeType, HubNode, QueryResult, Symbol, SymbolKind,
};
use std::collections::HashMap;
use std::collections::{HashSet, VecDeque};
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
        let cache = self.cache.read().expect("RwLock not poisoned");
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
        let mut cache = self.cache.write().expect("RwLock not poisoned");

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
        self.cache.write().expect("RwLock not poisoned").clear();
    }

    /// Get cache statistics
    pub fn stats(&self) -> (usize, usize) {
        let cache = self.cache.read().expect("RwLock not poisoned");
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

    pub fn dependencies(
        &self,
        query: &str,
        edge_type: Option<EdgeType>,
        depth: usize,
        limit: usize,
    ) -> Result<Vec<CodeEdge>> {
        self.traverse(query, edge_type, depth, limit, false)
    }

    pub fn reverse_dependencies(
        &self,
        query: &str,
        edge_type: Option<EdgeType>,
        depth: usize,
        limit: usize,
    ) -> Result<Vec<CodeEdge>> {
        self.traverse(query, edge_type, depth, limit, true)
    }

    pub fn call_chain(&self, query: &str, depth: usize, limit: usize) -> Result<Vec<CodeEdge>> {
        self.dependencies(query, Some(EdgeType::Calls), depth, limit)
    }

    pub fn hubs(&self, min_degree: u64, limit: usize) -> Result<Vec<HubNode>> {
        self.db.hub_nodes(min_degree, limit)
    }

    pub fn hotspots(&self, min_complexity: f32, limit: usize) -> Result<Vec<ComplexityHotspot>> {
        self.db.complexity_hotspots(min_complexity, limit)
    }

    fn traverse(
        &self,
        query: &str,
        edge_type: Option<EdgeType>,
        depth: usize,
        limit: usize,
        reverse: bool,
    ) -> Result<Vec<CodeEdge>> {
        let start = self.resolve_symbol_id(query)?;
        let Some(start) = start else {
            return Ok(vec![]);
        };

        let max_depth = depth.clamp(1, 8);
        let max_edges = limit.clamp(1, 1000);
        let mut queue = VecDeque::from([(start, 0usize)]);
        let mut seen_nodes = HashSet::new();
        let mut seen_edges = HashSet::new();
        let mut results = Vec::new();

        while let Some((node, current_depth)) = queue.pop_front() {
            if current_depth >= max_depth || results.len() >= max_edges {
                continue;
            }
            if !seen_nodes.insert((node.clone(), current_depth)) {
                continue;
            }

            let edges = if reverse {
                self.db.find_edges_to(&node, edge_type.clone(), max_edges)?
            } else {
                self.db
                    .find_edges_from(&node, edge_type.clone(), max_edges)?
            };

            for edge in edges {
                let edge_key = edge.id.unwrap_or_default();
                if !seen_edges.insert(edge_key) {
                    continue;
                }
                let next = if reverse {
                    edge.from_symbol.clone()
                } else {
                    edge.to_symbol.clone()
                };
                results.push(edge);
                if results.len() >= max_edges {
                    break;
                }
                if !next.starts_with("file:") && !next.starts_with("module:") {
                    queue.push_back((next, current_depth + 1));
                }
            }
        }

        Ok(results)
    }

    fn resolve_symbol_id(&self, query: &str) -> Result<Option<String>> {
        if query.len() == 64 && query.chars().all(|ch| ch.is_ascii_hexdigit()) {
            return Ok(Some(query.to_string()));
        }
        if let Some(symbol) = self.db.find_symbols(query, 1)?.symbols.into_iter().next() {
            return Ok(symbol.stable_id);
        }
        Ok(None)
    }

    /// Find by file
    pub fn in_file(&self, file_path: &str) -> Result<Vec<Symbol>> {
        self.db.find_by_file(file_path)
    }

    /// Get all symbols of a specific language
    pub fn by_language(&self, _lang: crate::types::Language, _limit: usize) -> Result<Vec<Symbol>> {
        // Would need a new db method
        Ok(vec![])
    }
}
