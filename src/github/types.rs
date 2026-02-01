//! GitHub API types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GitHubError {
    #[error("rate limited until {reset}")]
    RateLimited { reset: DateTime<Utc> },

    #[error("not found: {0}")]
    NotFound(String),

    #[error("authentication failed")]
    Unauthorized,

    #[error("API error: {status} - {message}")]
    ApiError { status: u16, message: String },

    #[error("request failed: {0}")]
    RequestFailed(String),

    #[error("configuration error: {0}")]
    ConfigError(String),
}

#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    pub limit: u32,
    pub remaining: u32,
    pub reset: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GitHubIssue {
    pub number: u32,
    pub title: String,
    pub body: Option<String>,
    pub state: String, // "open" or "closed"
    pub labels: Vec<GitHubLabel>,
    pub user: GitHubApiUser,
    pub assignees: Vec<GitHubApiUser>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub html_url: String,
    #[serde(default)]
    pub pull_request: Option<PullRequestRef>,
}

impl GitHubIssue {
    /// Returns true if this is a pull request (not a regular issue)
    pub fn is_pull_request(&self) -> bool {
        self.pull_request.is_some()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PullRequestRef {
    pub url: String,
    pub html_url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GitHubLabel {
    pub name: String,
    pub color: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GitHubApiUser {
    pub login: String,
    pub id: u64,
    #[serde(default)]
    pub avatar_url: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct IssueFilters {
    pub state: Option<IssueState>,
    pub labels: Vec<String>,
    pub assignee: Option<String>,
    pub since: Option<DateTime<Utc>>,
    pub per_page: Option<u8>, // max 100
}

impl IssueFilters {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn state(mut self, state: IssueState) -> Self {
        self.state = Some(state);
        self
    }

    pub fn labels(mut self, labels: Vec<String>) -> Self {
        self.labels = labels;
        self
    }

    pub fn assignee(mut self, assignee: impl Into<String>) -> Self {
        self.assignee = Some(assignee.into());
        self
    }

    pub fn since(mut self, since: DateTime<Utc>) -> Self {
        self.since = Some(since);
        self
    }

    pub fn per_page(mut self, per_page: u8) -> Self {
        self.per_page = Some(per_page.min(100));
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueState {
    Open,
    Closed,
    All,
}

impl IssueState {
    pub fn as_str(&self) -> &'static str {
        match self {
            IssueState::Open => "open",
            IssueState::Closed => "closed",
            IssueState::All => "all",
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GitHubPullRequest {
    pub number: u32,
    pub title: String,
    pub body: Option<String>,
    pub state: String, // "open", "closed"
    pub head: GitHubBranchRef,
    pub base: GitHubBranchRef,
    pub user: GitHubApiUser,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub html_url: String,
    pub merged: Option<bool>,
    pub mergeable: Option<bool>,
    pub mergeable_state: Option<String>,
    #[serde(default)]
    pub labels: Vec<GitHubLabel>,
    #[serde(default)]
    pub requested_reviewers: Vec<GitHubApiUser>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GitHubBranchRef {
    #[serde(rename = "ref")]
    pub ref_name: String,
    pub sha: String,
    pub repo: Option<GitHubRepoRef>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GitHubRepoRef {
    pub full_name: String,
    pub clone_url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GitHubRepository {
    pub id: u64,
    pub name: String,
    pub full_name: String,
    pub private: bool,
    pub html_url: String,
    pub clone_url: String,
    pub default_branch: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreatePullRequest {
    pub title: String,
    pub body: String,
    pub head: String, // branch name
    pub base: String, // target branch (usually "main")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateIssueComment {
    pub body: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GitHubComment {
    pub id: u64,
    pub body: String,
    pub user: GitHubApiUser,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// =========================================================================
// PR Files
// =========================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct PrFile {
    /// Path to the file
    pub filename: String,
    /// Status: added, removed, modified, renamed, etc.
    pub status: FileStatus,
    /// Number of additions
    pub additions: u32,
    /// Number of deletions
    pub deletions: u32,
    /// Total changes
    pub changes: u32,
    /// Patch content (may be empty for binary files or large diffs)
    pub patch: Option<String>,
    /// Previous filename (for renames)
    pub previous_filename: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileStatus {
    Added,
    Removed,
    Modified,
    Renamed,
    Copied,
    Changed,
    Unchanged,
}

#[derive(Debug, Clone, Default)]
pub struct PrChangeSummary {
    pub total_files: u32,
    pub files_added: u32,
    pub files_removed: u32,
    pub files_modified: u32,
    pub files_renamed: u32,
    pub additions: u32,
    pub deletions: u32,
}

// =========================================================================
// PR Reviews
// =========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReviewEvent {
    /// Approve the PR
    Approve,
    /// Request changes before merging
    RequestChanges,
    /// Comment without approving or requesting changes
    Comment,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateReviewRequest {
    /// Review event type
    pub event: ReviewEvent,
    /// Optional body for the review
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    /// Line comments (optional)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub comments: Vec<ReviewComment>,
    /// Commit SHA to review (optional, uses latest if not specified)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReviewComment {
    /// Path to the file
    pub path: String,
    /// Line number in the diff (not the file)
    pub position: u32,
    /// Comment body
    pub body: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GitHubReview {
    pub id: u64,
    pub user: GitHubApiUser,
    pub body: Option<String>,
    pub state: ReviewState,
    pub html_url: String,
    pub submitted_at: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReviewState {
    Approved,
    ChangesRequested,
    Commented,
    Dismissed,
    Pending,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_state_as_str() {
        assert_eq!(IssueState::Open.as_str(), "open");
        assert_eq!(IssueState::Closed.as_str(), "closed");
        assert_eq!(IssueState::All.as_str(), "all");
    }

    #[test]
    fn issue_filters_builder() {
        let filters = IssueFilters::new().state(IssueState::Open).labels(vec!["bug".to_string()]).per_page(50);

        assert_eq!(filters.state, Some(IssueState::Open));
        assert_eq!(filters.labels, vec!["bug"]);
        assert_eq!(filters.per_page, Some(50));
    }

    #[test]
    fn per_page_capped_at_100() {
        let filters = IssueFilters::new().per_page(200);
        assert_eq!(filters.per_page, Some(100));
    }

    #[test]
    fn file_status_deserializes() {
        let json = r#"{"status": "modified"}"#;
        #[derive(serde::Deserialize)]
        struct Test {
            status: FileStatus,
        }
        let t: Test = serde_json::from_str(json).unwrap();
        assert_eq!(t.status, FileStatus::Modified);
    }

    #[test]
    fn review_event_serializes() {
        let req = CreateReviewRequest {
            event: ReviewEvent::Approve,
            body: None,
            comments: Vec::new(),
            commit_id: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"event\":\"APPROVE\""));
    }

    #[test]
    fn review_state_deserializes() {
        let json = r#"{"state": "APPROVED"}"#;
        #[derive(serde::Deserialize)]
        struct Test {
            state: ReviewState,
        }
        let t: Test = serde_json::from_str(json).unwrap();
        assert_eq!(t.state, ReviewState::Approved);
    }
}
