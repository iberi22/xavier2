//! Tests for code-graph query engine

#[cfg(test)]
mod tests_inner {
    use crate::db::CodeGraphDB;
    use crate::types::{Language, Symbol, SymbolKind};

    /// Create a test database with sample symbols
    fn setup_test_db() -> CodeGraphDB {
        let db = CodeGraphDB::in_memory().expect("test assertion");

        // Insert test symbols
        let sym1 = Symbol {
            id: None,
            name: "main".to_string(),
            kind: SymbolKind::Function,
            lang: Language::Rust,
            file_path: "/src/main.rs".to_string(),
            start_line: 1,
            end_line: 10,
            start_col: 0,
            end_col: 0,
            signature: Some("fn main()".to_string()),
            parent: None,
        };
        db.insert_symbol(&sym1).expect("test assertion");

        let sym2 = Symbol {
            id: None,
            name: "process_data".to_string(),
            kind: SymbolKind::Function,
            lang: Language::Rust,
            file_path: "/src/processor.rs".to_string(),
            start_line: 5,
            end_line: 20,
            start_col: 0,
            end_col: 0,
            signature: Some("fn process_data(data: String) -> Result<()>".to_string()),
            parent: None,
        };
        db.insert_symbol(&sym2).expect("test assertion");

        let sym3 = Symbol {
            id: None,
            name: "User".to_string(),
            kind: SymbolKind::Struct,
            lang: Language::Rust,
            file_path: "/src/models.rs".to_string(),
            start_line: 1,
            end_line: 15,
            start_col: 0,
            end_col: 0,
            signature: Some("struct User { name: String }".to_string()),
            parent: None,
        };
        db.insert_symbol(&sym3).expect("test assertion");

        let sym4 = Symbol {
            id: None,
            name: "calculate_total".to_string(),
            kind: SymbolKind::Function,
            lang: Language::TypeScript,
            file_path: "/src/calc.ts".to_string(),
            start_line: 10,
            end_line: 25,
            start_col: 0,
            end_col: 0,
            signature: Some("function calculateTotal(items: Item[]): number".to_string()),
            parent: None,
        };
        db.insert_symbol(&sym4).expect("test assertion");

        db
    }

    #[test]
    fn test_insert_and_find_symbol() {
        let db = setup_test_db();

        // Test exact match
        let result = db.find_symbols("main", 10).expect("test assertion");
        assert!(!result.symbols.is_empty());
        assert_eq!(result.symbols[0].name, "main");
    }

    #[test]
    fn test_fuzzy_search() {
        let db = setup_test_db();

        // Test partial match
        let result = db.find_symbols("process", 10).expect("test assertion");
        assert!(!result.symbols.is_empty());
        assert!(result.symbols[0].name.contains("process"));
    }

    #[test]
    fn test_case_insensitive() {
        let db = setup_test_db();

        // Test case insensitive
        let result = db.find_symbols("MAIN", 10).expect("test assertion");
        assert!(!result.symbols.is_empty());
    }

    #[test]
    fn test_find_by_kind() {
        let db = setup_test_db();

        // Find all functions
        let functions = db.find_by_kind(SymbolKind::Function, 100).expect("test assertion");
        assert_eq!(functions.len(), 3); // main, process_data, calculate_total

        // Find all structs
        let structs = db.find_by_kind(SymbolKind::Struct, 100).expect("test assertion");
        assert_eq!(structs.len(), 1); // User
    }

    #[test]
    fn test_empty_query() {
        let db = setup_test_db();

        // Empty query should return some results
        let result = db.find_symbols("", 10).expect("test assertion");
        assert!(!result.symbols.is_empty());
    }

    #[test]
    fn test_no_results() {
        let db = setup_test_db();

        let result = db.find_symbols("nonexistent_symbol_xyz", 10).expect("test assertion");
        assert!(result.symbols.is_empty());
    }

    #[test]
    fn test_limit() {
        let db = setup_test_db();

        let result = db.find_symbols("", 2).expect("test assertion");
        assert!(result.symbols.len() <= 2);
    }

    #[test]
    #[allow(unused_comparisons)]
    #[allow(clippy::absurd_extreme_comparisons)]
    fn test_query_result_metadata() {
        let db = setup_test_db();

        let result = db.find_symbols("main", 10).expect("test assertion");

        assert!(result.total > 0);
        assert!(result.query_time_ms >= 0, "query time should be recorded");
    }
}
