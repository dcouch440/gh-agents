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
