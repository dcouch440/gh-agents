//! Execution layer for file, git, and test operations

mod approval;
mod files;
mod git;
mod sandbox;
mod test_runner;

pub use approval::{
    approval_channel, ApprovalContext, ApprovalError, ApprovalGate, ApprovalGatesConfig,
    ApprovalRequest, ApprovalRequestReceiver, ApprovalRequestSender, ApprovalResponse,
    AutoApprovalGate, AutonomyLevel, DangerLevel, DangerousOperation, InteractiveApprovalGate,
};
pub use files::{FileError, FileOps};
pub use git::{
    BranchInfo, ChangeType, CommitInfo, ConflictInfo, ConflictRegion, ConflictResolution,
    DiffOptions, FetchResult, FileChange, GitError, GitOps, GitStatus, MergeResult, PushOptions,
    PushResult,
};
pub use sandbox::{
    MountSpec, Sandbox, SandboxConfig, SandboxConfigBuilder, SandboxError, SandboxResult,
};
pub use test_runner::{
    TestError, TestFailure, TestFramework, TestOutputEvent, TestResult, TestRunner,
};

use std::path::{Path, PathBuf};
use thiserror::Error;

/// Context for all execution operations
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Root directory for the project (all paths scoped here)
    pub project_root: PathBuf,

    /// Whether to enforce sandboxing
    pub sandboxed: bool,
}

impl ExecutionContext {
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            project_root,
            sandboxed: true,
        }
    }

    /// Check if a path is within the project root
    pub fn is_path_allowed(&self, path: &std::path::Path) -> bool {
        if !self.sandboxed {
            return true;
        }

        // Canonicalize the project root
        let canonical_root = match self.project_root.canonicalize() {
            Ok(p) => p,
            Err(_) => return false,
        };

        // Try to canonicalize the path
        let canonical_path = match path.canonicalize() {
            Ok(p) => p,
            Err(_) => {
                // Path doesn't exist yet - walk up to find existing ancestor
                self.validate_hypothetical_path(path, &canonical_root)
            }
        };

        canonical_path.starts_with(&canonical_root)
    }

    /// Validate a path that doesn't exist yet by checking its logical location
    fn validate_hypothetical_path(
        &self,
        path: &std::path::Path,
        canonical_root: &std::path::Path,
    ) -> PathBuf {
        // Walk up until we find an existing ancestor
        let mut current = path.to_path_buf();
        let mut to_append: Vec<std::ffi::OsString> = Vec::new();

        while !current.exists() {
            if let Some(name) = current.file_name() {
                to_append.push(name.to_os_string());
            }
            current = match current.parent() {
                Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
                _ => {
                    // Can't find existing ancestor, reject
                    return PathBuf::from("/invalid");
                }
            };
        }

        // Now current exists - canonicalize it
        let resolved_ancestor = match current.canonicalize() {
            Ok(p) => p,
            Err(_) => return PathBuf::from("/invalid"),
        };

        // Rebuild the full path
        let mut result = resolved_ancestor;
        for component in to_append.into_iter().rev() {
            // Check for path traversal attempts in the non-existent parts
            let component_str = component.to_string_lossy();
            if component_str == ".." {
                return PathBuf::from("/invalid");
            }
            result.push(component);
        }

        result
    }
}

#[derive(Error, Debug)]
pub enum PathValidationError {
    #[error("path escapes project directory: {path}")]
    EscapeAttempt { path: PathBuf },

    #[error("path outside project: {path} resolved to {resolved}, project root is {project_root}")]
    OutsideProject {
        path: PathBuf,
        resolved: PathBuf,
        project_root: PathBuf,
    },

    #[error("path has no parent: {path}")]
    NoParent { path: PathBuf },

    #[error("failed to resolve path {path}: {reason}")]
    ResolutionFailed { path: PathBuf, reason: String },

    #[error("invalid project root: {reason}")]
    InvalidProjectRoot { reason: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn context_allows_path_inside_project() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("test.txt"), "content").unwrap();

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        assert!(ctx.is_path_allowed(&tmp.path().join("test.txt")));
    }

    #[test]
    fn context_rejects_path_outside_project() {
        let tmp = TempDir::new().unwrap();
        let ctx = ExecutionContext::new(tmp.path().to_path_buf());

        // /etc/passwd should be rejected
        assert!(!ctx.is_path_allowed(std::path::Path::new("/etc/passwd")));
    }

    #[test]
    fn context_rejects_traversal_attempt() {
        let tmp = TempDir::new().unwrap();
        let ctx = ExecutionContext::new(tmp.path().to_path_buf());

        // ../../../etc/passwd should be rejected
        let escape_path = tmp.path().join("../../../etc/passwd");
        assert!(!ctx.is_path_allowed(&escape_path));
    }
}
