//! File operations with path validation and audit logging

use crate::execution::ExecutionContext;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::fs;

#[derive(Error, Debug)]
pub enum FileError {
    #[error("path outside project directory: {path}")]
    PathOutsideProject { path: PathBuf },

    #[error("file not found: {path}")]
    NotFound { path: PathBuf },

    #[error("permission denied: {path}")]
    PermissionDenied { path: PathBuf },

    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("file too large: {path} ({size} bytes, max {max} bytes)")]
    FileTooLarge { path: PathBuf, size: u64, max: u64 },
}

/// Maximum file size to read (10 MB)
const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;

pub struct FileOps {
    ctx: ExecutionContext,
}

impl FileOps {
    pub fn new(ctx: ExecutionContext) -> Self {
        Self { ctx }
    }

    /// Read a file's contents as a string
    ///
    /// # Arguments
    /// * `path` - Path relative to project root, or absolute path within project
    ///
    /// # Errors
    /// * `PathOutsideProject` if path escapes project directory
    /// * `NotFound` if file doesn't exist
    /// * `FileTooLarge` if file exceeds size limit
    pub async fn read_file(&self, path: impl AsRef<Path>) -> Result<String, FileError> {
        let path = self.resolve_path(path.as_ref())?;

        // Check file size first
        let metadata = fs::metadata(&path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                FileError::NotFound { path: path.clone() }
            } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                FileError::PermissionDenied { path: path.clone() }
            } else {
                FileError::IoError(e)
            }
        })?;

        if metadata.len() > MAX_FILE_SIZE {
            return Err(FileError::FileTooLarge {
                path,
                size: metadata.len(),
                max: MAX_FILE_SIZE,
            });
        }

        let content = fs::read_to_string(&path).await?;

        tracing::debug!(
            path = %path.display(),
            size = content.len(),
            "File read"
        );

        Ok(content)
    }

    /// Write content to a file
    ///
    /// Creates the file if it doesn't exist, overwrites if it does.
    /// Creates parent directories as needed.
    ///
    /// # Arguments
    /// * `path` - Path relative to project root, or absolute path within project
    /// * `content` - Content to write
    ///
    /// # Errors
    /// * `PathOutsideProject` if path escapes project directory
    pub async fn write_file(
        &self,
        path: impl AsRef<Path>,
        content: impl AsRef<str>,
    ) -> Result<(), FileError> {
        let path = self.resolve_path(path.as_ref())?;

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let content = content.as_ref();
        fs::write(&path, content).await?;

        tracing::debug!(
            path = %path.display(),
            size = content.len(),
            "File written"
        );

        Ok(())
    }

    /// Check if a file exists
    pub async fn exists(&self, path: impl AsRef<Path>) -> Result<bool, FileError> {
        let path = self.resolve_path(path.as_ref())?;
        Ok(fs::try_exists(&path).await.unwrap_or(false))
    }

    /// Delete a file
    pub async fn delete_file(&self, path: impl AsRef<Path>) -> Result<(), FileError> {
        let path = self.resolve_path(path.as_ref())?;

        fs::remove_file(&path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                FileError::NotFound { path: path.clone() }
            } else {
                FileError::IoError(e)
            }
        })?;

        tracing::debug!(
            path = %path.display(),
            "File deleted"
        );

        Ok(())
    }

    /// List files in a directory
    pub async fn list_dir(&self, path: impl AsRef<Path>) -> Result<Vec<PathBuf>, FileError> {
        let path = self.resolve_path(path.as_ref())?;

        let mut entries = Vec::new();
        let mut dir = fs::read_dir(&path).await?;

        while let Some(entry) = dir.next_entry().await? {
            entries.push(entry.path());
        }

        Ok(entries)
    }

    /// Walk a directory to `max_depth` levels, returning paths relative to it
    /// with a trailing `/` on every directory.
    ///
    /// The local counterpart to `ContainerHandle::list_files`, and it filters
    /// on the same rule — `diagnostics::workspace::is_noise` — so a listing
    /// looks the same whichever side of the container boundary it came from.
    /// Returns the entries and how many were dropped by the cap.
    pub async fn list_tree(
        &self,
        path: impl AsRef<Path>,
        max_depth: u32,
        max_entries: usize,
    ) -> Result<(Vec<String>, usize), FileError> {
        let root = self.resolve_path(path.as_ref())?;

        // The root is checked up front because everything below it is
        // skipped on error: a missing path, or one that is not a directory,
        // has to come back as an error rather than as an empty listing, which
        // the caller cannot tell from a directory that really is empty.
        match fs::metadata(&root).await {
            Ok(m) if m.is_dir() => {}
            Ok(_) => {
                return Err(FileError::IoError(std::io::Error::other(format!(
                    "not a directory: {}",
                    root.display()
                ))))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(FileError::NotFound { path: root })
            }
            Err(e) => return Err(FileError::IoError(e)),
        }

        let mut out = Vec::new();
        let mut queue = vec![(root.clone(), 1u32)];

        while let Some((dir, depth)) = queue.pop() {
            // A directory that cannot be read is skipped rather than failing
            // the walk. `find` on the container side writes to stderr and
            // keeps going, and at a depth of up to six one unreadable
            // subdirectory should not cost the caller the whole listing.
            let Ok(mut entries) = fs::read_dir(&dir).await else {
                continue;
            };
            while let Ok(Some(entry)) = entries.next_entry().await {
                let full = entry.path();
                let Ok(rel) = full.strip_prefix(&root) else {
                    continue;
                };
                if crate::execution::diagnostics::workspace::is_noise(rel) {
                    continue;
                }
                let is_dir = entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false);
                let mut name = rel.to_string_lossy().to_string();
                if is_dir {
                    name.push('/');
                    if depth < max_depth {
                        queue.push((full, depth + 1));
                    }
                }
                out.push(name);
            }
        }

        out.sort();
        let dropped = out.len().saturating_sub(max_entries);
        out.truncate(max_entries);
        Ok((out, dropped))
    }

    /// Resolve a path relative to project root and validate it
    fn resolve_path(&self, path: &Path) -> Result<PathBuf, FileError> {
        let full_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.ctx.project_root.join(path)
        };

        if !self.ctx.is_path_allowed(&full_path) {
            return Err(FileError::PathOutsideProject { path: full_path });
        }

        Ok(full_path)
    }
}

#[cfg(test)]
mod tests;
