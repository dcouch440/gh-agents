//! Pull request creation and management

use crate::execution::BranchInfo;
use crate::github::{CreatePullRequest, GitHubClient, GitHubError, GitHubPullRequest};
use crate::types::{Ticket, TicketSource, VerticalSlice};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PrError {
    #[error("cannot create PR for manually created ticket - no GitHub repository associated")]
    ManualTicket,

    #[error("github API error: {0}")]
    GitHubError(#[from] GitHubError),

    #[error("PR creation failed: {0}")]
    CreationFailed(String),
}

/// Result of creating a PR
#[derive(Debug, Clone)]
pub struct PrResult {
    pub number: u32,
    pub url: String,
    pub title: String,
}

impl std::fmt::Display for PrResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PR #{}: {} ({})", self.number, self.title, self.url)
    }
}

/// Generate PR body from slice information
pub struct PrBodyGenerator {
    summary: String,
    tasks: Vec<String>,
    files_modified: Vec<String>,
    issue_ref: Option<(String, String, u32)>, // (owner, repo, number)
}

impl PrBodyGenerator {
    pub fn new(summary: impl Into<String>) -> Self {
        Self {
            summary: summary.into(),
            tasks: Vec::new(),
            files_modified: Vec::new(),
            issue_ref: None,
        }
    }

    pub fn from_slice(slice: &VerticalSlice) -> Self {
        Self::new(&slice.description)
    }

    pub fn with_tasks(mut self, tasks: Vec<String>) -> Self {
        self.tasks = tasks;
        self
    }

    pub fn with_files(mut self, files: Vec<String>) -> Self {
        self.files_modified = files;
        self
    }

    pub fn with_issue(mut self, owner: impl Into<String>, repo: impl Into<String>, number: u32) -> Self {
        self.issue_ref = Some((owner.into(), repo.into(), number));
        self
    }

    pub fn from_ticket_source(slice: &VerticalSlice, source: &TicketSource) -> Self {
        let mut gen = Self::from_slice(slice);

        if let TicketSource::GitHub { owner, repo, issue_number } = source {
            gen = gen.with_issue(owner, repo, *issue_number);
        }

        gen
    }

    pub fn generate(&self) -> String {
        let mut body = String::new();

        // Summary section
        body.push_str("## Summary\n\n");
        body.push_str(&self.summary);
        body.push_str("\n\n");

        // Tasks completed
        if !self.tasks.is_empty() {
            body.push_str("## Changes\n\n");
            for task in &self.tasks {
                body.push_str(&format!("- {}\n", task));
            }
            body.push('\n');
        }

        // Files modified
        if !self.files_modified.is_empty() {
            body.push_str("## Files Modified\n\n");
            for file in &self.files_modified {
                body.push_str(&format!("- `{}`\n", file));
            }
            body.push('\n');
        }

        // Issue linking
        if let Some((owner, repo, number)) = &self.issue_ref {
            body.push_str(&format!("Fixes {}/{}#{}\n\n", owner, repo, number));
        }

        // Attribution
        body.push_str("---\n\n");
        body.push_str("*Created by [nexor](https://github.com/nexor) AI agents*\n");

        body
    }
}

/// Service for creating pull requests
pub struct PrService {
    client: GitHubClient,
    fallback_base: String,
}

impl PrService {
    pub fn new(client: GitHubClient) -> Self {
        Self {
            client,
            fallback_base: "main".to_string(),
        }
    }

    /// Set the fallback base branch (used when parent branch is unknown)
    pub fn with_fallback_base(mut self, base: impl Into<String>) -> Self {
        self.fallback_base = base.into();
        self
    }

    /// Create a PR for a completed slice
    pub async fn create_pr_for_slice(&self, slice: &VerticalSlice, ticket: &Ticket, branch_info: &BranchInfo, files_modified: Vec<String>) -> Result<PrResult, PrError> {
        // Extract repo info from ticket source
        let (owner, repo, issue_number) = match &ticket.source {
            TicketSource::GitHub { owner, repo, issue_number } => (owner.clone(), repo.clone(), Some(*issue_number)),
            TicketSource::Manual => {
                return Err(PrError::ManualTicket);
            }
        };

        // Generate PR title
        let title = slice.title.clone();

        // Determine base branch: use parent branch if known, otherwise fallback
        let base_branch = branch_info.parent_branch.as_ref().cloned().unwrap_or_else(|| self.fallback_base.clone());

        // Generate PR body
        let mut body_gen = PrBodyGenerator::from_slice(slice).with_files(files_modified);

        if let Some(num) = issue_number {
            body_gen = body_gen.with_issue(&owner, &repo, num);
        }

        let body = body_gen.generate();

        // Create the PR request
        let request = CreatePullRequest {
            title: title.clone(),
            body,
            head: branch_info.name.clone(),
            base: base_branch.clone(),
            draft: None,
        };

        tracing::info!(
            owner = %owner,
            repo = %repo,
            branch = %branch_info.name,
            base = %base_branch,
            "Creating pull request"
        );

        let pr: GitHubPullRequest = self.client.create_pull_request(&owner, &repo, &request).await?;

        tracing::info!(
            pr_number = pr.number,
            url = %pr.html_url,
            "Pull request created"
        );

        Ok(PrResult {
            number: pr.number,
            url: pr.html_url,
            title: pr.title,
        })
    }

    /// Create a simple PR without slice context
    pub async fn create_simple_pr(&self, owner: &str, repo: &str, title: &str, body: &str, head: &str, base: &str) -> Result<PrResult, PrError> {
        let request = CreatePullRequest {
            title: title.to_string(),
            body: body.to_string(),
            head: head.to_string(),
            base: base.to_string(),
            draft: None,
        };

        let pr = self.client.create_pull_request(owner, repo, &request).await?;

        Ok(PrResult {
            number: pr.number,
            url: pr.html_url,
            title: pr.title,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{SliceId, TaskStatus, TicketId};
    use chrono::Utc;

    fn mock_slice() -> VerticalSlice {
        VerticalSlice {
            id: SliceId::new(),
            ticket_id: uuid::Uuid::new_v4(),
            title: "Add user authentication".to_string(),
            description: "Implements basic auth flow with JWT tokens".to_string(),
            tasks: vec![],
            status: TaskStatus::Completed,
            created_at: Utc::now(),
        }
    }

    fn mock_ticket_github() -> Ticket {
        Ticket {
            id: TicketId::new(),
            source: TicketSource::GitHub {
                owner: "owner".to_string(),
                repo: "repo".to_string(),
                issue_number: 42,
            },
            title: "Feature request".to_string(),
            description: "Add auth".to_string(),
            labels: vec![],
            slices: vec![],
            status: crate::types::TicketStatus::New,
            created_at: Utc::now(),
        }
    }

    fn mock_ticket_manual() -> Ticket {
        Ticket {
            id: TicketId::new(),
            source: TicketSource::Manual,
            title: "Manual ticket".to_string(),
            description: "Manual".to_string(),
            labels: vec![],
            slices: vec![],
            status: crate::types::TicketStatus::New,
            created_at: Utc::now(),
        }
    }

    fn mock_branch_info() -> BranchInfo {
        BranchInfo {
            name: "feature/auth".to_string(),
            parent_branch: Some("main".to_string()),
            base_commit: "abc123".to_string(),
        }
    }

    #[test]
    fn pr_body_generator_basic() {
        let slice = mock_slice();
        let body = PrBodyGenerator::from_slice(&slice).generate();

        assert!(body.contains("## Summary"));
        assert!(body.contains("Implements basic auth flow"));
        assert!(body.contains("nexor"));
    }

    #[test]
    fn pr_body_generator_with_issue_link() {
        let slice = mock_slice();
        let body = PrBodyGenerator::from_slice(&slice).with_issue("owner", "repo", 42).generate();

        assert!(body.contains("Fixes owner/repo#42"));
    }

    #[test]
    fn pr_body_generator_with_files() {
        let slice = mock_slice();
        let body = PrBodyGenerator::from_slice(&slice).with_files(vec!["src/auth.rs".to_string(), "src/main.rs".to_string()]).generate();

        assert!(body.contains("## Files Modified"));
        assert!(body.contains("`src/auth.rs`"));
        assert!(body.contains("`src/main.rs`"));
    }

    #[test]
    fn pr_body_generator_with_tasks() {
        let body = PrBodyGenerator::new("Summary here").with_tasks(vec!["Task 1".to_string(), "Task 2".to_string()]).generate();

        assert!(body.contains("## Changes"));
        assert!(body.contains("- Task 1"));
        assert!(body.contains("- Task 2"));
    }

    #[test]
    fn pr_body_generator_from_ticket_source() {
        let slice = mock_slice();
        let ticket = mock_ticket_github();
        let body = PrBodyGenerator::from_ticket_source(&slice, &ticket.source).generate();

        assert!(body.contains("Fixes owner/repo#42"));
    }

    #[test]
    fn pr_result_display() {
        let result = PrResult {
            number: 123,
            url: "https://github.com/owner/repo/pull/123".to_string(),
            title: "Test PR".to_string(),
        };

        let display = format!("{}", result);
        assert!(display.contains("PR #123"));
        assert!(display.contains("Test PR"));
        assert!(display.contains("https://github.com"));
    }

    #[test]
    fn pr_body_generator_full_builder_chain_from_new() {
        let body = PrBodyGenerator::new("Implement login endpoint")
            .with_tasks(vec!["Add login route".to_string(), "Add JWT middleware".to_string()])
            .with_files(vec!["src/routes/login.rs".to_string(), "src/middleware/auth.rs".to_string()])
            .with_issue("myorg", "myrepo", 99)
            .generate();

        assert!(body.contains("## Summary"));
        assert!(body.contains("Implement login endpoint"));
        assert!(body.contains("## Changes"));
        assert!(body.contains("- Add login route"));
        assert!(body.contains("- Add JWT middleware"));
        assert!(body.contains("## Files Modified"));
        assert!(body.contains("`src/routes/login.rs`"));
        assert!(body.contains("`src/middleware/auth.rs`"));
        assert!(body.contains("Fixes myorg/myrepo#99"));
        assert!(body.contains("nexor"));
    }

    #[test]
    fn pr_body_generator_full_builder_chain_from_slice() {
        let slice = mock_slice();
        let body = PrBodyGenerator::from_slice(&slice)
            .with_tasks(vec!["Implement auth".to_string()])
            .with_files(vec!["src/auth.rs".to_string()])
            .with_issue("owner", "repo", 10)
            .generate();

        assert!(body.contains("Implements basic auth flow with JWT tokens"));
        assert!(body.contains("## Changes"));
        assert!(body.contains("- Implement auth"));
        assert!(body.contains("## Files Modified"));
        assert!(body.contains("`src/auth.rs`"));
        assert!(body.contains("Fixes owner/repo#10"));
    }

    #[test]
    fn pr_body_generator_from_ticket_source_manual_no_issue() {
        let slice = mock_slice();
        let ticket = mock_ticket_manual();
        let body = PrBodyGenerator::from_ticket_source(&slice, &ticket.source).generate();

        assert!(body.contains("## Summary"));
        assert!(body.contains("Implements basic auth flow"));
        assert!(!body.contains("Fixes"));
    }

    #[test]
    fn pr_body_generator_combined_tasks_files_issue() {
        let slice = mock_slice();
        let ticket = mock_ticket_github();
        let body = PrBodyGenerator::from_ticket_source(&slice, &ticket.source)
            .with_tasks(vec!["Task A".to_string(), "Task B".to_string(), "Task C".to_string()])
            .with_files(vec!["file1.rs".to_string(), "file2.rs".to_string()])
            .generate();

        // Verify all sections present and ordered
        let summary_pos = body.find("## Summary").unwrap();
        let changes_pos = body.find("## Changes").unwrap();
        let files_pos = body.find("## Files Modified").unwrap();
        let fixes_pos = body.find("Fixes owner/repo#42").unwrap();

        assert!(summary_pos < changes_pos);
        assert!(changes_pos < files_pos);
        assert!(files_pos < fixes_pos);

        assert!(body.contains("- Task A"));
        assert!(body.contains("- Task B"));
        assert!(body.contains("- Task C"));
        assert!(body.contains("`file1.rs`"));
        assert!(body.contains("`file2.rs`"));
    }

    #[test]
    fn pr_error_display_manual_ticket() {
        let err = PrError::ManualTicket;
        assert_eq!(err.to_string(), "cannot create PR for manually created ticket - no GitHub repository associated");
    }

    #[test]
    fn pr_error_display_creation_failed() {
        let err = PrError::CreationFailed("branch not found".to_string());
        assert_eq!(err.to_string(), "PR creation failed: branch not found");
    }

    #[test]
    fn pr_error_from_github_error() {
        let gh_err = GitHubError::Unauthorized;
        let pr_err: PrError = gh_err.into();
        assert!(matches!(pr_err, PrError::GitHubError(_)));
        assert!(pr_err.to_string().contains("github API error"));
    }

    #[test]
    fn pr_service_with_fallback_base() {
        let client = GitHubClient::with_token("test").unwrap();
        let svc = PrService::new(client).with_fallback_base("develop");
        assert_eq!(svc.fallback_base, "develop");
    }

    #[test]
    fn pr_service_default_fallback_base() {
        let client = GitHubClient::with_token("test").unwrap();
        let svc = PrService::new(client);
        assert_eq!(svc.fallback_base, "main");
    }

    #[test]
    fn pr_body_generator_empty_no_tasks_no_files_no_issue() {
        let body = PrBodyGenerator::new("Just a summary").generate();
        assert!(body.contains("## Summary"));
        assert!(body.contains("Just a summary"));
        assert!(!body.contains("## Changes"));
        assert!(!body.contains("## Files Modified"));
        assert!(!body.contains("Fixes"));
        assert!(body.contains("nexor"));
    }

    #[tokio::test]
    async fn create_pr_for_slice_manual_ticket_returns_error() {
        let client = GitHubClient::with_token("test").unwrap();
        let svc = PrService::new(client);
        let slice = mock_slice();
        let ticket = mock_ticket_manual();
        let branch = mock_branch_info();

        let result = svc.create_pr_for_slice(&slice, &ticket, &branch, vec![]).await;
        assert!(matches!(result.unwrap_err(), PrError::ManualTicket));
    }

    #[tokio::test]
    async fn create_pr_for_slice_success() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/repos/owner/repo/pulls"))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "id": 1,
                "number": 55,
                "title": "Add user authentication",
                "state": "open",
                "user": { "login": "bot", "id": 1 },
                "html_url": "https://github.com/owner/repo/pull/55",
                "body": "pr body",
                "head": { "ref": "feature/auth", "sha": "aaa" },
                "base": { "ref": "main", "sha": "bbb" },
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-01-01T00:00:00Z"
            })))
            .mount(&server)
            .await;

        let client = GitHubClient::with_token("test").unwrap().with_base_url(server.uri());
        let svc = PrService::new(client);
        let slice = mock_slice();
        let ticket = mock_ticket_github();
        let branch = mock_branch_info();

        let result = svc.create_pr_for_slice(&slice, &ticket, &branch, vec!["src/auth.rs".to_string()]).await.unwrap();

        assert_eq!(result.number, 55);
        assert_eq!(result.url, "https://github.com/owner/repo/pull/55");
        assert_eq!(result.title, "Add user authentication");
    }

    #[tokio::test]
    async fn create_pr_for_slice_uses_fallback_when_no_parent_branch() {
        use wiremock::matchers::{body_string_contains, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/repos/owner/repo/pulls"))
            .and(body_string_contains("\"base\":\"develop\""))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "id": 1,
                "number": 60,
                "title": "Add user authentication",
                "state": "open",
                "user": { "login": "bot", "id": 1 },
                "html_url": "https://github.com/owner/repo/pull/60",
                "body": "body",
                "head": { "ref": "feature/auth", "sha": "aaa" },
                "base": { "ref": "develop", "sha": "bbb" },
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-01-01T00:00:00Z"
            })))
            .mount(&server)
            .await;

        let client = GitHubClient::with_token("test").unwrap().with_base_url(server.uri());
        let svc = PrService::new(client).with_fallback_base("develop");
        let slice = mock_slice();
        let ticket = mock_ticket_github();
        let branch = BranchInfo {
            name: "feature/auth".to_string(),
            parent_branch: None,
            base_commit: "abc123".to_string(),
        };

        let result = svc.create_pr_for_slice(&slice, &ticket, &branch, vec![]).await.unwrap();
        assert_eq!(result.number, 60);
    }

    #[tokio::test]
    async fn create_pr_for_slice_api_error() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/repos/owner/repo/pulls"))
            .respond_with(ResponseTemplate::new(422).set_body_string("Validation Failed"))
            .mount(&server)
            .await;

        let client = GitHubClient::with_token("test").unwrap().with_base_url(server.uri());
        let svc = PrService::new(client);
        let slice = mock_slice();
        let ticket = mock_ticket_github();
        let branch = mock_branch_info();

        let result = svc.create_pr_for_slice(&slice, &ticket, &branch, vec![]).await;
        assert!(matches!(result.unwrap_err(), PrError::GitHubError(_)));
    }

    #[tokio::test]
    async fn create_simple_pr_success() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/repos/org/project/pulls"))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "id": 1,
                "number": 77,
                "title": "Quick fix",
                "state": "open",
                "user": { "login": "dev", "id": 1 },
                "html_url": "https://github.com/org/project/pull/77",
                "body": "fix stuff",
                "head": { "ref": "hotfix", "sha": "aaa" },
                "base": { "ref": "main", "sha": "bbb" },
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-01-01T00:00:00Z"
            })))
            .mount(&server)
            .await;

        let client = GitHubClient::with_token("test").unwrap().with_base_url(server.uri());
        let svc = PrService::new(client);

        let result = svc.create_simple_pr("org", "project", "Quick fix", "fix stuff", "hotfix", "main").await.unwrap();

        assert_eq!(result.number, 77);
        assert_eq!(result.title, "Quick fix");
        assert_eq!(result.url, "https://github.com/org/project/pull/77");
    }

    #[tokio::test]
    async fn create_simple_pr_api_error() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/repos/org/project/pulls"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;

        let client = GitHubClient::with_token("test").unwrap().with_base_url(server.uri());
        let svc = PrService::new(client);

        let result = svc.create_simple_pr("org", "project", "title", "body", "head", "base").await;
        assert!(matches!(result.unwrap_err(), PrError::GitHubError(_)));
    }

    #[test]
    fn pr_result_debug() {
        let result = PrResult {
            number: 1,
            url: "url".to_string(),
            title: "t".to_string(),
        };
        let debug = format!("{:?}", result);
        assert!(debug.contains("PrResult"));
    }

    // Note: Integration tests with actual API calls would go in tests/ directory
    // These unit tests verify the local logic only
}
