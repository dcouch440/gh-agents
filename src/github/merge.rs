//! PR merge types and helpers

use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

/// Merge method for pull requests
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum MergeMethod {
    /// Create a merge commit
    #[default]
    Merge,
    /// Squash all commits into one
    Squash,
    /// Rebase commits onto base branch
    Rebase,
}

impl MergeMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            MergeMethod::Merge => "merge",
            MergeMethod::Squash => "squash",
            MergeMethod::Rebase => "rebase",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MergePrRequest {
    /// Title for the merge commit (used with merge and squash)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_title: Option<String>,
    /// Message for the merge commit
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_message: Option<String>,
    /// SHA of the head commit to ensure we're merging expected code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha: Option<String>,
    /// Merge method
    pub merge_method: MergeMethod,
}

impl Default for MergePrRequest {
    fn default() -> Self {
        Self {
            commit_title: None,
            commit_message: None,
            sha: None,
            merge_method: MergeMethod::Merge,
        }
    }
}

impl MergePrRequest {
    pub fn new(method: MergeMethod) -> Self {
        Self {
            merge_method: method,
            ..Default::default()
        }
    }

    pub fn with_sha(mut self, sha: impl Into<String>) -> Self {
        self.sha = Some(sha.into());
        self
    }

    pub fn with_message(mut self, title: impl Into<String>, message: impl Into<String>) -> Self {
        self.commit_title = Some(title.into());
        self.commit_message = Some(message.into());
        self
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct MergePrResponse {
    /// SHA of the merge commit
    pub sha: String,
    /// Whether the merge was successful
    pub merged: bool,
    /// Message from GitHub
    pub message: String,
}

/// Result of attempting to merge a PR
#[derive(Debug, Clone)]
pub enum MergePrResult {
    /// PR was merged successfully
    Merged { sha: String, message: String },
    /// PR is not mergeable (needs update or has conflicts)
    NotMergeable { reason: String },
    /// PR has merge conflicts
    HasConflicts,
    /// PR is already merged
    AlreadyMerged,
    /// Head SHA doesn't match (PR was updated)
    HeadMismatch { expected: String, actual: String },
    /// Other failure
    Failed { status: u16, message: String },
}

impl MergePrResult {
    pub fn is_success(&self) -> bool {
        matches!(self, MergePrResult::Merged { .. })
    }

    pub fn merge_sha(&self) -> Option<&str> {
        match self {
            MergePrResult::Merged { sha, .. } => Some(sha),
            _ => None,
        }
    }
}

/// Mergeable status of a PR
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeableStatus {
    /// PR can be merged
    Mergeable,
    /// PR has conflicts that need resolution
    HasConflicts,
    /// PR is blocked (failing checks, review required, etc.)
    Blocked { reason: String },
    /// GitHub is still calculating - try again later
    Unknown,
    /// PR is already merged
    Merged,
    /// PR is closed
    Closed,
}

impl MergeableStatus {
    pub fn is_mergeable(&self) -> bool {
        matches!(self, MergeableStatus::Mergeable)
    }
}

/// Repository merge settings
#[derive(Debug, Clone)]
pub struct RepoMergeSettings {
    /// Allow merge commits
    pub allow_merge_commit: bool,
    /// Allow squash merging
    pub allow_squash_merge: bool,
    /// Allow rebase merging
    pub allow_rebase_merge: bool,
    /// Automatically delete head branches after merge
    pub delete_branch_on_merge: bool,
}

/// Detailed error for merge failures
#[derive(Error, Debug, Clone)]
pub enum MergeError {
    #[error("PR #{number} has merge conflicts")]
    Conflicts { number: u32 },

    #[error("PR #{number} is not mergeable: {reason}")]
    NotMergeable { number: u32, reason: String },

    #[error("PR #{number} was updated (expected {expected}, got {actual})")]
    HeadChanged {
        number: u32,
        expected: String,
        actual: String,
    },

    #[error("PR #{number} is already merged")]
    AlreadyMerged { number: u32 },

    #[error("PR #{number} is closed")]
    Closed { number: u32 },

    #[error("required status checks are failing for PR #{number}")]
    ChecksFailing { number: u32 },

    #[error("review approval required for PR #{number}")]
    ReviewRequired { number: u32 },

    #[error("branch protection prevents merge of PR #{number}: {reason}")]
    BranchProtection { number: u32, reason: String },

    #[error("merge method {method:?} not allowed for this repository")]
    MethodNotAllowed { method: MergeMethod },

    #[error("rate limited, retry after {retry_after:?}")]
    RateLimited { retry_after: Option<Duration> },

    #[error("API error: {message}")]
    Api { status: u16, message: String },

    #[error("network error: {0}")]
    Network(String),
}

impl MergeError {
    /// Whether this error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            MergeError::RateLimited { .. } | MergeError::Network(_)
        )
    }

    /// Whether this error requires conflict resolution
    pub fn needs_conflict_resolution(&self) -> bool {
        matches!(self, MergeError::Conflicts { .. })
    }

    /// Whether the PR needs to be updated before retry
    pub fn needs_refresh(&self) -> bool {
        matches!(self, MergeError::HeadChanged { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_method_as_str() {
        assert_eq!(MergeMethod::Squash.as_str(), "squash");
        assert_eq!(MergeMethod::Merge.as_str(), "merge");
        assert_eq!(MergeMethod::Rebase.as_str(), "rebase");
    }

    #[test]
    fn merge_request_serializes() {
        let req = MergePrRequest::new(MergeMethod::Squash)
            .with_sha("abc123")
            .with_message("Merge PR #1", "Description here");

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"merge_method\":\"squash\""));
        assert!(json.contains("\"sha\":\"abc123\""));
        assert!(json.contains("\"commit_title\":\"Merge PR #1\""));
    }

    #[test]
    fn merge_method_default() {
        let req = MergePrRequest::default();
        assert_eq!(req.merge_method, MergeMethod::Merge);
    }

    #[test]
    fn merge_result_accessors() {
        let merged = MergePrResult::Merged {
            sha: "abc123".to_string(),
            message: "Merged".to_string(),
        };
        assert!(merged.is_success());
        assert_eq!(merged.merge_sha(), Some("abc123"));

        let conflicts = MergePrResult::HasConflicts;
        assert!(!conflicts.is_success());
        assert_eq!(conflicts.merge_sha(), None);
    }

    #[test]
    fn mergeable_status_checks() {
        assert!(MergeableStatus::Mergeable.is_mergeable());
        assert!(!MergeableStatus::HasConflicts.is_mergeable());
        assert!(!MergeableStatus::Blocked {
            reason: "test".to_string()
        }
        .is_mergeable());
    }

    #[test]
    fn merge_error_retryable() {
        assert!(MergeError::RateLimited { retry_after: None }.is_retryable());
        assert!(MergeError::Network("timeout".to_string()).is_retryable());
        assert!(!MergeError::Conflicts { number: 1 }.is_retryable());
    }

    #[test]
    fn merge_error_needs_resolution() {
        assert!(MergeError::Conflicts { number: 1 }.needs_conflict_resolution());
        assert!(!MergeError::AlreadyMerged { number: 1 }.needs_conflict_resolution());
    }
}
