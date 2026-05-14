//! TypeScript and JavaScript parser using tree-sitter.

use crate::error::{GraphError, Result};
use crate::parser::{compact_node_signature, cyclomatic_complexity};
use crate::types::{Language, Symbol, SymbolKind};
use tree_sitter::{Node, Parser};

pub struct TypeScriptParser {
    parser: Parser,
    lang: Language,
}

impl Default for TypeScriptParser {
    fn default() -> Self {
        Self::new(Language::TypeScript)
    }
}

impl TypeScriptParser {
    pub fn new(lang: Language) -> Self {
        let mut parser = Parser::new();
        let grammar = tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into();
        parser
            .set_language(&grammar)
            .expect("failed to set TypeScript tree-sitter language");
        Self { parser, lang }
    }

    pub fn parse(&mut self, source: &str, file_path: &str) -> Result<Vec<Symbol>> {
        let tree = self
            .parser
            .parse(source.as_bytes(), None)
            .ok_or_else(|| GraphError::Parser("Failed to parse TypeScript source".to_string()))?;
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
            "function_declaration" | "generator_function_declaration" => {
                self.push_named(
                    node,
                    source,
                    file_path,
                    symbols,
                    SymbolKind::Function,
                    parent.clone(),
                );
            }
            "method_definition" | "public_field_definition" => {
                self.push_named(
                    node,
                    source,
                    file_path,
                    symbols,
                    SymbolKind::Method,
                    parent.clone(),
                );
            }
            "class_declaration" => {
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
            "interface_declaration" | "type_alias_declaration" => {
                self.push_named(
                    node,
                    source,
                    file_path,
                    symbols,
                    SymbolKind::Struct,
                    parent.clone(),
                );
            }
            "lexical_declaration" | "variable_declaration" => {
                self.extract_variable_functions(node, source, file_path, symbols, parent.clone());
            }
            "import_statement" => {
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
        self.push_symbol(node, source, file_path, symbols, name.clone(), kind, parent);
        Some(name)
    }

    fn extract_variable_functions(
        &self,
        node: Node,
        source: &str,
        file_path: &str,
        symbols: &mut Vec<Symbol>,
        parent: Option<String>,
    ) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() != "variable_declarator" {
                continue;
            }
            let value_kind = child
                .child_by_field_name("value")
                .map(|value| value.kind().to_string())
                .unwrap_or_default();
            if value_kind != "arrow_function" && value_kind != "function" {
                continue;
            }
            if let Some(name_node) = child.child_by_field_name("name") {
                if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                    self.push_symbol(
                        child,
                        source,
                        file_path,
                        symbols,
                        name.to_string(),
                        SymbolKind::Function,
                        parent.clone(),
                    );
                }
            }
        }
    }

    fn push_import(&self, node: Node, source: &str, file_path: &str, symbols: &mut Vec<Symbol>) {
        let raw = node.utf8_text(source.as_bytes()).unwrap_or_default();
        let name = raw
            .split(['"', '\''])
            .nth(1)
            .unwrap_or(raw)
            .trim()
            .to_string();
        self.push_symbol(
            node,
            source,
            file_path,
            symbols,
            name,
            SymbolKind::Import,
            None,
        );
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
            lang: self.lang.clone(),
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
