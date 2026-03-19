//! Workspace tracking — snapshots, diffs, and digests across commands.

pub mod digest;
pub mod snapshot;

mod tests;

use std::path::PathBuf;

use digest::WorkspaceDigest;
use snapshot::{FileMetadata, UpperDirSnapshot};

use super::types::{ChangeType, FileChange};

/// Tracks workspace state across commands within an agent's lifecycle.
pub struct WorkspaceTracker {
    prev_file_count: usize,
    prev_dir_count: usize,
}

impl WorkspaceTracker {
    pub fn new() -> Self {
        Self {
            prev_file_count: 0,
            prev_dir_count: 0,
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
    pub fn digest(&self, after: &UpperDirSnapshot) -> WorkspaceDigest {
        let file_count = after.file_count();
        let dir_count = after.dir_count();
        let total_size = after.total_size();
        let file_delta = file_count as i32 - self.prev_file_count as i32;

        // Find the most recently modified file by mtime
        let last_modified = after
            .entries
            .iter()
            .filter(|(_, m)| m.file_type == 'f')
            .max_by(|(_, a), (_, b)| {
                a.mtime
                    .partial_cmp(&b.mtime)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(p, _)| p.clone());

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
fn strip_whiteout_prefix(path: &PathBuf) -> PathBuf {
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        if let Some(stripped) = name.strip_prefix(".wh.") {
            if let Some(parent) = path.parent() {
                return parent.join(stripped);
            }
            return PathBuf::from(stripped);
        }
    }
    path.clone()
}
