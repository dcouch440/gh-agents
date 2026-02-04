//! GitHub REST API client

use crate::config::CredentialsStore;
use crate::github::types::{
    CreateIssueComment, CreatePullRequest, GitHubComment, GitHubError, GitHubIssue,
    GitHubPullRequest, GitHubRepository, IssueFilters, RateLimitInfo,
};
use chrono::{DateTime, Utc};
use reqwest::{header, Client, Response, StatusCode};
use serde::de::DeserializeOwned;

const GITHUB_API_BASE: &str = "https://api.github.com";

pub struct GitHubClient {
    client: Client,
    base_url: String,
}

impl GitHubClient {
    /// Create a new client, loading token from credentials store or env var
    pub fn new() -> Result<Self, GitHubError> {
        let store = CredentialsStore::new();
        let token = store.get_github_token().ok_or_else(|| {
            GitHubError::ConfigError(
                "No GitHub token found. Set GITHUB_TOKEN or run 'nexor auth login'".to_string(),
            )
        })?;

        Self::with_token(&token)
    }

    /// Create a client with a specific token
    pub fn with_token(token: &str) -> Result<Self, GitHubError> {
        let mut headers = header::HeaderMap::new();

        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&format!("Bearer {}", token))
                .map_err(|_| GitHubError::ConfigError("Invalid token format".to_string()))?,
        );
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("application/vnd.github+json"),
        );
        headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_static("nexor/0.1"),
        );
        headers.insert(
            "X-GitHub-Api-Version",
            header::HeaderValue::from_static("2022-11-28"),
        );

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .map_err(|e| {
                GitHubError::ConfigError(format!("Failed to create HTTP client: {}", e))
            })?;

        Ok(Self {
            client,
            base_url: GITHUB_API_BASE.to_string(),
        })
    }

    /// Override base URL (useful for testing or GitHub Enterprise)
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Get the current base URL
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Check rate limit headers from a response
    fn check_rate_limit(&self, response: &Response) -> Option<RateLimitInfo> {
        let headers = response.headers();

        let limit = headers
            .get("x-ratelimit-limit")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);

        let remaining = headers
            .get("x-ratelimit-remaining")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);

        let reset_timestamp = headers
            .get("x-ratelimit-reset")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(0);

        let reset = DateTime::from_timestamp(reset_timestamp, 0).unwrap_or_else(Utc::now);

        tracing::debug!(limit, remaining, %reset, "GitHub rate limit status");

        if remaining == 0 && limit > 0 {
            tracing::warn!("GitHub rate limit exhausted, resets at {}", reset);
        }

        Some(RateLimitInfo {
            limit,
            remaining,
            reset,
        })
    }

    /// Handle response and parse JSON
    async fn handle_response<T: DeserializeOwned>(
        &self,
        response: Response,
    ) -> Result<T, GitHubError> {
        let rate_limit = self.check_rate_limit(&response);
        let status = response.status();

        match status {
            s if s.is_success() => response.json().await.map_err(|e| {
                GitHubError::RequestFailed(format!("Failed to parse response: {}", e))
            }),

            StatusCode::FORBIDDEN => {
                if let Some(info) = rate_limit {
                    if info.remaining == 0 {
                        return Err(GitHubError::RateLimited { reset: info.reset });
                    }
                }
                let message = response.text().await.unwrap_or_default();
                Err(GitHubError::ApiError {
                    status: 403,
                    message,
                })
            }

            StatusCode::UNAUTHORIZED => Err(GitHubError::Unauthorized),

            StatusCode::NOT_FOUND => {
                let message = response.text().await.unwrap_or_default();
                Err(GitHubError::NotFound(message))
            }

            _ => {
                let message = response.text().await.unwrap_or_default();
                Err(GitHubError::ApiError {
                    status: status.as_u16(),
                    message,
                })
            }
        }
    }

    // =========================================================================
    // Issue Operations
    // =========================================================================

    /// Get a single issue by number
    pub async fn get_issue(
        &self,
        owner: &str,
        repo: &str,
        number: u32,
    ) -> Result<GitHubIssue, GitHubError> {
        let url = format!(
            "{}/repos/{}/{}/issues/{}",
            self.base_url, owner, repo, number
        );

        tracing::debug!(url = %url, "Fetching issue");

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| GitHubError::RequestFailed(e.to_string()))?;

        self.handle_response(response).await
    }

    /// List issues with optional filters
    pub async fn list_issues(
        &self,
        owner: &str,
        repo: &str,
        filters: &IssueFilters,
    ) -> Result<Vec<GitHubIssue>, GitHubError> {
        let mut url = format!("{}/repos/{}/{}/issues", self.base_url, owner, repo);

        // Build query parameters
        let mut params: Vec<String> = Vec::new();

        if let Some(state) = &filters.state {
            params.push(format!("state={}", state.as_str()));
        }

        if !filters.labels.is_empty() {
            params.push(format!("labels={}", filters.labels.join(",")));
        }

        if let Some(assignee) = &filters.assignee {
            params.push(format!("assignee={}", assignee));
        }

        if let Some(since) = &filters.since {
            params.push(format!("since={}", since.to_rfc3339()));
        }

        let per_page = filters.per_page.unwrap_or(30).min(100);
        params.push(format!("per_page={}", per_page));

        if !params.is_empty() {
            url = format!("{}?{}", url, params.join("&"));
        }

        tracing::debug!(url = %url, "Listing issues");

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| GitHubError::RequestFailed(e.to_string()))?;

        self.handle_response(response).await
    }

    // =========================================================================
    // Pull Request Operations
    // =========================================================================

    /// Get a single pull request by number
    pub async fn get_pull_request(
        &self,
        owner: &str,
        repo: &str,
        number: u32,
    ) -> Result<GitHubPullRequest, GitHubError> {
        let url = format!(
            "{}/repos/{}/{}/pulls/{}",
            self.base_url, owner, repo, number
        );

        tracing::debug!(url = %url, "Fetching pull request");

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| GitHubError::RequestFailed(e.to_string()))?;

        self.handle_response(response).await
    }

    /// List pull requests
    pub async fn list_pull_requests(
        &self,
        owner: &str,
        repo: &str,
        state: Option<&str>, // "open", "closed", "all"
    ) -> Result<Vec<GitHubPullRequest>, GitHubError> {
        let mut url = format!("{}/repos/{}/{}/pulls", self.base_url, owner, repo);

        if let Some(s) = state {
            url = format!("{}?state={}", url, s);
        }

        tracing::debug!(url = %url, "Listing pull requests");

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| GitHubError::RequestFailed(e.to_string()))?;

        self.handle_response(response).await
    }

    /// Create a pull request
    pub async fn create_pull_request(
        &self,
        owner: &str,
        repo: &str,
        pr: &CreatePullRequest,
    ) -> Result<GitHubPullRequest, GitHubError> {
        let url = format!("{}/repos/{}/{}/pulls", self.base_url, owner, repo);

        tracing::debug!(url = %url, title = %pr.title, "Creating pull request");

        let response = self
            .client
            .post(&url)
            .json(pr)
            .send()
            .await
            .map_err(|e| GitHubError::RequestFailed(e.to_string()))?;

        self.handle_response(response).await
    }

    // =========================================================================
    // Comment Operations
    // =========================================================================

    /// Add a comment to an issue or PR
    pub async fn create_issue_comment(
        &self,
        owner: &str,
        repo: &str,
        issue_number: u32,
        comment: &CreateIssueComment,
    ) -> Result<GitHubComment, GitHubError> {
        let url = format!(
            "{}/repos/{}/{}/issues/{}/comments",
            self.base_url, owner, repo, issue_number
        );

        tracing::debug!(url = %url, "Creating comment");

        let response = self
            .client
            .post(&url)
            .json(comment)
            .send()
            .await
            .map_err(|e| GitHubError::RequestFailed(e.to_string()))?;

        self.handle_response(response).await
    }

    /// List comments on an issue or PR
    pub async fn list_issue_comments(
        &self,
        owner: &str,
        repo: &str,
        issue_number: u32,
    ) -> Result<Vec<GitHubComment>, GitHubError> {
        let url = format!(
            "{}/repos/{}/{}/issues/{}/comments",
            self.base_url, owner, repo, issue_number
        );

        tracing::debug!(url = %url, "Listing comments");

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| GitHubError::RequestFailed(e.to_string()))?;

        self.handle_response(response).await
    }

    /// Update an existing comment
    pub async fn update_issue_comment(
        &self,
        owner: &str,
        repo: &str,
        comment_id: u64,
        comment: &CreateIssueComment,
    ) -> Result<GitHubComment, GitHubError> {
        let url = format!(
            "{}/repos/{}/{}/issues/comments/{}",
            self.base_url, owner, repo, comment_id
        );

        tracing::debug!(url = %url, comment_id, "Updating comment");

        let response = self
            .client
            .patch(&url)
            .json(comment)
            .send()
            .await
            .map_err(|e| GitHubError::RequestFailed(e.to_string()))?;

        self.handle_response(response).await
    }

    // =========================================================================
    // Repository Operations
    // =========================================================================

    /// Get repository info
    pub async fn get_repository(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<GitHubRepository, GitHubError> {
        let url = format!("{}/repos/{}/{}", self.base_url, owner, repo);

        tracing::debug!(url = %url, "Fetching repository");

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| GitHubError::RequestFailed(e.to_string()))?;

        self.handle_response(response).await
    }

    // =========================================================================
    // PR Files Operations
    // =========================================================================

    /// Get the files changed in a pull request
    pub async fn get_pr_files(
        &self,
        owner: &str,
        repo: &str,
        number: u32,
    ) -> Result<Vec<crate::github::types::PrFile>, GitHubError> {
        let url = format!(
            "{}/repos/{}/{}/pulls/{}/files",
            self.base_url, owner, repo, number
        );

        // GitHub paginates this - get all pages
        let mut all_files = Vec::new();
        let mut page = 1u32;

        loop {
            let page_url = format!("{}?page={}&per_page=100", url, page);

            tracing::debug!(url = %page_url, "Fetching PR files page {}", page);

            let response = self
                .client
                .get(&page_url)
                .send()
                .await
                .map_err(|e| GitHubError::RequestFailed(e.to_string()))?;

            let files: Vec<crate::github::types::PrFile> = self.handle_response(response).await?;

            if files.is_empty() {
                break;
            }

            all_files.extend(files);
            page += 1;

            // Safety limit
            if page > 50 {
                tracing::warn!("PR has too many files, stopping at 5000");
                break;
            }
        }

        tracing::debug!(
            pr = number,
            file_count = all_files.len(),
            "Retrieved PR files"
        );

        Ok(all_files)
    }

    /// Get a summary of changes in a PR
    pub async fn get_pr_change_summary(
        &self,
        owner: &str,
        repo: &str,
        number: u32,
    ) -> Result<crate::github::types::PrChangeSummary, GitHubError> {
        use crate::github::types::{FileStatus, PrChangeSummary};

        let files = self.get_pr_files(owner, repo, number).await?;

        let mut summary = PrChangeSummary::default();

        for file in &files {
            summary.total_files += 1;
            summary.additions += file.additions;
            summary.deletions += file.deletions;

            match file.status {
                FileStatus::Added => summary.files_added += 1,
                FileStatus::Removed => summary.files_removed += 1,
                FileStatus::Modified | FileStatus::Changed => summary.files_modified += 1,
                FileStatus::Renamed => summary.files_renamed += 1,
                _ => {}
            }
        }

        Ok(summary)
    }

    // =========================================================================
    // PR Review Operations
    // =========================================================================

    /// Submit a review on a pull request
    pub async fn create_review(
        &self,
        owner: &str,
        repo: &str,
        number: u32,
        review: &crate::github::types::CreateReviewRequest,
    ) -> Result<crate::github::types::GitHubReview, GitHubError> {
        let url = format!(
            "{}/repos/{}/{}/pulls/{}/reviews",
            self.base_url, owner, repo, number
        );

        tracing::debug!(url = %url, event = ?review.event, "Submitting PR review");

        let response = self
            .client
            .post(&url)
            .json(review)
            .send()
            .await
            .map_err(|e| GitHubError::RequestFailed(e.to_string()))?;

        self.handle_response(response).await
    }

    /// Approve a PR with optional comment
    pub async fn approve_pr(
        &self,
        owner: &str,
        repo: &str,
        number: u32,
        comment: Option<&str>,
    ) -> Result<crate::github::types::GitHubReview, GitHubError> {
        use crate::github::types::{CreateReviewRequest, ReviewEvent};

        self.create_review(
            owner,
            repo,
            number,
            &CreateReviewRequest {
                event: ReviewEvent::Approve,
                body: comment.map(String::from),
                comments: Vec::new(),
                commit_id: None,
            },
        )
        .await
    }

    /// Request changes on a PR
    pub async fn request_pr_changes(
        &self,
        owner: &str,
        repo: &str,
        number: u32,
        reason: &str,
    ) -> Result<crate::github::types::GitHubReview, GitHubError> {
        use crate::github::types::{CreateReviewRequest, ReviewEvent};

        self.create_review(
            owner,
            repo,
            number,
            &CreateReviewRequest {
                event: ReviewEvent::RequestChanges,
                body: Some(reason.to_string()),
                comments: Vec::new(),
                commit_id: None,
            },
        )
        .await
    }

    /// List reviews on a PR
    pub async fn list_pr_reviews(
        &self,
        owner: &str,
        repo: &str,
        number: u32,
    ) -> Result<Vec<crate::github::types::GitHubReview>, GitHubError> {
        let url = format!(
            "{}/repos/{}/{}/pulls/{}/reviews",
            self.base_url, owner, repo, number
        );

        tracing::debug!(url = %url, "Listing PR reviews");

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| GitHubError::RequestFailed(e.to_string()))?;

        self.handle_response(response).await
    }

    // =========================================================================
    // PR Merge Operations
    // =========================================================================

    /// Merge a pull request
    pub async fn merge_pr(
        &self,
        owner: &str,
        repo: &str,
        number: u32,
        request: &crate::github::merge::MergePrRequest,
    ) -> Result<crate::github::merge::MergePrResult, GitHubError> {
        use crate::github::merge::{MergePrResponse, MergePrResult};

        let url = format!(
            "{}/repos/{}/{}/pulls/{}/merge",
            self.base_url, owner, repo, number
        );

        tracing::debug!(
            url = %url,
            method = ?request.merge_method,
            "Merging PR"
        );

        let response = self
            .client
            .put(&url)
            .json(request)
            .send()
            .await
            .map_err(|e| GitHubError::RequestFailed(e.to_string()))?;

        match response.status() {
            reqwest::StatusCode::OK => {
                let result: MergePrResponse = response
                    .json()
                    .await
                    .map_err(|e| GitHubError::RequestFailed(e.to_string()))?;

                tracing::info!(
                    pr = number,
                    sha = %result.sha,
                    method = ?request.merge_method,
                    "PR merged successfully"
                );

                Ok(MergePrResult::Merged {
                    sha: result.sha,
                    message: result.message,
                })
            }
            reqwest::StatusCode::METHOD_NOT_ALLOWED => {
                let body = response.text().await.unwrap_or_default();
                tracing::warn!(pr = number, "PR not mergeable: {}", body);

                if body.contains("already merged") {
                    Ok(MergePrResult::AlreadyMerged)
                } else {
                    Ok(MergePrResult::NotMergeable { reason: body })
                }
            }
            reqwest::StatusCode::CONFLICT => {
                let body = response.text().await.unwrap_or_default();
                tracing::warn!(pr = number, "Merge conflict: {}", body);

                if body.contains("Head branch was modified") {
                    Ok(MergePrResult::HeadMismatch {
                        expected: request.sha.clone().unwrap_or_default(),
                        actual: "unknown".to_string(),
                    })
                } else {
                    Ok(MergePrResult::HasConflicts)
                }
            }
            status => {
                let body = response.text().await.unwrap_or_default();
                Ok(MergePrResult::Failed {
                    status: status.as_u16(),
                    message: body,
                })
            }
        }
    }

    /// Merge PR with simple options
    pub async fn merge_pr_simple(
        &self,
        owner: &str,
        repo: &str,
        number: u32,
        method: crate::github::merge::MergeMethod,
    ) -> Result<crate::github::merge::MergePrResult, GitHubError> {
        use crate::github::merge::MergePrRequest;
        self.merge_pr(owner, repo, number, &MergePrRequest::new(method))
            .await
    }

    /// Get the current mergeable status of a PR
    pub async fn get_mergeable_status(
        &self,
        owner: &str,
        repo: &str,
        number: u32,
    ) -> Result<crate::github::merge::MergeableStatus, GitHubError> {
        use crate::github::merge::MergeableStatus;

        let pr = self.get_pull_request(owner, repo, number).await?;

        // Check if already merged or closed
        if pr.merged == Some(true) {
            return Ok(MergeableStatus::Merged);
        }

        if pr.state == "closed" {
            return Ok(MergeableStatus::Closed);
        }

        // Check mergeable status
        match (pr.mergeable, pr.mergeable_state.as_deref()) {
            (Some(true), _) => Ok(MergeableStatus::Mergeable),
            (Some(false), Some("dirty")) => Ok(MergeableStatus::HasConflicts),
            (Some(false), Some(state)) => Ok(MergeableStatus::Blocked {
                reason: state.to_string(),
            }),
            (Some(false), None) => Ok(MergeableStatus::Blocked {
                reason: "unknown".to_string(),
            }),
            (None, _) => Ok(MergeableStatus::Unknown),
        }
    }

    /// Wait for PR to become mergeable (or definitely not)
    pub async fn wait_for_mergeable(
        &self,
        owner: &str,
        repo: &str,
        number: u32,
        timeout: std::time::Duration,
        poll_interval: std::time::Duration,
    ) -> Result<crate::github::merge::MergeableStatus, GitHubError> {
        use crate::github::merge::MergeableStatus;

        let start = std::time::Instant::now();

        loop {
            let status = self.get_mergeable_status(owner, repo, number).await?;

            match status {
                MergeableStatus::Unknown => {
                    if start.elapsed() >= timeout {
                        tracing::warn!(
                            pr = number,
                            elapsed = ?start.elapsed(),
                            "Timeout waiting for mergeable status"
                        );
                        return Ok(MergeableStatus::Unknown);
                    }

                    tracing::debug!(
                        pr = number,
                        "Mergeable status unknown, retrying in {:?}",
                        poll_interval
                    );

                    tokio::time::sleep(poll_interval).await;
                }
                _ => {
                    tracing::debug!(pr = number, status = ?status, "Got mergeable status");
                    return Ok(status);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_requires_token() {
        // Clear any existing token from env
        std::env::remove_var(crate::constants::ENV_GITHUB_TOKEN);

        // Creating client without token should fail
        let result = GitHubClient::new();
        assert!(result.is_err());
    }

    #[test]
    fn client_with_token_works() {
        let result = GitHubClient::with_token("test-token");
        assert!(result.is_ok());

        let client = result.unwrap();
        assert_eq!(client.base_url(), GITHUB_API_BASE);
    }

    #[test]
    fn base_url_can_be_overridden() {
        let client = GitHubClient::with_token("test-token")
            .unwrap()
            .with_base_url("https://github.example.com/api/v3");

        assert_eq!(client.base_url(), "https://github.example.com/api/v3");
    }

    #[tokio::test]
    async fn get_repository_success() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/owner/repo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 1,
                "name": "repo",
                "full_name": "owner/repo",
                "private": false,
                "owner": { "login": "owner", "id": 1 },
                "html_url": "https://github.com/owner/repo",
                "description": "A test repo",
                "default_branch": "main",
                "clone_url": "https://github.com/owner/repo.git"
            })))
            .mount(&mock_server)
            .await;

        let client = GitHubClient::with_token("test-token")
            .unwrap()
            .with_base_url(mock_server.uri());

        let repo = client.get_repository("owner", "repo").await.unwrap();
        assert_eq!(repo.name, "repo");
        assert_eq!(repo.default_branch, "main");
    }

    #[tokio::test]
    async fn get_repository_not_found() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/owner/nonexistent"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "message": "Not Found"
            })))
            .mount(&mock_server)
            .await;

        let client = GitHubClient::with_token("test-token")
            .unwrap()
            .with_base_url(mock_server.uri());

        let result = client.get_repository("owner", "nonexistent").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), GitHubError::NotFound(_)));
    }

    #[tokio::test]
    async fn list_pull_requests_success() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/pulls"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {
                    "id": 1,
                    "number": 42,
                    "title": "Fix bug",
                    "state": "open",
                    "user": { "login": "dev", "id": 2 },
                    "html_url": "https://github.com/owner/repo/pull/42",
                    "body": "Fixes #1",
                    "head": { "ref": "fix-bug", "sha": "abc123" },
                    "base": { "ref": "main", "sha": "def456" },
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-01T00:00:00Z"
                }
            ])))
            .mount(&mock_server)
            .await;

        let client = GitHubClient::with_token("test-token")
            .unwrap()
            .with_base_url(mock_server.uri());

        let prs = client
            .list_pull_requests("owner", "repo", None)
            .await
            .unwrap();
        assert_eq!(prs.len(), 1);
        assert_eq!(prs[0].number, 42);
        assert_eq!(prs[0].title, "Fix bug");
    }

    #[tokio::test]
    async fn create_issue_comment_success() {
        use crate::github::types::CreateIssueComment;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/repos/owner/repo/issues/1/comments"))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "id": 100,
                "body": "Nice work!",
                "user": { "login": "bot", "id": 3 },
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-01-01T00:00:00Z",
                "html_url": "https://github.com/owner/repo/issues/1#comment-100"
            })))
            .mount(&mock_server)
            .await;

        let client = GitHubClient::with_token("test-token")
            .unwrap()
            .with_base_url(mock_server.uri());

        let comment = client
            .create_issue_comment(
                "owner",
                "repo",
                1,
                &CreateIssueComment {
                    body: "Nice work!".into(),
                },
            )
            .await
            .unwrap();
        assert_eq!(comment.body, "Nice work!");
    }

    #[tokio::test]
    async fn get_pull_request_success() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/pulls/10"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 10,
                "number": 10,
                "title": "Add feature",
                "state": "open",
                "user": { "login": "dev", "id": 1 },
                "html_url": "https://github.com/owner/repo/pull/10",
                "body": "New feature",
                "head": { "ref": "feature", "sha": "aaa" },
                "base": { "ref": "main", "sha": "bbb" },
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-01-01T00:00:00Z"
            })))
            .mount(&mock_server)
            .await;

        let client = GitHubClient::with_token("test-token")
            .unwrap()
            .with_base_url(mock_server.uri());

        let pr = client.get_pull_request("owner", "repo", 10).await.unwrap();
        assert_eq!(pr.title, "Add feature");
        assert_eq!(pr.number, 10);
    }

    #[tokio::test]
    async fn unauthorized_returns_error() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/owner/repo"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&mock_server)
            .await;

        let client = GitHubClient::with_token("bad-token")
            .unwrap()
            .with_base_url(mock_server.uri());

        let result = client.get_repository("owner", "repo").await;
        assert!(matches!(result.unwrap_err(), GitHubError::Unauthorized));
    }

    // Helper to build a mock issue JSON
    fn mock_issue_json(number: u32, title: &str) -> serde_json::Value {
        serde_json::json!({
            "number": number,
            "title": title,
            "body": "body",
            "state": "open",
            "labels": [],
            "user": { "login": "dev", "id": 1 },
            "assignees": [],
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z",
            "html_url": format!("https://github.com/owner/repo/issues/{}", number)
        })
    }

    fn mock_comment_json(id: u64, body: &str) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "body": body,
            "user": { "login": "bot", "id": 3 },
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z"
        })
    }

    fn mock_pr_json(number: u32, title: &str) -> serde_json::Value {
        serde_json::json!({
            "id": number,
            "number": number,
            "title": title,
            "state": "open",
            "user": { "login": "dev", "id": 1 },
            "html_url": format!("https://github.com/owner/repo/pull/{}", number),
            "body": "pr body",
            "head": { "ref": "feature", "sha": "abc123" },
            "base": { "ref": "main", "sha": "def456" },
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z"
        })
    }

    fn mock_review_json(id: u64, state: &str) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "user": { "login": "reviewer", "id": 5 },
            "body": "Looks good",
            "state": state,
            "html_url": "https://github.com/owner/repo/pull/1#review-1",
            "submitted_at": "2024-01-01T00:00:00Z"
        })
    }

    async fn mock_client(server: &wiremock::MockServer) -> GitHubClient {
        GitHubClient::with_token("test-token")
            .unwrap()
            .with_base_url(server.uri())
    }

    #[tokio::test]
    async fn list_issues_no_filters() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/issues"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                mock_issue_json(1, "Issue one"),
                mock_issue_json(2, "Issue two"),
            ])))
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let issues = client
            .list_issues("owner", "repo", &IssueFilters::default())
            .await
            .unwrap();
        assert_eq!(issues.len(), 2);
        assert_eq!(issues[0].title, "Issue one");
    }

    #[tokio::test]
    async fn list_issues_with_filters() {
        use crate::github::types::IssueState;
        use wiremock::matchers::{method, path, query_param};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/issues"))
            .and(query_param("state", "closed"))
            .and(query_param("labels", "bug,urgent"))
            .and(query_param("assignee", "alice"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!([mock_issue_json(3, "Bug fix")])),
            )
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let filters = IssueFilters::new()
            .state(IssueState::Closed)
            .labels(vec!["bug".into(), "urgent".into()])
            .assignee("alice");
        let issues = client.list_issues("owner", "repo", &filters).await.unwrap();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].number, 3);
    }

    #[tokio::test]
    async fn create_pull_request_success() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/repos/owner/repo/pulls"))
            .respond_with(ResponseTemplate::new(201).set_body_json(mock_pr_json(99, "New PR")))
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let pr = client
            .create_pull_request(
                "owner",
                "repo",
                &CreatePullRequest {
                    title: "New PR".into(),
                    body: "Description".into(),
                    head: "feature".into(),
                    base: "main".into(),
                    draft: None,
                },
            )
            .await
            .unwrap();
        assert_eq!(pr.number, 99);
        assert_eq!(pr.title, "New PR");
    }

    #[tokio::test]
    async fn list_issue_comments_success() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/issues/5/comments"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                mock_comment_json(10, "First comment"),
                mock_comment_json(11, "Second comment"),
            ])))
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let comments = client
            .list_issue_comments("owner", "repo", 5)
            .await
            .unwrap();
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].body, "First comment");
        assert_eq!(comments[1].id, 11);
    }

    #[tokio::test]
    async fn update_issue_comment_success() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/repos/owner/repo/issues/comments/42"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(mock_comment_json(42, "Updated body")),
            )
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let comment = client
            .update_issue_comment(
                "owner",
                "repo",
                42,
                &CreateIssueComment {
                    body: "Updated body".into(),
                },
            )
            .await
            .unwrap();
        assert_eq!(comment.id, 42);
        assert_eq!(comment.body, "Updated body");
    }

    #[tokio::test]
    async fn get_pr_files_with_pagination() {
        use wiremock::matchers::{method, path, query_param};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;

        // Page 1: returns one file
        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/pulls/7/files"))
            .and(query_param("page", "1"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!([{
                    "filename": "src/main.rs",
                    "status": "modified",
                    "additions": 10,
                    "deletions": 2,
                    "changes": 12,
                    "patch": "@@ -1,5 +1,13 @@"
                }])),
            )
            .mount(&server)
            .await;

        // Page 2: empty, stops pagination
        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/pulls/7/files"))
            .and(query_param("page", "2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let files = client.get_pr_files("owner", "repo", 7).await.unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].filename, "src/main.rs");
        assert_eq!(files[0].additions, 10);
    }

    #[tokio::test]
    async fn list_pr_reviews_success() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/pulls/3/reviews"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                mock_review_json(1, "APPROVED"),
                mock_review_json(2, "CHANGES_REQUESTED"),
            ])))
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let reviews = client.list_pr_reviews("owner", "repo", 3).await.unwrap();
        assert_eq!(reviews.len(), 2);
        assert_eq!(
            reviews[0].state,
            crate::github::types::ReviewState::Approved
        );
        assert_eq!(
            reviews[1].state,
            crate::github::types::ReviewState::ChangesRequested
        );
    }

    #[tokio::test]
    async fn create_review_success() {
        use crate::github::types::{CreateReviewRequest, ReviewEvent};
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/repos/owner/repo/pulls/4/reviews"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(mock_review_json(10, "APPROVED")),
            )
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let review = client
            .create_review(
                "owner",
                "repo",
                4,
                &CreateReviewRequest {
                    event: ReviewEvent::Approve,
                    body: Some("LGTM".into()),
                    comments: Vec::new(),
                    commit_id: None,
                },
            )
            .await
            .unwrap();
        assert_eq!(review.id, 10);
    }

    #[tokio::test]
    async fn merge_pr_success() {
        use crate::github::merge::{MergeMethod, MergePrRequest, MergePrResult};
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .and(path("/repos/owner/repo/pulls/1/merge"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "sha": "abc123",
                "merged": true,
                "message": "Pull Request successfully merged"
            })))
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let result = client
            .merge_pr(
                "owner",
                "repo",
                1,
                &MergePrRequest::new(MergeMethod::Squash),
            )
            .await
            .unwrap();
        assert!(matches!(result, MergePrResult::Merged { sha, .. } if sha == "abc123"));
    }

    #[tokio::test]
    async fn merge_pr_method_not_allowed_already_merged() {
        use crate::github::merge::{MergeMethod, MergePrRequest, MergePrResult};
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .and(path("/repos/owner/repo/pulls/2/merge"))
            .respond_with(ResponseTemplate::new(405).set_body_string("Pull request already merged"))
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let result = client
            .merge_pr("owner", "repo", 2, &MergePrRequest::new(MergeMethod::Merge))
            .await
            .unwrap();
        assert!(matches!(result, MergePrResult::AlreadyMerged));
    }

    #[tokio::test]
    async fn merge_pr_method_not_allowed_not_mergeable() {
        use crate::github::merge::{MergeMethod, MergePrRequest, MergePrResult};
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .and(path("/repos/owner/repo/pulls/2/merge"))
            .respond_with(
                ResponseTemplate::new(405).set_body_string("Required status check missing"),
            )
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let result = client
            .merge_pr("owner", "repo", 2, &MergePrRequest::new(MergeMethod::Merge))
            .await
            .unwrap();
        assert!(matches!(result, MergePrResult::NotMergeable { .. }));
    }

    #[tokio::test]
    async fn merge_pr_conflict_head_mismatch() {
        use crate::github::merge::{MergeMethod, MergePrRequest, MergePrResult};
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .and(path("/repos/owner/repo/pulls/3/merge"))
            .respond_with(ResponseTemplate::new(409).set_body_string("Head branch was modified"))
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let req = MergePrRequest::new(MergeMethod::Merge).with_sha("expected_sha");
        let result = client.merge_pr("owner", "repo", 3, &req).await.unwrap();
        assert!(matches!(result, MergePrResult::HeadMismatch { .. }));
    }

    #[tokio::test]
    async fn merge_pr_conflict_has_conflicts() {
        use crate::github::merge::{MergeMethod, MergePrRequest, MergePrResult};
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .and(path("/repos/owner/repo/pulls/3/merge"))
            .respond_with(ResponseTemplate::new(409).set_body_string("Merge conflict"))
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let result = client
            .merge_pr("owner", "repo", 3, &MergePrRequest::new(MergeMethod::Merge))
            .await
            .unwrap();
        assert!(matches!(result, MergePrResult::HasConflicts));
    }

    #[tokio::test]
    async fn merge_pr_other_error() {
        use crate::github::merge::{MergeMethod, MergePrRequest, MergePrResult};
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .and(path("/repos/owner/repo/pulls/4/merge"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal server error"))
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let result = client
            .merge_pr("owner", "repo", 4, &MergePrRequest::new(MergeMethod::Merge))
            .await
            .unwrap();
        assert!(matches!(result, MergePrResult::Failed { status: 500, .. }));
    }

    #[tokio::test]
    async fn handle_response_403_rate_limited() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/repos/owner/repo"))
            .respond_with(
                ResponseTemplate::new(403)
                    .insert_header("x-ratelimit-limit", "60")
                    .insert_header("x-ratelimit-remaining", "0")
                    .insert_header("x-ratelimit-reset", "1700000000")
                    .set_body_string("rate limit exceeded"),
            )
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let result = client.get_repository("owner", "repo").await;
        assert!(matches!(
            result.unwrap_err(),
            GitHubError::RateLimited { .. }
        ));
    }

    #[tokio::test]
    async fn handle_response_403_not_rate_limited() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/repos/owner/repo"))
            .respond_with(
                ResponseTemplate::new(403)
                    .insert_header("x-ratelimit-limit", "60")
                    .insert_header("x-ratelimit-remaining", "30")
                    .insert_header("x-ratelimit-reset", "1700000000")
                    .set_body_string("forbidden"),
            )
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let result = client.get_repository("owner", "repo").await;
        match result.unwrap_err() {
            GitHubError::ApiError { status, message } => {
                assert_eq!(status, 403);
                assert_eq!(message, "forbidden");
            }
            other => panic!("Expected ApiError, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn get_issue_success() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/issues/42"))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_issue_json(42, "My issue")))
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let issue = client.get_issue("owner", "repo", 42).await.unwrap();
        assert_eq!(issue.number, 42);
        assert_eq!(issue.title, "My issue");
    }

    #[tokio::test]
    async fn get_issue_not_found() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/issues/999"))
            .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let result = client.get_issue("owner", "repo", 999).await;
        assert!(matches!(result.unwrap_err(), GitHubError::NotFound(_)));
    }

    #[tokio::test]
    async fn list_issues_with_since_and_per_page() {
        use wiremock::matchers::{method, path, query_param};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/issues"))
            .and(query_param("per_page", "10"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!([mock_issue_json(1, "Issue")])),
            )
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let filters = IssueFilters::new().since(chrono::Utc::now()).per_page(10);
        let issues = client.list_issues("owner", "repo", &filters).await.unwrap();
        assert_eq!(issues.len(), 1);
    }

    #[tokio::test]
    async fn list_pull_requests_with_state_filter() {
        use wiremock::matchers::{method, path, query_param};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/pulls"))
            .and(query_param("state", "closed"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!([mock_pr_json(5, "Closed PR")])),
            )
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let prs = client
            .list_pull_requests("owner", "repo", Some("closed"))
            .await
            .unwrap();
        assert_eq!(prs.len(), 1);
        assert_eq!(prs[0].title, "Closed PR");
    }

    #[tokio::test]
    async fn get_pr_change_summary_counts_statuses() {
        use wiremock::matchers::{method, path, query_param};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;

        // Page 1 with various file statuses
        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/pulls/1/files"))
            .and(query_param("page", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                { "filename": "new.rs", "status": "added", "additions": 10, "deletions": 0, "changes": 10 },
                { "filename": "old.rs", "status": "removed", "additions": 0, "deletions": 5, "changes": 5 },
                { "filename": "mod.rs", "status": "modified", "additions": 3, "deletions": 1, "changes": 4 },
                { "filename": "ren.rs", "status": "renamed", "additions": 0, "deletions": 0, "changes": 0, "previous_filename": "old_name.rs" },
                { "filename": "chg.rs", "status": "changed", "additions": 1, "deletions": 1, "changes": 2 },
                { "filename": "cp.rs", "status": "copied", "additions": 5, "deletions": 0, "changes": 5 },
                { "filename": "unch.rs", "status": "unchanged", "additions": 0, "deletions": 0, "changes": 0 }
            ])))
            .mount(&server)
            .await;

        // Page 2 empty
        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/pulls/1/files"))
            .and(query_param("page", "2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let summary = client
            .get_pr_change_summary("owner", "repo", 1)
            .await
            .unwrap();

        assert_eq!(summary.total_files, 7);
        assert_eq!(summary.files_added, 1);
        assert_eq!(summary.files_removed, 1);
        assert_eq!(summary.files_modified, 2); // modified + changed
        assert_eq!(summary.files_renamed, 1);
        assert_eq!(summary.additions, 19);
        assert_eq!(summary.deletions, 7);
    }

    #[tokio::test]
    async fn approve_pr_success() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/repos/owner/repo/pulls/5/reviews"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(mock_review_json(20, "APPROVED")),
            )
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let review = client
            .approve_pr("owner", "repo", 5, Some("LGTM"))
            .await
            .unwrap();
        assert_eq!(review.state, crate::github::types::ReviewState::Approved);
    }

    #[tokio::test]
    async fn approve_pr_without_comment() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/repos/owner/repo/pulls/5/reviews"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(mock_review_json(21, "APPROVED")),
            )
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let review = client.approve_pr("owner", "repo", 5, None).await.unwrap();
        assert_eq!(review.id, 21);
    }

    #[tokio::test]
    async fn request_pr_changes_success() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/repos/owner/repo/pulls/6/reviews"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(mock_review_json(30, "CHANGES_REQUESTED")),
            )
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let review = client
            .request_pr_changes("owner", "repo", 6, "Please fix")
            .await
            .unwrap();
        assert_eq!(
            review.state,
            crate::github::types::ReviewState::ChangesRequested
        );
    }

    #[tokio::test]
    async fn merge_pr_simple_delegates() {
        use crate::github::merge::{MergeMethod, MergePrResult};
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .and(path("/repos/owner/repo/pulls/10/merge"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "sha": "def456",
                "merged": true,
                "message": "Merged"
            })))
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let result = client
            .merge_pr_simple("owner", "repo", 10, MergeMethod::Rebase)
            .await
            .unwrap();
        assert!(matches!(result, MergePrResult::Merged { sha, .. } if sha == "def456"));
    }

    #[tokio::test]
    async fn get_mergeable_status_merged() {
        use crate::github::merge::MergeableStatus;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        let mut pr = mock_pr_json(1, "PR");
        pr.as_object_mut()
            .unwrap()
            .insert("merged".into(), serde_json::json!(true));
        pr.as_object_mut()
            .unwrap()
            .insert("state".into(), serde_json::json!("closed"));

        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/pulls/1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(pr))
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let status = client
            .get_mergeable_status("owner", "repo", 1)
            .await
            .unwrap();
        assert_eq!(status, MergeableStatus::Merged);
    }

    #[tokio::test]
    async fn get_mergeable_status_closed() {
        use crate::github::merge::MergeableStatus;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        let mut pr = mock_pr_json(2, "Closed PR");
        pr.as_object_mut()
            .unwrap()
            .insert("state".into(), serde_json::json!("closed"));
        pr.as_object_mut()
            .unwrap()
            .insert("merged".into(), serde_json::json!(false));

        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/pulls/2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(pr))
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let status = client
            .get_mergeable_status("owner", "repo", 2)
            .await
            .unwrap();
        assert_eq!(status, MergeableStatus::Closed);
    }

    #[tokio::test]
    async fn get_mergeable_status_mergeable() {
        use crate::github::merge::MergeableStatus;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        let mut pr = mock_pr_json(3, "Mergeable PR");
        pr.as_object_mut()
            .unwrap()
            .insert("mergeable".into(), serde_json::json!(true));
        pr.as_object_mut()
            .unwrap()
            .insert("mergeable_state".into(), serde_json::json!("clean"));

        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/pulls/3"))
            .respond_with(ResponseTemplate::new(200).set_body_json(pr))
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let status = client
            .get_mergeable_status("owner", "repo", 3)
            .await
            .unwrap();
        assert_eq!(status, MergeableStatus::Mergeable);
    }

    #[tokio::test]
    async fn get_mergeable_status_has_conflicts() {
        use crate::github::merge::MergeableStatus;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        let mut pr = mock_pr_json(4, "Conflicted PR");
        pr.as_object_mut()
            .unwrap()
            .insert("mergeable".into(), serde_json::json!(false));
        pr.as_object_mut()
            .unwrap()
            .insert("mergeable_state".into(), serde_json::json!("dirty"));

        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/pulls/4"))
            .respond_with(ResponseTemplate::new(200).set_body_json(pr))
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let status = client
            .get_mergeable_status("owner", "repo", 4)
            .await
            .unwrap();
        assert_eq!(status, MergeableStatus::HasConflicts);
    }

    #[tokio::test]
    async fn get_mergeable_status_blocked() {
        use crate::github::merge::MergeableStatus;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        let mut pr = mock_pr_json(5, "Blocked PR");
        pr.as_object_mut()
            .unwrap()
            .insert("mergeable".into(), serde_json::json!(false));
        pr.as_object_mut()
            .unwrap()
            .insert("mergeable_state".into(), serde_json::json!("blocked"));

        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/pulls/5"))
            .respond_with(ResponseTemplate::new(200).set_body_json(pr))
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let status = client
            .get_mergeable_status("owner", "repo", 5)
            .await
            .unwrap();
        assert_eq!(
            status,
            MergeableStatus::Blocked {
                reason: "blocked".to_string()
            }
        );
    }

    #[tokio::test]
    async fn get_mergeable_status_blocked_no_state() {
        use crate::github::merge::MergeableStatus;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        let mut pr = mock_pr_json(6, "Blocked PR no state");
        pr.as_object_mut()
            .unwrap()
            .insert("mergeable".into(), serde_json::json!(false));

        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/pulls/6"))
            .respond_with(ResponseTemplate::new(200).set_body_json(pr))
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let status = client
            .get_mergeable_status("owner", "repo", 6)
            .await
            .unwrap();
        assert_eq!(
            status,
            MergeableStatus::Blocked {
                reason: "unknown".to_string()
            }
        );
    }

    #[tokio::test]
    async fn get_mergeable_status_unknown() {
        use crate::github::merge::MergeableStatus;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        // mergeable is null by default (not present in mock_pr_json)
        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/pulls/7"))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_pr_json(7, "Unknown PR")))
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let status = client
            .get_mergeable_status("owner", "repo", 7)
            .await
            .unwrap();
        assert_eq!(status, MergeableStatus::Unknown);
    }

    #[tokio::test]
    async fn handle_response_403_no_rate_limit_headers_defaults_to_rate_limited() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/repos/owner/repo"))
            .respond_with(ResponseTemplate::new(403).set_body_string("forbidden no headers"))
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let result = client.get_repository("owner", "repo").await;
        // With no rate limit headers, remaining defaults to 0 which triggers RateLimited
        assert!(matches!(
            result.unwrap_err(),
            GitHubError::RateLimited { .. }
        ));
    }

    #[tokio::test]
    async fn check_rate_limit_with_valid_headers() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/repos/owner/repo"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("x-ratelimit-limit", "5000")
                    .insert_header("x-ratelimit-remaining", "4999")
                    .insert_header("x-ratelimit-reset", "1700000000")
                    .set_body_json(serde_json::json!({
                        "id": 1,
                        "name": "repo",
                        "full_name": "owner/repo",
                        "private": false,
                        "owner": { "login": "owner", "id": 1 },
                        "html_url": "https://github.com/owner/repo",
                        "description": "A test repo",
                        "default_branch": "main",
                        "clone_url": "https://github.com/owner/repo.git"
                    })),
            )
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        // Just verifying the request succeeds with rate limit headers present
        let repo = client.get_repository("owner", "repo").await.unwrap();
        assert_eq!(repo.name, "repo");
    }

    #[tokio::test]
    async fn create_pull_request_unauthorized() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/repos/owner/repo/pulls"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let result = client
            .create_pull_request(
                "owner",
                "repo",
                &CreatePullRequest {
                    title: "PR".into(),
                    body: "body".into(),
                    head: "feature".into(),
                    base: "main".into(),
                    draft: None,
                },
            )
            .await;
        assert!(matches!(result.unwrap_err(), GitHubError::Unauthorized));
    }

    #[tokio::test]
    async fn handle_response_generic_error() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/repos/owner/repo"))
            .respond_with(ResponseTemplate::new(422).set_body_string("Validation failed"))
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let result = client.get_repository("owner", "repo").await;
        match result.unwrap_err() {
            GitHubError::ApiError { status, message } => {
                assert_eq!(status, 422);
                assert_eq!(message, "Validation failed");
            }
            other => panic!("Expected ApiError, got {:?}", other),
        }
    }
}
