//! Java parser placeholder
use crate::types::Symbol;

pub struct JavaParser;

impl Default for JavaParser {
    fn default() -> Self {
        Self::new()
    }
}

impl JavaParser {
    pub fn new() -> Self {
        Self
    }
    pub fn parse(&self, _source: &str, _file_path: &str) -> crate::error::Result<Vec<Symbol>> {
        Ok(vec![])
    }
}
