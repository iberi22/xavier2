//! CLI debug commands for code-graph

use crate::db::CodeGraphDB;
use crate::types::SymbolKind;
use std::path::Path;

/// CLI tool for debugging code-graph
pub struct DebugCLI {
    db: CodeGraphDB,
}

impl DebugCLI {
    pub fn new(db: CodeGraphDB) -> Self {
        Self { db }
    }

    /// Print all indexed files
    pub fn list_files(&self) -> Result<Vec<String>, String> {
        self.db
            .stats()
            .map(|s| {
                s.languages
                    .iter()
                    .map(|l| format!("{:?}: {}", l.lang, l.count))
                    .collect()
            })
            .map_err(|e| e.to_string())
    }

    /// Print all symbols of a kind
    pub fn list_symbols(&self, kind: Option<String>) -> Result<Vec<String>, String> {
        let kind = match kind.as_deref() {
            Some("function") => SymbolKind::Function,
            Some("struct") => SymbolKind::Struct,
            Some("class") => SymbolKind::Class,
            Some("enum") => SymbolKind::Enum,
            Some("module") => SymbolKind::Module,
            _ => SymbolKind::Function,
        };

        let symbols = self
            .db
            .find_by_kind(kind, 1000)
            .map_err(|e| e.to_string())?;

        Ok(symbols
            .iter()
            .map(|s| format!("{}:{} - {}", s.file_path, s.start_line, s.name))
            .collect())
    }

    /// Export graph to DOT format
    pub fn export_dot(&self, output: &Path) -> Result<(), String> {
        let symbols = self
            .db
            .find_by_kind(SymbolKind::Function, 10000)
            .map_err(|e| e.to_string())?;

        let mut dot = String::from("digraph code_graph {\n");
        for sym in &symbols {
            dot.push_str(&format!("  \"{}\" [label=\"{}\"];\n", sym.name, sym.name));
        }
        dot.push_str("}\n");

        std::fs::write(output, dot).map_err(|e| e.to_string())
    }

    /// Verify index integrity
    pub fn verify(&self) -> Result<String, String> {
        let stats = self.db.stats().map_err(|e| e.to_string())?;

        if stats.total_symbols == 0 {
            return Ok("Index is empty".to_string());
        }

        if stats.total_files == 0 {
            return Err("Index has symbols but no files!".to_string());
        }

        Ok(format!(
            "✓ Index OK: {} files, {} symbols, {} imports",
            stats.total_files, stats.total_symbols, stats.total_imports
        ))
    }
}
