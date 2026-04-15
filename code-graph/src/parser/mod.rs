//! Parser module - tree-sitter based

use crate::error::Result;
use crate::parser::rust::RustParser;
use crate::types::{Language, Symbol};

pub mod go;
pub mod java;
pub mod python;
pub mod rust;
pub mod typescript;

/// Parse source code using tree-sitter
pub fn parse_source(source: &str, lang: &Language, file_path: &str) -> Result<Vec<Symbol>> {
    match lang {
        Language::Rust => {
            let mut parser = RustParser::new();
            parser.parse(source, file_path)
        }
        // TODO: Implement other languages
        _ => {
            // Fallback - return empty for now
            Ok(vec![])
        }
    }
}
