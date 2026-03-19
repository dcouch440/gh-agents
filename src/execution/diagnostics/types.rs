//! Shared types for the diagnostics engine.

use std::path::PathBuf;

/// A single file change observed in the overlay upper directory.
#[derive(Debug, Clone)]
pub struct FileChange {
    /// Relative path within the workspace.
    pub path: PathBuf,
    /// What kind of change occurred.
    pub change_type: ChangeType,
    /// File size in bytes after the change (0 for deletions).
    pub size: u64,
}

/// Classification of a filesystem change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeType {
    Created,
    Modified,
    Deleted,
}

impl std::fmt::Display for ChangeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChangeType::Created => write!(f, "created"),
            ChangeType::Modified => write!(f, "modified"),
            ChangeType::Deleted => write!(f, "deleted"),
        }
    }
}
