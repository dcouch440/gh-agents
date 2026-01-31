//! Live Context Graph — background repo indexing for agent context injection.
//!
//! The indexer scans source files using Haiku to build summaries and symbol maps.
//! The compiler selects relevant context for a given task and injects it into TaskContext.

pub mod compiler;
pub mod indexer;

use std::collections::HashMap;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

/// A single indexed file entry.
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: String,
    pub summary: String,
    pub symbols: Vec<Symbol>,
    pub size_bytes: u64,
    pub last_modified: SystemTime,
}

/// A symbol extracted from a source file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub name: String,
    pub kind: String,
    pub line: u32,
}

/// The full in-memory repo index.
#[derive(Debug, Clone, Default)]
pub struct RepoIndex {
    pub files: HashMap<String, FileEntry>,
    /// symbol name (lowercased) -> list of file paths containing it
    pub symbol_map: HashMap<String, Vec<String>>,
    /// Compact one-line-per-file tree summary
    pub tree_summary: String,
    /// Whether the initial indexing pass is complete
    pub ready: bool,
}

/// Status of the repo indexing process.
#[derive(Debug, Clone, serde::Serialize)]
pub struct IndexingStatus {
    pub state: IndexingState,
    pub files_total: usize,
    pub files_indexed: usize,
    pub last_completed: Option<String>,
    pub error: Option<String>,
}

impl Default for IndexingStatus {
    fn default() -> Self {
        Self {
            state: IndexingState::Idle,
            files_total: 0,
            files_indexed: 0,
            last_completed: None,
            error: None,
        }
    }
}

/// State of the indexing process.
#[derive(Debug, Clone, serde::Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum IndexingState {
    Idle,
    Running,
    Complete,
    Failed,
}

/// Compiled context ready for injection into TaskContext.
#[derive(Debug, Clone)]
pub struct CompiledContext {
    pub briefing: String,
    pub relevant_files: Vec<(String, String)>, // (path, content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_index_default_not_ready() {
        let idx = RepoIndex::default();
        assert!(!idx.ready);
        assert!(idx.files.is_empty());
    }

    #[test]
    fn file_entry_clone() {
        let entry = FileEntry {
            path: "src/main.rs".into(),
            summary: "Entry point".into(),
            symbols: vec![Symbol {
                name: "main".into(),
                kind: "Function".into(),
                line: 1,
            }],
            size_bytes: 100,
            last_modified: SystemTime::now(),
        };
        let cloned = entry.clone();
        assert_eq!(cloned.path, "src/main.rs");
        assert_eq!(cloned.symbols.len(), 1);
    }
}
