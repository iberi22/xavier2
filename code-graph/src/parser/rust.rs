//! Simplified Rust parser using tree-sitter

use crate::error::{GraphError, Result};
use crate::types::{Language, Symbol, SymbolKind};
use tree_sitter::{Node, Parser, Tree};

pub struct RustParser {
    parser: Parser,
}

impl RustParser {
    pub fn new() -> Self {
        let mut parser = Parser::new();
        let lang = tree_sitter_rust::LANGUAGE.into();
        parser.set_language(&lang).expect("failed to set Rust tree-sitter language");
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
        self.extract_symbols_from_node(tree.root_node(), source, file_path, symbols);
    }

    fn extract_symbols_from_node(
        &mut self,
        node: Node,
        source: &str,
        file_path: &str,
        symbols: &mut Vec<Symbol>,
    ) {
        let kind = node.kind();

        match kind {
            "function_item" | "function_declaration" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    self.push_symbol(
                        symbols,
                        node,
                        name_node,
                        source,
                        file_path,
                        SymbolKind::Function,
                    );
                }
            }
            "struct_item" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    self.push_symbol(
                        symbols,
                        node,
                        name_node,
                        source,
                        file_path,
                        SymbolKind::Struct,
                    );
                }
            }
            "enum_item" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    self.push_symbol(
                        symbols,
                        node,
                        name_node,
                        source,
                        file_path,
                        SymbolKind::Enum,
                    );
                }
            }
            "trait_item" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    self.push_symbol(
                        symbols,
                        node,
                        name_node,
                        source,
                        file_path,
                        SymbolKind::Trait,
                    );
                }
            }
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.extract_symbols_from_node(child, source, file_path, symbols);
        }
    }

    fn push_symbol(
        &self,
        symbols: &mut Vec<Symbol>,
        node: Node,
        name_node: Node,
        source: &str,
        file_path: &str,
        kind: SymbolKind,
    ) {
        let start = node.start_position();
        let end = node.end_position();

        symbols.push(Symbol {
            id: None,
            name: name_node
                .utf8_text(source.as_bytes())
                .unwrap_or("?")
                .to_string(),
            kind,
            lang: Language::Rust,
            file_path: file_path.to_string(),
            start_line: (start.row + 1) as u32,
            end_line: (end.row + 1) as u32,
            start_col: start.column as u32,
            end_col: end.column as u32,
            signature: compact_signature(node, source),
            parent: None,
        });
    }
}

fn compact_signature(node: Node, source: &str) -> Option<String> {
    let raw = node.utf8_text(source.as_bytes()).ok()?;
    let header = if let Some(body) = node.child_by_field_name("body") {
        let index = body.start_byte().saturating_sub(node.start_byte());
        format!("{} {{ ... }}", raw.get(..index).unwrap_or(raw))
    } else {
        match raw.find('{') {
            Some(index) => format!("{} {{ ... }}", &raw[..index]),
            None => raw
                .find(';')
                .map(|index| raw[..=index].to_string())
                .unwrap_or_else(|| raw.to_string()),
        }
    };

    let compact = header.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.is_empty() {
        None
    } else if compact.len() > 400 {
        Some(format!(
            "{}...",
            compact.chars().take(400).collect::<String>()
        ))
    } else {
        Some(compact)
    }
}

impl Default for RustParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_compact_function_signature_without_body() {
        let mut parser = RustParser::new();
        let symbols = parser
            .parse(
                r#"
                pub async fn search_filtered(
                    &self,
                    query: &str,
                    limit: usize,
                ) -> Result<Vec<MemoryDocument>> {
                    expensive_body();
                }
                "#,
                "memory.rs",
            )
            .expect("test assertion");

        let symbol = symbols
            .iter()
            .find(|symbol| symbol.name == "search_filtered")
            .expect("test assertion");
        let signature = symbol.signature.as_deref().expect("test assertion");

        assert!(signature.contains("pub async fn search_filtered"));
        assert!(signature.contains("Result<Vec<MemoryDocument>>"));
        assert!(signature.ends_with("{ ... }"));
        assert!(!signature.contains("expensive_body"));
    }

    #[test]
    fn extracts_struct_signature_without_fields() {
        let mut parser = RustParser::new();
        let symbols = parser
            .parse(
                r#"
                pub struct SemanticCache {
                    entries: Vec<String>,
                }
                "#,
                "semantic_cache.rs",
            )
            .expect("test assertion");

        let symbol = symbols
            .iter()
            .find(|symbol| symbol.name == "SemanticCache")
            .expect("test assertion");

        assert_eq!(symbol.kind, SymbolKind::Struct);
        assert_eq!(
            symbol.signature.as_deref(),
            Some("pub struct SemanticCache { ... }")
        );
    }

    #[test]
    fn truncates_long_unicode_signature_on_char_boundary() {
        let long_name = "á".repeat(450);
        let source = format!("pub fn {long_name}() {{}}");
        let mut parser = RustParser::new();
        let symbols = parser.parse(&source, "unicode.rs").expect("test assertion");
        let signature = symbols[0].signature.as_deref().expect("test assertion");

        assert!(signature.ends_with("..."));
        assert!(signature.is_char_boundary(signature.len()));
    }
}
