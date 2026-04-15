//! File traversal utilities using std::fs only
//!
//! Provides a lightweight WalkDir alternative without external dependencies.

use std::fs;
use std::path::{Path, PathBuf};

/// Directory entry with path information
#[derive(Debug, Clone)]
pub struct DirEntry {
    pub path: PathBuf,
    pub is_dir: bool,
    pub is_file: bool,
}

/// Recursive directory walker using std::fs only
pub struct WalkDir {
    root: PathBuf,
    exclude_patterns: Vec<String>,
}

impl WalkDir {
    /// Create a new walker for the given root path
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            exclude_patterns: Vec::new(),
        }
    }

    /// Add an exclude pattern (simple contains check)
    pub fn exclude(mut self, pattern: &str) -> Self {
        self.exclude_patterns.push(pattern.to_string());
        self
    }

    /// Returns true if the path should be excluded
    fn should_exclude(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        for pattern in &self.exclude_patterns {
            if path_str.contains(pattern) {
                return true;
            }
        }
        false
    }

    /// Walk the directory tree and collect entries matching the filter
    pub fn walk<F>(&self, filter: F) -> std::io::Result<Vec<DirEntry>>
    where
        F: Fn(&Path) -> bool,
    {
        let mut results = Vec::new();
        let mut stack = vec![self.root.clone()];

        while let Some(current) = stack.pop() {
            if !current.is_dir() {
                continue;
            }

            if self.should_exclude(&current) {
                continue;
            }

            let entries = match fs::read_dir(&current) {
                Ok(e) => e,
                Err(_) => continue,
            };

            for entry in entries.flatten() {
                let path = entry.path();
                if self.should_exclude(&path) {
                    continue;
                }

                if path.is_dir() {
                    stack.push(path.clone());
                    if filter(&path) {
                        results.push(DirEntry {
                            path,
                            is_dir: true,
                            is_file: false,
                        });
                    }
                } else if path.is_file() && filter(&path) {
                    results.push(DirEntry {
                        path,
                        is_dir: false,
                        is_file: true,
                    });
                }
            }
        }

        Ok(results)
    }

    /// Collect all files matching the filter
    pub fn files<F>(&self, filter: F) -> std::io::Result<Vec<PathBuf>>
    where
        F: Fn(&Path) -> bool,
    {
        Ok(self
            .walk(|p| p.is_file() && filter(p))?
            .into_iter()
            .map(|e| e.path)
            .collect())
    }

    /// Collect all directories matching the filter
    pub fn dirs<F>(&self, filter: F) -> std::io::Result<Vec<PathBuf>>
    where
        F: Fn(&Path) -> bool,
    {
        Ok(self
            .walk(|p| p.is_dir() && filter(p))?
            .into_iter()
            .map(|e| e.path)
            .collect())
    }
}

impl Default for WalkDir {
    fn default() -> Self {
        Self::new(".")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_walk_dir_creation() {
        let walker = WalkDir::new("/tmp");
        assert_eq!(walker.root, PathBuf::from("/tmp"));
    }

    #[test]
    fn test_walk_dir_exclude_chain() {
        let walker = WalkDir::new("/tmp").exclude("node_modules").exclude(".git");
        assert_eq!(walker.exclude_patterns.len(), 2);
    }
}
