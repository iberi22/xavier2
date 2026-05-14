//! Parser module - tree-sitter based

use crate::error::Result;
use crate::parser::go::GoParser;
use crate::parser::java::JavaParser;
use crate::parser::python::PythonParser;
use crate::parser::rust::RustParser;
use crate::parser::typescript::TypeScriptParser;
use crate::types::{Language, Symbol};
use tree_sitter::Node;

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
        Language::TypeScript | Language::JavaScript => {
            let mut parser = TypeScriptParser::new(lang.clone());
            parser.parse(source, file_path)
        }
        Language::Python => {
            let mut parser = PythonParser::new();
            parser.parse(source, file_path)
        }
        Language::Go => {
            let mut parser = GoParser::new();
            parser.parse(source, file_path)
        }
        Language::Java => {
            let mut parser = JavaParser::new();
            parser.parse(source, file_path)
        }
        _ => Ok(vec![]),
    }
}

pub(crate) fn compact_node_signature(node: Node, source: &str) -> Option<String> {
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
                .unwrap_or_else(|| raw.lines().next().unwrap_or(raw).to_string()),
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

pub(crate) fn cyclomatic_complexity(node: Node, source: &str) -> f32 {
    fn count(node: Node, source: &str) -> usize {
        let mut total = if matches!(
            node.kind(),
            "if_expression"
                | "if_statement"
                | "elif_clause"
                | "else_if_clause"
                | "while_expression"
                | "while_statement"
                | "for_expression"
                | "for_statement"
                | "for_in_clause"
                | "enhanced_for_statement"
                | "loop_expression"
                | "match_expression"
                | "match_statement"
                | "switch_statement"
                | "case"
                | "conditional_expression"
                | "catch_clause"
                | "except_clause"
        ) {
            1
        } else {
            0
        };

        if matches!(node.kind(), "binary_expression" | "boolean_operator") {
            if let Ok(text) = node.utf8_text(source.as_bytes()) {
                total += text.matches("&&").count();
                total += text.matches("||").count();
                total += text.matches(" and ").count();
                total += text.matches(" or ").count();
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            total += count(child, source);
        }
        total
    }

    1.0 + count(node, source) as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SymbolKind;

    #[test]
    fn parses_typescript_symbols() {
        let symbols = parse_source(
            "import x from 'pkg';\nclass UserService { run() {} }\nfunction main() {}\nconst load = () => main();",
            &Language::TypeScript,
            "app.ts",
        )
        .expect("parse");
        assert!(symbols
            .iter()
            .any(|s| s.name == "UserService" && s.kind == SymbolKind::Class));
        assert!(symbols
            .iter()
            .any(|s| s.name == "main" && s.kind == SymbolKind::Function));
        assert!(symbols.iter().any(|s| s.kind == SymbolKind::Import));
    }

    #[test]
    fn parses_python_symbols() {
        let symbols = parse_source(
            "import os\nclass Service:\n    def run(self):\n        return os.getcwd()\n",
            &Language::Python,
            "app.py",
        )
        .expect("parse");
        assert!(symbols
            .iter()
            .any(|s| s.name == "Service" && s.kind == SymbolKind::Class));
        assert!(symbols
            .iter()
            .any(|s| s.name == "run" && s.kind == SymbolKind::Function));
        assert!(symbols.iter().any(|s| s.kind == SymbolKind::Import));
    }

    #[test]
    fn parses_go_symbols() {
        let symbols = parse_source(
            "package main\nimport \"fmt\"\ntype User struct{}\nfunc main() { fmt.Println(\"x\") }\n",
            &Language::Go,
            "main.go",
        )
        .expect("parse");
        assert!(symbols
            .iter()
            .any(|s| s.name == "User" && s.kind == SymbolKind::Struct));
        assert!(symbols
            .iter()
            .any(|s| s.name == "main" && s.kind == SymbolKind::Function));
        assert!(symbols.iter().any(|s| s.kind == SymbolKind::Import));
    }

    #[test]
    fn parses_java_symbols() {
        let symbols = parse_source(
            "import java.util.List; class Service { void run() {} }",
            &Language::Java,
            "Service.java",
        )
        .expect("parse");
        assert!(symbols
            .iter()
            .any(|s| s.name == "Service" && s.kind == SymbolKind::Class));
        assert!(symbols
            .iter()
            .any(|s| s.name == "run" && s.kind == SymbolKind::Method));
        assert!(symbols.iter().any(|s| s.kind == SymbolKind::Import));
    }
}
