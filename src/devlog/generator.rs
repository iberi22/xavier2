//! DevLog static site generation logic.

use anyhow::Result;
use std::path::Path;

pub struct Generator;

impl Default for Generator {
    fn default() -> Self {
        Self::new()
    }
}

impl Generator {
    pub fn new() -> Self {
        Self
    }

    pub fn build(&self, _source_dir: &Path, _output_dir: &Path) -> Result<()> {
        // TODO: Jules will implement Markdown to HTML logic here
        Ok(())
    }
}
