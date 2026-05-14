//! Go parser using tree-sitter.

use crate::error::{GraphError, Result};
use crate::parser::{compact_node_signature, cyclomatic_complexity};
use crate::types::{Language, Symbol, SymbolKind};
use tree_sitter::{Node, Parser};

pub struct GoParser {
    parser: Parser,
}

impl Default for GoParser {
    fn default() -> Self {
        Self::new()
    }
}

impl GoParser {
    pub fn new() -> Self {
        let mut parser = Parser::new();
        let language = tree_sitter_go::LANGUAGE.into();
        parser
            .set_language(&language)
            .expect("failed to set Go tree-sitter language");
        Self { parser }
    }

    pub fn parse(&mut self, source: &str, file_path: &str) -> Result<Vec<Symbol>> {
        let tree = self
            .parser
            .parse(source.as_bytes(), None)
            .ok_or_else(|| GraphError::Parser("Failed to parse Go source".to_string()))?;
        let mut symbols = Vec::new();
        self.extract(tree.root_node(), source, file_path, &mut symbols, None);
        Ok(symbols)
    }

    fn extract(
        &self,
        node: Node,
        source: &str,
        file_path: &str,
        symbols: &mut Vec<Symbol>,
        parent: Option<String>,
    ) {
        match node.kind() {
            "function_declaration" => {
                self.push_named(
                    node,
                    source,
                    file_path,
                    symbols,
                    SymbolKind::Function,
                    parent.clone(),
                );
            }
            "method_declaration" => {
                self.push_named(
                    node,
                    source,
                    file_path,
                    symbols,
                    SymbolKind::Method,
                    parent.clone(),
                );
            }
            "type_declaration" => {
                self.extract_type_specs(node, source, file_path, symbols);
            }
            "import_declaration" | "import_spec" => {
                self.push_import(node, source, file_path, symbols);
            }
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.extract(child, source, file_path, symbols, parent.clone());
        }
    }

    fn extract_type_specs(
        &self,
        node: Node,
        source: &str,
        file_path: &str,
        symbols: &mut Vec<Symbol>,
    ) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() != "type_spec" {
                continue;
            }
            if let Some(name_node) = child.child_by_field_name("name") {
                if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                    let kind = child
                        .child_by_field_name("type")
                        .map(|node| match node.kind() {
                            "struct_type" => SymbolKind::Struct,
                            "interface_type" => SymbolKind::Trait,
                            _ => SymbolKind::Symbol,
                        })
                        .unwrap_or(SymbolKind::Symbol);
                    self.push_symbol(
                        child,
                        source,
                        file_path,
                        symbols,
                        name.to_string(),
                        kind,
                        None,
                    );
                }
            }
        }
    }

    fn push_named(
        &self,
        node: Node,
        source: &str,
        file_path: &str,
        symbols: &mut Vec<Symbol>,
        kind: SymbolKind,
        parent: Option<String>,
    ) {
        if let Some(name_node) = node.child_by_field_name("name") {
            if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                self.push_symbol(
                    node,
                    source,
                    file_path,
                    symbols,
                    name.to_string(),
                    kind,
                    parent,
                );
            }
        }
    }

    fn push_import(&self, node: Node, source: &str, file_path: &str, symbols: &mut Vec<Symbol>) {
        let raw = node.utf8_text(source.as_bytes()).unwrap_or_default();
        for part in raw.split('"').skip(1).step_by(2) {
            self.push_symbol(
                node,
                source,
                file_path,
                symbols,
                part.to_string(),
                SymbolKind::Import,
                None,
            );
        }
    }

    fn push_symbol(
        &self,
        node: Node,
        source: &str,
        file_path: &str,
        symbols: &mut Vec<Symbol>,
        name: String,
        kind: SymbolKind,
        parent: Option<String>,
    ) {
        let start = node.start_position();
        let end = node.end_position();
        let complexity = matches!(kind, SymbolKind::Function | SymbolKind::Method)
            .then(|| cyclomatic_complexity(node, source));
        symbols.push(Symbol {
            id: None,
            stable_id: None,
            name,
            kind,
            lang: Language::Go,
            file_path: file_path.to_string(),
            start_line: (start.row + 1) as u32,
            end_line: (end.row + 1) as u32,
            start_col: start.column as u32,
            end_col: end.column as u32,
            signature: compact_node_signature(node, source),
            parent,
            complexity,
        });
    }
}
