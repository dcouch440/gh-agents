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
            .map_err(|e| GitHubError::ConfigError(format!("Failed to create HTTP client: {}", e)))?;

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
            s if s.is_success() => response
                .json()
                .await
                .map_err(|e| GitHubError::RequestFailed(format!("Failed to parse response: {}", e))),

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
        let url = format!("{}/repos/{}/{}/pulls/{}", self.base_url, owner, repo, number);

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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_requires_token() {
        // Clear any existing token from env
        std::env::remove_var("GITHUB_TOKEN");

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
}
