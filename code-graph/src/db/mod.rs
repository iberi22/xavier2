//! SQLite database for storing code graph

pub mod benchmarks;

use crate::error::{GraphError, Result};
use crate::types::{IndexStats, Language, LanguageCount, QueryResult, Symbol, SymbolKind};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex;
use tracing::{debug, info};

pub struct CodeGraphDB {
    conn: Mutex<Connection>,
}

fn parse_language(value: &str) -> Language {
    serde_json::from_str(value).unwrap_or(match value {
        "Rust" => Language::Rust,
        "TypeScript" => Language::TypeScript,
        "JavaScript" => Language::JavaScript,
        "Python" => Language::Python,
        "Go" => Language::Go,
        "Java" => Language::Java,
        "C" => Language::C,
        "Cpp" => Language::Cpp,
        _ => Language::Unknown,
    })
}

fn parse_symbol_kind(value: &str) -> SymbolKind {
    serde_json::from_str(value).unwrap_or(match value {
        "Function" => SymbolKind::Function,
        "Struct" => SymbolKind::Struct,
        "Enum" => SymbolKind::Enum,
        "Trait" => SymbolKind::Trait,
        "Impl" => SymbolKind::Impl,
        "Class" => SymbolKind::Class,
        "Method" => SymbolKind::Method,
        "Variable" => SymbolKind::Variable,
        "Constant" => SymbolKind::Constant,
        "Import" => SymbolKind::Import,
        "Export" => SymbolKind::Export,
        "Module" => SymbolKind::Module,
        "File" => SymbolKind::File,
        _ => SymbolKind::Symbol,
    })
}

impl CodeGraphDB {
    /// Open or create a database at the given path
    pub fn new(path: &Path) -> Result<Self> {
        info!("Opening database at {:?}", path);

        let conn = Connection::open(path).map_err(|e| GraphError::Database(e.to_string()))?;

        let db = Self {
            conn: Mutex::new(conn),
        };

        db.init_schema()?;
        Ok(db)
    }

    /// Create a new database (overwrite if exists)
    pub fn create_new(path: &Path) -> Result<Self> {
        info!("Creating NEW database at {:?}", path);

        // Remove existing file if present
        if path.exists() {
            std::fs::remove_file(path).map_err(|e| GraphError::Database(e.to_string()))?;
        }

        let conn = Connection::open(path).map_err(|e| GraphError::Database(e.to_string()))?;

        let db = Self {
            conn: Mutex::new(conn),
        };

        db.init_schema()?;
        Ok(db)
    }

    /// Create an in-memory database
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory().map_err(|e| GraphError::Database(e.to_string()))?;

        let db = Self {
            conn: Mutex::new(conn),
        };

        db.init_schema()?;
        Ok(db)
    }

    /// Initialize database schema
    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS symbols (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                kind TEXT NOT NULL,
                lang TEXT NOT NULL,
                file_path TEXT NOT NULL,
                start_line INTEGER NOT NULL,
                end_line INTEGER NOT NULL,
                start_col INTEGER NOT NULL,
                end_col INTEGER NOT NULL,
                signature TEXT,
                parent TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE INDEX IF NOT EXISTS idx_symbols_name ON symbols(name);
            CREATE INDEX IF NOT EXISTS idx_symbols_kind ON symbols(kind);
            CREATE INDEX IF NOT EXISTS idx_symbols_lang ON symbols(lang);
            CREATE INDEX IF NOT EXISTS idx_symbols_file ON symbols(file_path);

            CREATE TABLE IF NOT EXISTS refs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                symbol_id INTEGER NOT NULL,
                file_path TEXT NOT NULL,
                line INTEGER NOT NULL,
                col INTEGER NOT NULL,
                context TEXT,
                FOREIGN KEY (symbol_id) REFERENCES symbols(id)
            );

            CREATE INDEX IF NOT EXISTS idx_refs_symbol ON refs(symbol_id);

            CREATE TABLE IF NOT EXISTS imports (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                from_path TEXT NOT NULL,
                to_path TEXT NOT NULL,
                file_path TEXT NOT NULL,
                line INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_imports_file ON imports(file_path);

            CREATE TABLE IF NOT EXISTS metadata (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            "#,
        )
        .map_err(|e| GraphError::Database(e.to_string()))?;

        info!("Database schema initialized");
        Ok(())
    }

    /// Insert a symbol
    pub fn insert_symbol(&self, symbol: &Symbol) -> Result<i64> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            r#"INSERT INTO symbols (name, kind, lang, file_path, start_line, end_line, start_col, end_col, signature, parent)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"#,
            params![
                symbol.name,
                format!("{:?}", symbol.kind),
                format!("{:?}", symbol.lang),
                symbol.file_path,
                symbol.start_line,
                symbol.end_line,
                symbol.start_col,
                symbol.end_col,
                symbol.signature,
                symbol.parent,
            ],
        )
        .map_err(|e| GraphError::Database(e.to_string()))?;

        Ok(conn.last_insert_rowid())
    }

    /// Insert multiple symbols in a batch
    pub fn insert_symbols(&self, symbols: &[Symbol]) -> Result<()> {
        let mut conn = self.conn.lock().unwrap();

        let tx = conn
            .transaction()
            .map_err(|e| GraphError::Database(e.to_string()))?;

        {
            let mut stmt = tx
                .prepare(
                    r#"INSERT INTO symbols (name, kind, lang, file_path, start_line, end_line, start_col, end_col, signature, parent)
                       VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"#,
                )
                .map_err(|e| GraphError::Database(e.to_string()))?;

            for symbol in symbols {
                stmt.execute(params![
                    symbol.name,
                    format!("{:?}", symbol.kind),
                    format!("{:?}", symbol.lang),
                    symbol.file_path,
                    symbol.start_line,
                    symbol.end_line,
                    symbol.start_col,
                    symbol.end_col,
                    symbol.signature,
                    symbol.parent,
                ])
                .map_err(|e| GraphError::Database(e.to_string()))?;
            }
        }

        tx.commit()
            .map_err(|e| GraphError::Database(e.to_string()))?;

        debug!("Inserted {} symbols", symbols.len());
        Ok(())
    }

    /// Calculate search score for ranking results
    /// exact = 10, prefix = 5, fuzzy = 1, bonus for public/exports
    fn calculate_score(symbol_name: &str, query: &str) -> i32 {
        let name_lower = symbol_name.to_lowercase();
        let query_lower = query.to_lowercase();

        // Exact match (case insensitive)
        if name_lower == query_lower {
            return 10;
        }

        // Prefix match
        if name_lower.starts_with(&query_lower) {
            return 5;
        }

        // Contains match
        if name_lower.contains(&query_lower) {
            return 1;
        }

        // Fuzzy - check if all chars exist in order
        let mut query_chars = query_lower.chars().peekable();
        for c in name_lower.chars() {
            if query_chars.peek() == Some(&c) {
                query_chars.next();
            }
        }
        if query_chars.peek().is_none() {
            return 1;
        }

        0
    }

    /// Find symbols by name with hybrid ranking
    pub fn find_symbols(&self, query: &str, limit: usize) -> Result<QueryResult> {
        let start = std::time::Instant::now();
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn
            .prepare(
                r#"SELECT id, name, kind, lang, file_path, start_line, end_line, start_col, end_col, signature, parent
                   FROM symbols
                   WHERE name LIKE ?1"#,
            )
            .map_err(|e| GraphError::Database(e.to_string()))?;

        let pattern = format!("%{}%", query);
        let mut symbols: Vec<Symbol> = stmt
            .query_map(params![pattern], |row| {
                Ok(Symbol {
                    id: Some(row.get(0)?),
                    name: row.get(1)?,
                    kind: parse_symbol_kind(&row.get::<_, String>(2)?),
                    lang: parse_language(&row.get::<_, String>(3)?),
                    file_path: row.get(4)?,
                    start_line: row.get(5)?,
                    end_line: row.get(6)?,
                    start_col: row.get(7)?,
                    end_col: row.get(8)?,
                    signature: row.get(9)?,
                    parent: row.get(10)?,
                })
            })
            .map_err(|e| GraphError::Database(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        // Apply scoring and ranking without mutating semantic fields.
        if !query.is_empty() {
            symbols.sort_by(|a, b| {
                let score_for = |symbol: &Symbol| {
                    let score = Self::calculate_score(&symbol.name, query);
                    let bonus = match symbol.kind {
                        SymbolKind::Function | SymbolKind::Struct => 1,
                        _ => 0,
                    };
                    score + bonus
                };
                score_for(b).cmp(&score_for(a))
            });
        }

        // Apply limit
        symbols.truncate(limit);

        let total = symbols.len();
        let query_time_ms = start.elapsed().as_millis() as u64;

        Ok(QueryResult {
            symbols,
            total,
            query_time_ms,
        })
    }

    /// Find symbols in a specific file
    pub fn find_by_file(&self, file_path: &str) -> Result<Vec<Symbol>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn
            .prepare(
                r#"SELECT id, name, kind, lang, file_path, start_line, end_line, start_col, end_col, signature, parent
                   FROM symbols
                   WHERE file_path = ?1"#,
            )
            .map_err(|e| GraphError::Database(e.to_string()))?;

        let symbols = stmt
            .query_map(params![file_path], |row| {
                Ok(Symbol {
                    id: Some(row.get(0)?),
                    name: row.get(1)?,
                    kind: parse_symbol_kind(&row.get::<_, String>(2)?),
                    lang: parse_language(&row.get::<_, String>(3)?),
                    file_path: row.get(4)?,
                    start_line: row.get(5)?,
                    end_line: row.get(6)?,
                    start_col: row.get(7)?,
                    end_col: row.get(8)?,
                    signature: row.get(9)?,
                    parent: row.get(10)?,
                })
            })
            .map_err(|e| GraphError::Database(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(symbols)
    }

    /// Find symbols by kind
    pub fn find_by_kind(&self, kind: SymbolKind, limit: usize) -> Result<Vec<Symbol>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn
            .prepare(
                r#"SELECT id, name, kind, lang, file_path, start_line, end_line, start_col, end_col, signature, parent
                   FROM symbols
                   WHERE kind = ?1
                   LIMIT ?2"#,
            )
            .map_err(|e| GraphError::Database(e.to_string()))?;

        let kind_str = format!("{:?}", kind);
        let symbols = stmt
            .query_map(params![kind_str, limit], |row| {
                Ok(Symbol {
                    id: Some(row.get(0)?),
                    name: row.get(1)?,
                    kind: parse_symbol_kind(&row.get::<_, String>(2)?),
                    lang: parse_language(&row.get::<_, String>(3)?),
                    file_path: row.get(4)?,
                    start_line: row.get(5)?,
                    end_line: row.get(6)?,
                    start_col: row.get(7)?,
                    end_col: row.get(8)?,
                    signature: row.get(9)?,
                    parent: row.get(10)?,
                })
            })
            .map_err(|e| GraphError::Database(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(symbols)
    }

    /// Get statistics
    pub fn stats(&self) -> Result<IndexStats> {
        let conn = self.conn.lock().unwrap();

        let total_files: u64 = conn
            .query_row("SELECT COUNT(DISTINCT file_path) FROM symbols", [], |row| {
                row.get(0)
            })
            .unwrap_or(0);

        let total_symbols: u64 = conn
            .query_row("SELECT COUNT(*) FROM symbols", [], |row| row.get(0))
            .unwrap_or(0);

        let total_imports: u64 = conn
            .query_row("SELECT COUNT(*) FROM imports", [], |row| row.get(0))
            .unwrap_or(0);

        let mut stmt = conn
            .prepare("SELECT lang, COUNT(*) FROM symbols GROUP BY lang")
            .map_err(|e| GraphError::Database(e.to_string()))?;

        let languages = stmt
            .query_map([], |row| {
                let lang_str: String = row.get(0)?;
                let count: u64 = row.get(1)?;
                Ok(LanguageCount {
                    lang: parse_language(&lang_str),
                    count,
                })
            })
            .map_err(|e| GraphError::Database(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(IndexStats {
            total_files,
            total_symbols,
            total_imports,
            languages,
            duration_ms: 0,
        })
    }

    /// Clear all data
    pub fn clear(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute_batch(
            r#"
            DELETE FROM refs;
            DELETE FROM imports;
            DELETE FROM symbols;
            "#,
        )
        .map_err(|e| GraphError::Database(e.to_string()))?;

        info!("Database cleared");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_text_language_values_from_sqlite() {
        assert_eq!(parse_language("Rust"), Language::Rust);
        assert_eq!(parse_language("TypeScript"), Language::TypeScript);
        assert_eq!(parse_language("unknown-value"), Language::Unknown);
    }

    #[test]
    fn parses_plain_text_symbol_kind_values_from_sqlite() {
        assert_eq!(parse_symbol_kind("Struct"), SymbolKind::Struct);
        assert_eq!(parse_symbol_kind("Function"), SymbolKind::Function);
        assert_eq!(parse_symbol_kind("unknown-value"), SymbolKind::Symbol);
    }
}
