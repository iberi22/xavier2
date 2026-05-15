//! Python parser using tree-sitter.

use crate::error::{GraphError, Result};
use crate::parser::{compact_node_signature, cyclomatic_complexity, PushSymbolArgs};
use crate::types::{Language, Symbol, SymbolKind};
use tree_sitter::{Node, Parser};

pub struct PythonParser {
    parser: Parser,
}

impl Default for PythonParser {
    fn default() -> Self {
        Self::new()
    }
}

impl PythonParser {
    pub fn new() -> Self {
        let mut parser = Parser::new();
        let language = tree_sitter_python::LANGUAGE.into();
        parser
            .set_language(&language)
            .expect("failed to set Python tree-sitter language");
        Self { parser }
    }

    pub fn parse(&mut self, source: &str, file_path: &str) -> Result<Vec<Symbol>> {
        let tree = self
            .parser
            .parse(source.as_bytes(), None)
            .ok_or_else(|| GraphError::Parser("Failed to parse Python source".to_string()))?;
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
            "function_definition" => {
                self.push_named(
                    node,
                    source,
                    file_path,
                    symbols,
                    SymbolKind::Function,
                    parent.clone(),
                );
            }
            "class_definition" => {
                let class_name = self.push_named(
                    node,
                    source,
                    file_path,
                    symbols,
                    SymbolKind::Class,
                    parent.clone(),
                );
                self.extract_children(node, source, file_path, symbols, class_name.or(parent));
                return;
            }
            "import_statement" | "import_from_statement" => {
                self.push_import(node, source, file_path, symbols);
            }
            _ => {}
        }

        self.extract_children(node, source, file_path, symbols, parent);
    }

    fn extract_children(
        &self,
        node: Node,
        source: &str,
        file_path: &str,
        symbols: &mut Vec<Symbol>,
        parent: Option<String>,
    ) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.extract(child, source, file_path, symbols, parent.clone());
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
    ) -> Option<String> {
        let name_node = node.child_by_field_name("name")?;
        let name = name_node.utf8_text(source.as_bytes()).ok()?.to_string();
        self.push_symbol(
            symbols,
            PushSymbolArgs {
                node,
                source,
                language: Language::Python,
                kind,
                file_path,
                name: name.clone(),
                depth: 0,
                parent,
            },
        );
        Some(name)
    }

    fn push_import(&self, node: Node, source: &str, file_path: &str, symbols: &mut Vec<Symbol>) {
        let raw = node.utf8_text(source.as_bytes()).unwrap_or_default();
        let name = raw
            .trim_start_matches("from ")
            .trim_start_matches("import ")
            .split_whitespace()
            .next()
            .unwrap_or(raw)
            .trim()
            .to_string();
        self.push_symbol(
            symbols,
            PushSymbolArgs {
                node,
                source,
                language: Language::Python,
                kind: SymbolKind::Import,
                file_path,
                name,
                depth: 0,
                parent: None,
            },
        );
    }

    fn push_symbol(&self, symbols: &mut Vec<Symbol>, args: PushSymbolArgs<'_>) {
        let start = args.node.start_position();
        let end = args.node.end_position();
        let complexity =
            (args.kind == SymbolKind::Function).then(|| cyclomatic_complexity(args.node, args.source));
        symbols.push(Symbol {
            id: None,
            stable_id: None,
            name: args.name,
            kind: args.kind,
            lang: Language::Python,
            file_path: args.file_path.to_string(),
            start_line: (start.row + 1) as u32,
            end_line: (end.row + 1) as u32,
            start_col: start.column as u32,
            end_col: end.column as u32,
            signature: compact_node_signature(args.node, args.source),
            parent: args.parent,
            complexity,
        });
    }
}
