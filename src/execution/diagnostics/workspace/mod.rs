//! Workspace tracking — snapshots, diffs, and digests across commands.

pub mod digest;
pub mod snapshot;

mod tests;

use std::path::{Path, PathBuf};

use digest::WorkspaceDigest;
use snapshot::{FileMetadata, UpperDirSnapshot};

use super::types::{ChangeType, FileChange};

/// Tracks workspace state across commands within an agent's lifecycle.
#[derive(Default)]
pub struct WorkspaceTracker {
    prev_file_count: usize,
    prev_dir_count: usize,
    initialized: bool,
}

/// Paths that are workspace machinery rather than agent output.
///
/// `diff()` reports every file the container touched — pip installs, bytecode
/// caches, git internals, scratch. None of that belongs in a downstream
/// agent's orientation block.
pub fn is_noise(path: &std::path::Path) -> bool {
    const NOISE_DIRS: [&str; 3] = ["node_modules", "__pycache__", "site-packages"];

    path.components().any(|c| {
        let part = c.as_os_str().to_string_lossy();
        part.starts_with('.') || NOISE_DIRS.contains(&part.as_ref())
    })
}

impl WorkspaceTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Initialize baseline counts from the first pre-command snapshot.
    /// Prevents the first digest from showing `+1486` when the workspace
    /// already has files from prior steps.
    pub fn initialize(&mut self, before: &UpperDirSnapshot) {
        if !self.initialized {
            self.prev_file_count = before.file_count();
            self.prev_dir_count = before.dir_count();
            self.initialized = true;
        }
    }

    /// Compute file changes between two snapshots.
    pub fn diff(before: &UpperDirSnapshot, after: &UpperDirSnapshot) -> Vec<FileChange> {
        let mut changes = Vec::new();

        // Files in `after` but not in `before` → Created
        // Files in both but changed (mtime or size differ) → Modified
        // Whiteout entries (type 'c') → Deleted
        for (path, meta) in &after.entries {
            if meta.file_type == 'c' {
                // OverlayFS whiteout = deletion marker
                let deleted_path = strip_whiteout_prefix(path);
                changes.push(FileChange {
                    path: deleted_path,
                    change_type: ChangeType::Deleted,
                    size: 0,
                });
                continue;
            }

            // Skip directories — we only report file-level changes
            if meta.file_type != 'f' {
                continue;
            }

            match before.entries.get(path) {
                None => {
                    changes.push(FileChange {
                        path: path.clone(),
                        change_type: ChangeType::Created,
                        size: meta.size,
                    });
                }
                Some(prev) => {
                    if file_changed(prev, meta) {
                        changes.push(FileChange {
                            path: path.clone(),
                            change_type: ChangeType::Modified,
                            size: meta.size,
                        });
                    }
                }
            }
        }

        // Files in `before` but not in `after` → Deleted
        // (This catches files removed without whiteout markers)
        for (path, meta) in &before.entries {
            if meta.file_type == 'f' && !after.entries.contains_key(path) {
                changes.push(FileChange {
                    path: path.clone(),
                    change_type: ChangeType::Deleted,
                    size: 0,
                });
            }
        }

        // Sort by path for deterministic output
        changes.sort_by(|a, b| a.path.cmp(&b.path));
        changes
    }

    /// Update tracked state from the latest snapshot.
    pub fn update(&mut self, after: &UpperDirSnapshot) {
        self.prev_file_count = after.file_count();
        self.prev_dir_count = after.dir_count();
    }

    /// Build a workspace digest from the current snapshot and recent changes.
    ///
    /// `last_modified` is derived from `changes` (files changed this command),
    /// not from the full workspace — so agents see which file *this command*
    /// touched most recently, not an arbitrary pre-existing file.
    pub fn digest(&self, after: &UpperDirSnapshot, changes: &[FileChange]) -> WorkspaceDigest {
        let file_count = after.file_count();
        let dir_count = after.dir_count();
        let total_size = after.total_size();
        let file_delta = file_count as i32 - self.prev_file_count as i32;

        // Pick last_modified from changed files only (skip deletions)
        let last_modified = if changes.is_empty() {
            None
        } else {
            changes
                .iter()
                .filter(|c| c.change_type != ChangeType::Deleted)
                .filter_map(|c| after.entries.get(&c.path).map(|m| (&c.path, m.mtime)))
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(p, _)| p.clone())
        };

        WorkspaceDigest {
            file_count,
            file_delta,
            dir_count,
            total_size,
            last_modified,
        }
    }
}

/// Check if a file's metadata changed between snapshots.
fn file_changed(before: &FileMetadata, after: &FileMetadata) -> bool {
    before.size != after.size || (after.mtime - before.mtime).abs() > 0.001
}

/// Strip the `.wh.` prefix from OverlayFS whiteout filenames.
fn strip_whiteout_prefix(path: &Path) -> PathBuf {
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        if let Some(stripped) = name.strip_prefix(".wh.") {
            if let Some(parent) = path.parent() {
                return parent.join(stripped);
            }
            return PathBuf::from(stripped);
        }
    }
    path.to_path_buf()
}
