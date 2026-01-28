//! GitHub API integration

mod auth;
mod client;
mod comments;
mod issue_sync;
mod pr;
mod types;

pub use auth::{AuthError, DeviceCodeResponse, GitHubAuth, GitHubUser};
pub use client::GitHubClient;
pub use comments::{CommentError, CommentService, ProgressSummary, SliceProgress};
pub use issue_sync::{convert_issue_to_ticket, IssueRef, IssueSync, IssueSyncError, SyncResult};
pub use pr::{PrBodyGenerator, PrError, PrResult, PrService};
pub use types::{
    CreateIssueComment, CreatePullRequest, CreateReviewRequest, FileStatus, GitHubApiUser,
    GitHubBranchRef, GitHubComment, GitHubError, GitHubIssue, GitHubLabel, GitHubPullRequest,
    GitHubRepoRef, GitHubRepository, GitHubReview, IssueFilters, IssueState, PrChangeSummary,
    PrFile, PullRequestRef, RateLimitInfo, ReviewComment, ReviewEvent, ReviewState,
};
