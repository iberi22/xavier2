//! Core types for code-graph

use serde::{Deserialize, Serialize};

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

/// A code symbol with location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub id: Option<i64>,
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
