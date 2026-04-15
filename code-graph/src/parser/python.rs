//! Python parser placeholder
use crate::types::Symbol;

pub struct PythonParser;

impl Default for PythonParser {
    fn default() -> Self {
        Self::new()
    }
}

impl PythonParser {
    pub fn new() -> Self {
        Self
    }
    pub fn parse(&self, _source: &str, _file_path: &str) -> crate::error::Result<Vec<Symbol>> {
        Ok(vec![])
    }
}
