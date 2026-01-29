//! GitHub issue synchronization

use crate::github::{GitHubClient, GitHubError, GitHubIssue};
use crate::types::{Ticket, TicketId, TicketSource, TicketStatus};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum IssueSyncError {
    #[error("invalid issue URL format: {0}")]
    InvalidUrl(String),

    #[error("failed to parse issue number: {0}")]
    InvalidNumber(String),

    #[error("github API error: {0}")]
    GitHubError(#[from] GitHubError),
}

/// Reference to a GitHub issue
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssueRef {
    pub owner: String,
    pub repo: String,
    pub number: u32,
}

impl IssueRef {
    pub fn new(owner: impl Into<String>, repo: impl Into<String>, number: u32) -> Self {
        Self {
            owner: owner.into(),
            repo: repo.into(),
            number,
        }
    }

    /// Parse various GitHub issue URL formats
    pub fn parse(url: &str) -> Result<Self, IssueSyncError> {
        let url = url.trim();

        // Format: https://github.com/owner/repo/issues/123
        if url.contains("github.com") && url.contains("/issues/") {
            return Self::parse_github_url(url);
        }

        // Format: owner/repo#123
        if url.contains('#') && url.contains('/') {
            return Self::parse_short_format(url);
        }

        Err(IssueSyncError::InvalidUrl(url.to_string()))
    }

    fn parse_github_url(url: &str) -> Result<Self, IssueSyncError> {
        // Remove protocol and host prefix
        let path = url
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_start_matches("github.com/");

        // Expected: owner/repo/issues/123
        let parts: Vec<&str> = path.split('/').collect();

        if parts.len() >= 4 && parts[2] == "issues" {
            let number = parts[3]
                .parse::<u32>()
                .map_err(|_| IssueSyncError::InvalidNumber(parts[3].to_string()))?;

            return Ok(Self {
                owner: parts[0].to_string(),
                repo: parts[1].to_string(),
                number,
            });
        }

        Err(IssueSyncError::InvalidUrl(url.to_string()))
    }

    fn parse_short_format(url: &str) -> Result<Self, IssueSyncError> {
        // Format: owner/repo#123
        let parts: Vec<&str> = url.splitn(2, '#').collect();
        if parts.len() != 2 {
            return Err(IssueSyncError::InvalidUrl(url.to_string()));
        }

        let repo_parts: Vec<&str> = parts[0].split('/').collect();
        if repo_parts.len() != 2 {
            return Err(IssueSyncError::InvalidUrl(url.to_string()));
        }

        let number = parts[1]
            .parse::<u32>()
            .map_err(|_| IssueSyncError::InvalidNumber(parts[1].to_string()))?;

        Ok(Self {
            owner: repo_parts[0].to_string(),
            repo: repo_parts[1].to_string(),
            number,
        })
    }

    /// Convert to a full GitHub URL
    pub fn to_url(&self) -> String {
        format!(
            "https://github.com/{}/{}/issues/{}",
            self.owner, self.repo, self.number
        )
    }
}

/// Convert a GitHub issue to an internal Ticket
pub fn convert_issue_to_ticket(issue: GitHubIssue, owner: &str, repo: &str) -> Ticket {
    Ticket {
        id: TicketId::new(),
        source: TicketSource::GitHub {
            owner: owner.to_string(),
            repo: repo.to_string(),
            issue_number: issue.number,
        },
        title: issue.title,
        description: issue.body.unwrap_or_default(),
        labels: issue.labels.into_iter().map(|l| l.name).collect(),
        slices: Vec::new(), // Will be populated by orchestrator
        status: TicketStatus::New,
        created_at: issue.created_at,
    }
}

/// Result of syncing an issue
#[derive(Debug)]
pub enum SyncResult {
    Created(Ticket),
    Updated(Ticket),
}

impl SyncResult {
    pub fn ticket(&self) -> &Ticket {
        match self {
            SyncResult::Created(t) => t,
            SyncResult::Updated(t) => t,
        }
    }

    pub fn is_new(&self) -> bool {
        matches!(self, SyncResult::Created(_))
    }

    pub fn into_ticket(self) -> Ticket {
        match self {
            SyncResult::Created(t) => t,
            SyncResult::Updated(t) => t,
        }
    }
}

/// Service for syncing GitHub issues to internal tickets
pub struct IssueSync {
    client: GitHubClient,
}

impl IssueSync {
    pub fn new(client: GitHubClient) -> Self {
        Self { client }
    }

    /// Fetch and convert a GitHub issue to a Ticket
    pub async fn fetch_issue(&self, issue_ref: &IssueRef) -> Result<Ticket, IssueSyncError> {
        let github_issue = self
            .client
            .get_issue(&issue_ref.owner, &issue_ref.repo, issue_ref.number)
            .await?;

        let ticket = convert_issue_to_ticket(github_issue, &issue_ref.owner, &issue_ref.repo);

        tracing::info!(
            owner = %issue_ref.owner,
            repo = %issue_ref.repo,
            number = issue_ref.number,
            title = %ticket.title,
            "Fetched issue from GitHub"
        );

        Ok(ticket)
    }

    /// Fetch issue by URL string
    pub async fn fetch_issue_by_url(&self, url: &str) -> Result<Ticket, IssueSyncError> {
        let issue_ref = IssueRef::parse(url)?;
        self.fetch_issue(&issue_ref).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::{GitHubApiUser, GitHubLabel};
    use chrono::Utc;

    #[test]
    fn parse_full_github_url() {
        let url = "https://github.com/anthropics/claude-code/issues/42";
        let issue_ref = IssueRef::parse(url).unwrap();
        assert_eq!(issue_ref.owner, "anthropics");
        assert_eq!(issue_ref.repo, "claude-code");
        assert_eq!(issue_ref.number, 42);
    }

    #[test]
    fn parse_url_without_https() {
        let url = "github.com/owner/repo/issues/123";
        let issue_ref = IssueRef::parse(url).unwrap();
        assert_eq!(issue_ref.owner, "owner");
        assert_eq!(issue_ref.repo, "repo");
        assert_eq!(issue_ref.number, 123);
    }

    #[test]
    fn parse_short_format() {
        let url = "owner/repo#456";
        let issue_ref = IssueRef::parse(url).unwrap();
        assert_eq!(issue_ref.owner, "owner");
        assert_eq!(issue_ref.repo, "repo");
        assert_eq!(issue_ref.number, 456);
    }

    #[test]
    fn parse_url_with_whitespace() {
        let url = "  https://github.com/owner/repo/issues/1  ";
        let issue_ref = IssueRef::parse(url).unwrap();
        assert_eq!(issue_ref.number, 1);
    }

    #[test]
    fn parse_invalid_url() {
        assert!(IssueRef::parse("not a valid url").is_err());
        assert!(IssueRef::parse("https://example.com/issues/1").is_err());
        assert!(IssueRef::parse("owner/repo/123").is_err()); // Missing #
    }

    #[test]
    fn issue_ref_to_url() {
        let issue_ref = IssueRef::new("owner", "repo", 42);
        assert_eq!(
            issue_ref.to_url(),
            "https://github.com/owner/repo/issues/42"
        );
    }

    #[test]
    fn convert_issue_with_body() {
        let issue = GitHubIssue {
            number: 42,
            title: "Test issue".to_string(),
            body: Some("Issue description".to_string()),
            state: "open".to_string(),
            labels: vec![GitHubLabel {
                name: "bug".to_string(),
                color: "ff0000".to_string(),
                description: None,
            }],
            user: GitHubApiUser {
                login: "user".to_string(),
                id: 1,
                avatar_url: None,
            },
            assignees: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            html_url: "https://github.com/owner/repo/issues/42".to_string(),
            pull_request: None,
        };

        let ticket = convert_issue_to_ticket(issue, "owner", "repo");

        assert_eq!(ticket.title, "Test issue");
        assert_eq!(ticket.description, "Issue description");
        assert_eq!(ticket.labels, vec!["bug"]);
        assert!(matches!(
            ticket.source,
            TicketSource::GitHub {
                issue_number: 42,
                ..
            }
        ));
        assert_eq!(ticket.status, TicketStatus::New);
        assert!(ticket.slices.is_empty());
    }

    #[test]
    fn convert_issue_without_body() {
        let issue = GitHubIssue {
            number: 1,
            title: "No body".to_string(),
            body: None,
            state: "open".to_string(),
            labels: vec![],
            user: GitHubApiUser {
                login: "user".to_string(),
                id: 1,
                avatar_url: None,
            },
            assignees: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            html_url: "https://github.com/owner/repo/issues/1".to_string(),
            pull_request: None,
        };

        let ticket = convert_issue_to_ticket(issue, "owner", "repo");
        assert_eq!(ticket.description, "");
    }

    #[test]
    fn sync_result_accessors() {
        let ticket = Ticket {
            id: TicketId::new(),
            source: TicketSource::Manual,
            title: "Test".to_string(),
            description: "".to_string(),
            labels: vec![],
            slices: vec![],
            status: TicketStatus::New,
            created_at: Utc::now(),
        };

        let created = SyncResult::Created(ticket.clone());
        assert!(created.is_new());
        assert_eq!(created.ticket().title, "Test");

        let updated = SyncResult::Updated(ticket);
        assert!(!updated.is_new());
    }

    #[test]
    fn parse_http_url() {
        let url = "http://github.com/owner/repo/issues/99";
        let issue_ref = IssueRef::parse(url).unwrap();
        assert_eq!(issue_ref.owner, "owner");
        assert_eq!(issue_ref.repo, "repo");
        assert_eq!(issue_ref.number, 99);
    }

    #[test]
    fn parse_invalid_issue_number_in_url() {
        let url = "https://github.com/owner/repo/issues/abc";
        let err = IssueRef::parse(url).unwrap_err();
        assert!(matches!(err, IssueSyncError::InvalidNumber(_)));
        assert!(err.to_string().contains("abc"));
    }

    #[test]
    fn parse_invalid_issue_number_in_short_format() {
        let err = IssueRef::parse("owner/repo#notanumber").unwrap_err();
        assert!(matches!(err, IssueSyncError::InvalidNumber(_)));
    }

    #[test]
    fn parse_short_format_missing_owner() {
        // No slash means it won't have 2 repo_parts
        let err = IssueRef::parse("repo#123").unwrap_err();
        assert!(matches!(err, IssueSyncError::InvalidUrl(_)));
    }

    #[test]
    fn parse_github_url_missing_issues_segment() {
        // Has github.com but not /issues/ -- falls through to InvalidUrl
        let err = IssueRef::parse("https://github.com/owner/repo/pull/1").unwrap_err();
        assert!(matches!(err, IssueSyncError::InvalidUrl(_)));
    }

    #[test]
    fn parse_url_with_only_hash_no_slash() {
        // Has # but no / => doesn't enter short_format branch
        let err = IssueRef::parse("repo#123").unwrap_err();
        assert!(matches!(err, IssueSyncError::InvalidUrl(_)));
    }

    #[test]
    fn issue_ref_new_and_fields() {
        let r = IssueRef::new("a", "b", 7);
        assert_eq!(r.owner, "a");
        assert_eq!(r.repo, "b");
        assert_eq!(r.number, 7);
    }

    #[test]
    fn sync_result_into_ticket() {
        let ticket = Ticket {
            id: TicketId::new(),
            source: TicketSource::Manual,
            title: "T".to_string(),
            description: "D".to_string(),
            labels: vec![],
            slices: vec![],
            status: TicketStatus::New,
            created_at: Utc::now(),
        };

        let t_clone = ticket.clone();
        let created = SyncResult::Created(ticket);
        let t = created.into_ticket();
        assert_eq!(t.title, "T");

        let updated = SyncResult::Updated(t_clone);
        let t2 = updated.into_ticket();
        assert_eq!(t2.title, "T");
    }

    #[test]
    fn sync_result_ticket_ref_updated() {
        let ticket = Ticket {
            id: TicketId::new(),
            source: TicketSource::Manual,
            title: "Updated".to_string(),
            description: "".to_string(),
            labels: vec![],
            slices: vec![],
            status: TicketStatus::New,
            created_at: Utc::now(),
        };
        let updated = SyncResult::Updated(ticket);
        assert_eq!(updated.ticket().title, "Updated");
    }

    #[test]
    fn convert_issue_multiple_labels() {
        let issue = GitHubIssue {
            number: 5,
            title: "Multi".to_string(),
            body: Some("body".to_string()),
            state: "open".to_string(),
            labels: vec![
                GitHubLabel {
                    name: "a".to_string(),
                    color: "000".to_string(),
                    description: None,
                },
                GitHubLabel {
                    name: "b".to_string(),
                    color: "fff".to_string(),
                    description: None,
                },
            ],
            user: GitHubApiUser {
                login: "u".to_string(),
                id: 1,
                avatar_url: None,
            },
            assignees: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            html_url: "https://github.com/o/r/issues/5".to_string(),
            pull_request: None,
        };
        let ticket = convert_issue_to_ticket(issue, "o", "r");
        assert_eq!(ticket.labels, vec!["a", "b"]);
    }

    #[test]
    fn convert_issue_preserves_source_owner_repo() {
        let issue = GitHubIssue {
            number: 1,
            title: "T".to_string(),
            body: None,
            state: "open".to_string(),
            labels: vec![],
            user: GitHubApiUser {
                login: "u".to_string(),
                id: 1,
                avatar_url: None,
            },
            assignees: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            html_url: "".to_string(),
            pull_request: None,
        };
        let ticket = convert_issue_to_ticket(issue, "myowner", "myrepo");
        match &ticket.source {
            TicketSource::GitHub {
                owner,
                repo,
                issue_number,
            } => {
                assert_eq!(owner, "myowner");
                assert_eq!(repo, "myrepo");
                assert_eq!(*issue_number, 1);
            }
            _ => panic!("Expected GitHub source"),
        }
    }

    #[test]
    fn error_display_messages() {
        let e1 = IssueSyncError::InvalidUrl("bad".to_string());
        assert!(e1.to_string().contains("bad"));

        let e2 = IssueSyncError::InvalidNumber("nan".to_string());
        assert!(e2.to_string().contains("nan"));
    }
}
