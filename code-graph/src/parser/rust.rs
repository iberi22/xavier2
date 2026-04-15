//! Simplified Rust parser using tree-sitter

use crate::error::{GraphError, Result};
use crate::types::{Language, Symbol, SymbolKind};
use tree_sitter::{Parser, Tree};

pub struct RustParser {
    parser: Parser,
}

impl RustParser {
    pub fn new() -> Self {
        let mut parser = Parser::new();
        let lang = tree_sitter_rust::LANGUAGE.into();
        parser.set_language(&lang).unwrap();
        Self { parser }
    }

    pub fn parse(&mut self, source: &str, file_path: &str) -> Result<Vec<Symbol>> {
        let source_bytes = source.as_bytes();
        let tree = self
            .parser
            .parse(source_bytes, None)
            .ok_or_else(|| GraphError::Parser("Failed to parse Rust source".to_string()))?;

        let mut symbols = Vec::new();
        self.extract_symbols(&tree, source, file_path, &mut symbols);
        Ok(symbols)
    }

    fn extract_symbols(
        &mut self,
        tree: &Tree,
        source: &str,
        file_path: &str,
        symbols: &mut Vec<Symbol>,
    ) {
        let mut cursor = tree.walk();

        for node in tree.root_node().children(&mut cursor) {
            let kind = node.kind();

            match kind {
                "function_item" | "function_declaration" => {
                    if let Some(name_node) = node.child_by_field_name("name") {
                        let start = node.start_position();
                        let end = node.end_position();

                        symbols.push(Symbol {
                            id: None,
                            name: name_node
                                .utf8_text(source.as_bytes())
                                .unwrap_or("?")
                                .to_string(),
                            kind: SymbolKind::Function,
                            lang: Language::Rust,
                            file_path: file_path.to_string(),
                            start_line: (start.row + 1) as u32,
                            end_line: (end.row + 1) as u32,
                            start_col: start.column as u32,
                            end_col: end.column as u32,
                            signature: None,
                            parent: None,
                        });
                    }
                }
                "struct_item" => {
                    if let Some(name_node) = node.child_by_field_name("name") {
                        let start = node.start_position();
                        let end = node.end_position();

                        symbols.push(Symbol {
                            id: None,
                            name: name_node
                                .utf8_text(source.as_bytes())
                                .unwrap_or("?")
                                .to_string(),
                            kind: SymbolKind::Struct,
                            lang: Language::Rust,
                            file_path: file_path.to_string(),
                            start_line: (start.row + 1) as u32,
                            end_line: (end.row + 1) as u32,
                            start_col: start.column as u32,
                            end_col: end.column as u32,
                            signature: None,
                            parent: None,
                        });
                    }
                }
                "enum_item" => {
                    if let Some(name_node) = node.child_by_field_name("name") {
                        let start = node.start_position();
                        let end = node.end_position();

                        symbols.push(Symbol {
                            id: None,
                            name: name_node
                                .utf8_text(source.as_bytes())
                                .unwrap_or("?")
                                .to_string(),
                            kind: SymbolKind::Enum,
                            lang: Language::Rust,
                            file_path: file_path.to_string(),
                            start_line: (start.row + 1) as u32,
                            end_line: (end.row + 1) as u32,
                            start_col: start.column as u32,
                            end_col: end.column as u32,
                            signature: None,
                            parent: None,
                        });
                    }
                }
                "trait_item" => {
                    if let Some(name_node) = node.child_by_field_name("name") {
                        let start = node.start_position();
                        let end = node.end_position();

                        symbols.push(Symbol {
                            id: None,
                            name: name_node
                                .utf8_text(source.as_bytes())
                                .unwrap_or("?")
                                .to_string(),
                            kind: SymbolKind::Trait,
                            lang: Language::Rust,
                            file_path: file_path.to_string(),
                            start_line: (start.row + 1) as u32,
                            end_line: (end.row + 1) as u32,
                            start_col: start.column as u32,
                            end_col: end.column as u32,
                            signature: None,
                            parent: None,
                        });
                    }
                }
                _ => {}
            }
        }
    }
}

impl Default for RustParser {
    fn default() -> Self {
        Self::new()
    }
}
