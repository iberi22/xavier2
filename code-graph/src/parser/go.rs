//! Go parser placeholder
use crate::types::Symbol;

pub struct GoParser;

impl Default for GoParser {
    fn default() -> Self {
        Self::new()
    }
}

impl GoParser {
    pub fn new() -> Self {
        Self
    }
    pub fn parse(&self, _source: &str, _file_path: &str) -> crate::error::Result<Vec<Symbol>> {
        Ok(vec![])
    }
}
