//! Core types for code-graph

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Programming language supported
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Language {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
    Java,
    C,
    Cpp,
    Unknown,
}

impl Language {
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "rs" => Language::Rust,
            "ts" => Language::TypeScript,
            "tsx" => Language::TypeScript,
            "js" => Language::JavaScript,
            "jsx" => Language::JavaScript,
            "py" => Language::Python,
            "go" => Language::Go,
            "java" => Language::Java,
            "c" | "h" => Language::C,
            "cpp" | "cc" | "cxx" | "hpp" => Language::Cpp,
            _ => Language::Unknown,
        }
    }
}

/// Symbol type in the codebase
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    Function,
    Struct,
    Enum,
    Trait,
    Impl,
    Class,
    Method,
    Variable,
    Constant,
    Import,
    Export,
    Module,
    File,
    Symbol, // Fallback
}

/// Relationship type between indexed code entities.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EdgeType {
    Calls,
    Defines,
    Uses,
    Imports,
    Contains,
    References,
}

impl EdgeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EdgeType::Calls => "Calls",
            EdgeType::Defines => "Defines",
            EdgeType::Uses => "Uses",
            EdgeType::Imports => "Imports",
            EdgeType::Contains => "Contains",
            EdgeType::References => "References",
        }
    }
}

/// A code symbol with location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub id: Option<i64>,
    pub stable_id: Option<String>,
    pub name: String,
    pub kind: SymbolKind,
    pub lang: Language,
    pub file_path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub start_col: u32,
    pub end_col: u32,
    pub signature: Option<String>,
    pub parent: Option<String>, // parent struct/class
    pub complexity: Option<f32>,
}

impl Symbol {
    pub fn deterministic_id(&self, project_id: &str) -> String {
        stable_symbol_id(
            project_id,
            &self.file_path,
            &self.name,
            &format!("{:?}", self.kind),
            self.start_line,
        )
    }

    pub fn stable_key(&self, project_id: &str) -> String {
        self.stable_id
            .clone()
            .unwrap_or_else(|| self.deterministic_id(project_id))
    }
}

/// A graph edge. Endpoints are stable symbol IDs or prefixed pseudo-nodes:
/// `file:<path>` and `module:<name>`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeEdge {
    pub id: Option<i64>,
    pub from_symbol: String,
    pub to_symbol: String,
    pub edge_type: EdgeType,
    pub file_path: String,
    pub line: u32,
    pub confidence: f32,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HubNode {
    pub symbol: Symbol,
    pub incoming: u64,
    pub outgoing: u64,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityHotspot {
    pub symbol: Symbol,
    pub incoming: u64,
    pub outgoing: u64,
    pub risk_score: f32,
}

pub fn stable_symbol_id(
    project_id: &str,
    file_path: &str,
    name: &str,
    kind: &str,
    start_line: u32,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(project_id.as_bytes());
    hasher.update(b"|");
    hasher.update(file_path.as_bytes());
    hasher.update(b"|");
    hasher.update(name.as_bytes());
    hasher.update(b"|");
    hasher.update(kind.as_bytes());
    hasher.update(b"|");
    hasher.update(start_line.to_le_bytes());
    format!("{:x}", hasher.finalize())
}

/// Reference to a symbol (caller/callee)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct Reference {
    pub symbol_id: i64,
    pub file_path: String,
    pub line: u32,
    pub column: u32,
    pub context: String, // surrounding code
}

/// Import/dependency relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct Import {
    pub from: String,
    pub to: String,
    pub file_path: String,
    pub line: u32,
}

/// Indexing statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    pub total_files: u64,
    pub total_symbols: u64,
    pub total_imports: u64,
    pub languages: Vec<LanguageCount>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageCount {
    pub lang: Language,
    pub count: u64,
}

/// Query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub symbols: Vec<Symbol>,
    pub total: usize,
    pub query_time_ms: u64,
}
