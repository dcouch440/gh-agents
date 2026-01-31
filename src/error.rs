//! Centralized error handling for nexor
//!
//! Provides user-friendly error types with recovery suggestions.

use chrono::{DateTime, Utc};
use std::future::Future;
use thiserror::Error;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::error;

/// Main error type for nexor with user-friendly messages and suggestions
#[derive(Error, Debug, Clone)]
pub enum NexorError {
    #[error("configuration error: {message}")]
    Config {
        message: String,
        suggestion: Option<String>,
    },

    #[error("database error: {message}")]
    Database {
        message: String,
        suggestion: Option<String>,
    },

    #[error("LLM API error: {message}")]
    LlmApi {
        message: String,
        suggestion: Option<String>,
    },

    #[error("GitHub API error: {message}")]
    GitHubApi {
        message: String,
        suggestion: Option<String>,
    },

    #[error("agent error: {agent_id} - {message}")]
    Agent {
        agent_id: String,
        message: String,
        suggestion: Option<String>,
    },

    #[error("task failed: {task_id} - {message}")]
    TaskFailed {
        task_id: String,
        message: String,
        recoverable: bool,
    },

    #[error("internal error: {message}")]
    Internal { message: String },
}

impl NexorError {
    /// Get the suggestion for this error, if any
    pub fn suggestion(&self) -> Option<&str> {
        match self {
            NexorError::Config { suggestion, .. } => suggestion.as_deref(),
            NexorError::Database { suggestion, .. } => suggestion.as_deref(),
            NexorError::LlmApi { suggestion, .. } => suggestion.as_deref(),
            NexorError::GitHubApi { suggestion, .. } => suggestion.as_deref(),
            NexorError::Agent { suggestion, .. } => suggestion.as_deref(),
            _ => None,
        }
    }

    /// Check if this error is recoverable (can be retried)
    pub fn is_recoverable(&self) -> bool {
        match self {
            NexorError::TaskFailed { recoverable, .. } => *recoverable,
            NexorError::LlmApi { .. } => true, // Usually transient (rate limits, network)
            NexorError::GitHubApi { message, .. } => {
                // 404/403 are permanent; rate limits and server errors are transient
                let msg = message.to_lowercase();
                !msg.contains("not found")
                    && !msg.contains("404")
                    && !msg.contains("forbidden")
                    && !msg.contains("403")
                    && !msg.contains("authentication")
            }
            NexorError::Database { message, .. } => {
                // Connection/timeout issues are transient; constraint violations are not
                let msg = message.to_lowercase();
                msg.contains("timeout")
                    || msg.contains("connection")
                    || msg.contains("locked")
                    || msg.contains("pool")
            }
            _ => false,
        }
    }

    // Constructors with appropriate suggestions

    /// Create error for missing config key
    pub fn config_missing_key(key: &str) -> Self {
        NexorError::Config {
            message: format!("missing required key: {}", key),
            suggestion: Some(format!("Add '{}' to your config file", key)),
        }
    }

    /// Create error for invalid config value
    pub fn config_invalid_value(key: &str, value: &str, expected: &str) -> Self {
        NexorError::Config {
            message: format!("invalid value for '{}': {}", key, value),
            suggestion: Some(format!("Expected: {}", expected)),
        }
    }

    /// Create error for missing API key
    pub fn api_key_missing(provider: &str) -> Self {
        let env_var = match provider {
            "anthropic" => "ANTHROPIC_API_KEY",
            "github" => "GITHUB_TOKEN",
            _ => "API_KEY",
        };
        NexorError::Config {
            message: format!("{} API key not found", provider),
            suggestion: Some(format!(
                "Set the {} environment variable: export {}=\"your-key\"",
                env_var, env_var
            )),
        }
    }

    /// Create error for rate limiting
    pub fn rate_limited(reset_time: DateTime<Utc>) -> Self {
        NexorError::LlmApi {
            message: "rate limit exceeded".to_string(),
            suggestion: Some(format!(
                "Wait until {} or reduce agent concurrency in config",
                reset_time.format("%H:%M:%S UTC")
            )),
        }
    }

    /// Create error for rate limiting without specific reset time
    pub fn rate_limited_simple() -> Self {
        NexorError::LlmApi {
            message: "rate limit exceeded".to_string(),
            suggestion: Some("Rate limited. Wait a moment and try again.".to_string()),
        }
    }

    /// Create error for network issues
    pub fn network_error(details: &str) -> Self {
        NexorError::LlmApi {
            message: format!("network error: {}", details),
            suggestion: Some("Check your internet connection and try again".to_string()),
        }
    }

    /// Create error for database lock
    pub fn database_locked() -> Self {
        NexorError::Database {
            message: "database is locked".to_string(),
            suggestion: Some(
                "Another nexor instance may be running. Close it or delete .nexor/state.db-lock"
                    .to_string(),
            ),
        }
    }

    /// Create error for database query failure
    pub fn database_query(details: &str) -> Self {
        NexorError::Database {
            message: format!("query failed: {}", details),
            suggestion: None,
        }
    }

    /// Create error for GitHub resource not found
    pub fn github_not_found(resource: &str) -> Self {
        NexorError::GitHubApi {
            message: format!("{} not found", resource),
            suggestion: Some(
                "Check the URL and ensure you have access to this repository".to_string(),
            ),
        }
    }

    /// Create error for GitHub authentication failure
    pub fn github_unauthorized() -> Self {
        NexorError::GitHubApi {
            message: "authentication failed".to_string(),
            suggestion: Some(
                "Check your GITHUB_TOKEN has the required permissions (repo scope)".to_string(),
            ),
        }
    }

    /// Create error for GitHub rate limiting
    pub fn github_rate_limited(reset_time: Option<DateTime<Utc>>) -> Self {
        let suggestion = match reset_time {
            Some(time) => format!(
                "Rate limited. Try again after {}",
                time.format("%H:%M:%S UTC")
            ),
            None => "Rate limited. Wait a moment and try again.".to_string(),
        };
        NexorError::GitHubApi {
            message: "GitHub rate limit exceeded".to_string(),
            suggestion: Some(suggestion),
        }
    }

    /// Create error for agent timeout
    pub fn agent_timeout(agent_id: &str) -> Self {
        NexorError::Agent {
            agent_id: agent_id.to_string(),
            message: "agent timed out".to_string(),
            suggestion: Some(
                "The task may be too complex. Try breaking it into smaller pieces.".to_string(),
            ),
        }
    }

    /// Create error for agent failure
    pub fn agent_failed(agent_id: &str, reason: &str) -> Self {
        NexorError::Agent {
            agent_id: agent_id.to_string(),
            message: reason.to_string(),
            suggestion: None,
        }
    }

    /// Create error for task failure
    pub fn task_failed(task_id: &str, reason: &str, recoverable: bool) -> Self {
        NexorError::TaskFailed {
            task_id: task_id.to_string(),
            message: reason.to_string(),
            recoverable,
        }
    }

    /// Create internal error
    pub fn internal(message: impl Into<String>) -> Self {
        NexorError::Internal {
            message: message.into(),
        }
    }
}

/// Spawn a task with error boundary that reports failures to a channel
pub fn spawn_with_boundary<F, T>(
    name: &str,
    error_tx: mpsc::Sender<NexorError>,
    future: F,
) -> JoinHandle<()>
where
    F: Future<Output = Result<T, anyhow::Error>> + Send + 'static,
    T: Send + 'static,
{
    let name = name.to_string();
    tokio::spawn(async move {
        match future.await {
            Ok(_) => {}
            Err(e) => {
                error!(task = %name, error = %e, "task failed");
                let nexor_error = enrich_error(e);
                let _ = error_tx.send(nexor_error).await;
            }
        }
    })
}

/// Spawn a task with error boundary that returns the error
pub fn spawn_with_result<F, T>(name: &str, future: F) -> JoinHandle<Result<T, NexorError>>
where
    F: Future<Output = Result<T, anyhow::Error>> + Send + 'static,
    T: Send + 'static,
{
    let name = name.to_string();
    tokio::spawn(async move {
        match future.await {
            Ok(result) => Ok(result),
            Err(e) => {
                error!(task = %name, error = %e, "task failed");
                Err(enrich_error(e))
            }
        }
    })
}

/// Convert common anyhow errors to NexorError with appropriate suggestions
pub fn enrich_error(error: anyhow::Error) -> NexorError {
    let msg = error.to_string().to_lowercase();

    // Check for API key / auth issues
    if msg.contains("api key") || msg.contains("unauthorized") || msg.contains("401") {
        return NexorError::api_key_missing("unknown");
    }

    // Check for rate limiting
    if msg.contains("rate limit") || msg.contains("429") {
        return NexorError::rate_limited_simple();
    }

    // Check for network issues
    if msg.contains("network")
        || msg.contains("connection")
        || msg.contains("timeout")
        || msg.contains("dns")
    {
        return NexorError::network_error(&error.to_string());
    }

    // Check for database issues
    if msg.contains("database") || msg.contains("sqlite") || msg.contains("sqlx") {
        if msg.contains("locked") {
            return NexorError::database_locked();
        }
        return NexorError::Database {
            message: error.to_string(),
            suggestion: None,
        };
    }

    // Check for GitHub issues
    if msg.contains("github") {
        if msg.contains("not found") || msg.contains("404") {
            return NexorError::github_not_found("resource");
        }
        if msg.contains("forbidden") || msg.contains("403") {
            return NexorError::github_unauthorized();
        }
        return NexorError::GitHubApi {
            message: error.to_string(),
            suggestion: None,
        };
    }

    // Default: internal error
    NexorError::Internal {
        message: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nexor_error_display() {
        let err = NexorError::Config {
            message: "test".to_string(),
            suggestion: Some("fix it".to_string()),
        };
        assert_eq!(format!("{}", err), "configuration error: test");
    }

    #[test]
    fn nexor_error_suggestion() {
        let err = NexorError::Config {
            message: "test".to_string(),
            suggestion: Some("fix it".to_string()),
        };
        assert_eq!(err.suggestion(), Some("fix it"));

        let err = NexorError::Internal {
            message: "test".to_string(),
        };
        assert_eq!(err.suggestion(), None);
    }

    #[test]
    fn nexor_error_is_recoverable() {
        let err = NexorError::LlmApi {
            message: "rate limited".to_string(),
            suggestion: None,
        };
        assert!(err.is_recoverable());

        let err = NexorError::Config {
            message: "missing key".to_string(),
            suggestion: None,
        };
        assert!(!err.is_recoverable());

        let err = NexorError::TaskFailed {
            task_id: "1".to_string(),
            message: "failed".to_string(),
            recoverable: true,
        };
        assert!(err.is_recoverable());

        let err = NexorError::TaskFailed {
            task_id: "1".to_string(),
            message: "failed".to_string(),
            recoverable: false,
        };
        assert!(!err.is_recoverable());
    }

    #[test]
    fn api_key_missing_anthropic() {
        let err = NexorError::api_key_missing("anthropic");
        assert!(err.to_string().contains("anthropic"));
        assert!(err.suggestion().unwrap().contains("ANTHROPIC_API_KEY"));
    }

    #[test]
    fn api_key_missing_github() {
        let err = NexorError::api_key_missing("github");
        assert!(err.suggestion().unwrap().contains("GITHUB_TOKEN"));
    }

    #[test]
    fn rate_limited_with_time() {
        let time = Utc::now();
        let err = NexorError::rate_limited(time);
        assert!(err.to_string().contains("rate limit"));
        assert!(err.suggestion().is_some());
    }

    #[test]
    fn network_error_has_suggestion() {
        let err = NexorError::network_error("connection refused");
        assert!(err.suggestion().unwrap().contains("internet connection"));
    }

    #[test]
    fn database_locked_has_suggestion() {
        let err = NexorError::database_locked();
        assert!(err.suggestion().unwrap().contains("Another nexor instance"));
    }

    #[test]
    fn github_not_found_has_suggestion() {
        let err = NexorError::github_not_found("issue #123");
        assert!(err.to_string().contains("issue #123"));
        assert!(err.suggestion().unwrap().contains("access"));
    }

    #[test]
    fn github_unauthorized_has_suggestion() {
        let err = NexorError::github_unauthorized();
        assert!(err.suggestion().unwrap().contains("permissions"));
    }

    #[test]
    fn agent_timeout_has_suggestion() {
        let err = NexorError::agent_timeout("agent-1");
        assert!(err.to_string().contains("agent-1"));
        assert!(err.suggestion().unwrap().contains("smaller pieces"));
    }

    #[test]
    fn enrich_error_rate_limit() {
        let err = anyhow::anyhow!("rate limit exceeded (429)");
        let nexor_err = enrich_error(err);
        assert!(matches!(nexor_err, NexorError::LlmApi { .. }));
    }

    #[test]
    fn enrich_error_network() {
        let err = anyhow::anyhow!("connection timeout");
        let nexor_err = enrich_error(err);
        assert!(matches!(nexor_err, NexorError::LlmApi { .. }));
        assert!(nexor_err.suggestion().is_some());
    }

    #[test]
    fn enrich_error_database_locked() {
        let err = anyhow::anyhow!("database is locked");
        let nexor_err = enrich_error(err);
        assert!(matches!(nexor_err, NexorError::Database { .. }));
        assert!(nexor_err.suggestion().unwrap().contains("Another"));
    }

    #[test]
    fn enrich_error_github_not_found() {
        let err = anyhow::anyhow!("github 404 not found");
        let nexor_err = enrich_error(err);
        assert!(matches!(nexor_err, NexorError::GitHubApi { .. }));
    }

    #[test]
    fn enrich_error_unknown() {
        let err = anyhow::anyhow!("something went wrong");
        let nexor_err = enrich_error(err);
        assert!(matches!(nexor_err, NexorError::Internal { .. }));
    }

    #[tokio::test]
    async fn spawn_with_boundary_success() {
        let (tx, mut rx) = mpsc::channel(1);
        let handle = spawn_with_boundary("test", tx, async { Ok::<_, anyhow::Error>("success") });
        handle.await.unwrap();
        // No error should be sent
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn spawn_with_boundary_failure() {
        let (tx, mut rx) = mpsc::channel(1);
        let handle = spawn_with_boundary("test", tx, async {
            Err::<(), _>(anyhow::anyhow!("rate limit exceeded"))
        });
        handle.await.unwrap();
        // Error should be sent
        let err = rx.recv().await.unwrap();
        assert!(matches!(err, NexorError::LlmApi { .. }));
    }

    #[tokio::test]
    async fn spawn_with_result_success() {
        let handle = spawn_with_result("test", async { Ok::<_, anyhow::Error>(42) });
        let result = handle.await.unwrap();
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn spawn_with_result_failure() {
        let handle = spawn_with_result("test", async {
            Err::<i32, _>(anyhow::anyhow!("database locked"))
        });
        let result = handle.await.unwrap();
        assert!(matches!(result, Err(NexorError::Database { .. })));
    }

    // --- Additional coverage tests ---

    #[test]
    fn display_all_variants() {
        let db = NexorError::Database {
            message: "fail".into(),
            suggestion: None,
        };
        assert_eq!(format!("{}", db), "database error: fail");

        let llm = NexorError::LlmApi {
            message: "bad".into(),
            suggestion: None,
        };
        assert_eq!(format!("{}", llm), "LLM API error: bad");

        let gh = NexorError::GitHubApi {
            message: "nope".into(),
            suggestion: None,
        };
        assert_eq!(format!("{}", gh), "GitHub API error: nope");

        let agent = NexorError::Agent {
            agent_id: "a1".into(),
            message: "boom".into(),
            suggestion: None,
        };
        assert_eq!(format!("{}", agent), "agent error: a1 - boom");

        let task = NexorError::TaskFailed {
            task_id: "t1".into(),
            message: "oops".into(),
            recoverable: false,
        };
        assert_eq!(format!("{}", task), "task failed: t1 - oops");

        let internal = NexorError::Internal {
            message: "bug".into(),
        };
        assert_eq!(format!("{}", internal), "internal error: bug");
    }

    #[test]
    fn suggestion_all_variants() {
        let db = NexorError::Database {
            message: "x".into(),
            suggestion: Some("retry".into()),
        };
        assert_eq!(db.suggestion(), Some("retry"));

        let llm = NexorError::LlmApi {
            message: "x".into(),
            suggestion: Some("wait".into()),
        };
        assert_eq!(llm.suggestion(), Some("wait"));

        let gh = NexorError::GitHubApi {
            message: "x".into(),
            suggestion: Some("check token".into()),
        };
        assert_eq!(gh.suggestion(), Some("check token"));

        let agent = NexorError::Agent {
            agent_id: "a".into(),
            message: "x".into(),
            suggestion: Some("split task".into()),
        };
        assert_eq!(agent.suggestion(), Some("split task"));

        // TaskFailed returns None
        let task = NexorError::TaskFailed {
            task_id: "t".into(),
            message: "x".into(),
            recoverable: true,
        };
        assert_eq!(task.suggestion(), None);
    }

    #[test]
    fn is_recoverable_github_api_transient() {
        let err = NexorError::GitHubApi {
            message: "rate limit exceeded".into(),
            suggestion: None,
        };
        assert!(err.is_recoverable());

        let err = NexorError::GitHubApi {
            message: "server error 500".into(),
            suggestion: None,
        };
        assert!(err.is_recoverable());
    }

    #[test]
    fn is_not_recoverable_github_api_permanent() {
        let err = NexorError::GitHubApi {
            message: "not found 404".into(),
            suggestion: None,
        };
        assert!(!err.is_recoverable());

        let err = NexorError::GitHubApi {
            message: "forbidden 403".into(),
            suggestion: None,
        };
        assert!(!err.is_recoverable());

        let err = NexorError::GitHubApi {
            message: "authentication failed".into(),
            suggestion: None,
        };
        assert!(!err.is_recoverable());
    }

    #[test]
    fn is_recoverable_database_transient() {
        let err = NexorError::Database {
            message: "connection timeout".into(),
            suggestion: None,
        };
        assert!(err.is_recoverable());

        let err = NexorError::Database {
            message: "database is locked".into(),
            suggestion: None,
        };
        assert!(err.is_recoverable());

        let err = NexorError::Database {
            message: "pool timed out".into(),
            suggestion: None,
        };
        assert!(err.is_recoverable());
    }

    #[test]
    fn is_not_recoverable_database_permanent() {
        let err = NexorError::Database {
            message: "constraint violation".into(),
            suggestion: None,
        };
        assert!(!err.is_recoverable());
    }

    #[test]
    fn is_recoverable_agent_not_recoverable() {
        let err = NexorError::Agent {
            agent_id: "a".into(),
            message: "crash".into(),
            suggestion: None,
        };
        assert!(!err.is_recoverable());
    }

    #[test]
    fn is_recoverable_internal_not_recoverable() {
        let err = NexorError::Internal {
            message: "bug".into(),
        };
        assert!(!err.is_recoverable());
    }

    #[test]
    fn api_key_missing_unknown_provider() {
        let err = NexorError::api_key_missing("openai");
        assert!(err.suggestion().unwrap().contains("API_KEY"));
    }

    #[test]
    fn rate_limited_simple_has_suggestion() {
        let err = NexorError::rate_limited_simple();
        assert!(err.to_string().contains("rate limit"));
        assert!(err.suggestion().unwrap().contains("Wait"));
    }

    #[test]
    fn database_query_no_suggestion() {
        let err = NexorError::database_query("syntax error");
        assert!(err.to_string().contains("syntax error"));
        assert_eq!(err.suggestion(), None);
    }

    #[test]
    fn github_rate_limited_with_time() {
        let time = Utc::now();
        let err = NexorError::github_rate_limited(Some(time));
        assert!(err.suggestion().unwrap().contains("Try again after"));
    }

    #[test]
    fn github_rate_limited_without_time() {
        let err = NexorError::github_rate_limited(None);
        assert!(err.suggestion().unwrap().contains("Wait a moment"));
    }

    #[test]
    fn agent_failed_no_suggestion() {
        let err = NexorError::agent_failed("a1", "out of memory");
        assert!(err.to_string().contains("a1"));
        assert!(err.to_string().contains("out of memory"));
        assert_eq!(err.suggestion(), None);
    }

    #[test]
    fn task_failed_constructor() {
        let err = NexorError::task_failed("t1", "compile error", false);
        assert!(err.to_string().contains("t1"));
        assert!(!err.is_recoverable());

        let err = NexorError::task_failed("t2", "flaky test", true);
        assert!(err.is_recoverable());
    }

    #[test]
    fn internal_constructor() {
        let err = NexorError::internal("unexpected state");
        assert_eq!(format!("{}", err), "internal error: unexpected state");
    }

    #[test]
    fn config_missing_key_constructor() {
        let err = NexorError::config_missing_key("db_url");
        assert!(err.to_string().contains("db_url"));
        assert!(err.suggestion().unwrap().contains("db_url"));
    }

    #[test]
    fn config_invalid_value_constructor() {
        let err = NexorError::config_invalid_value("port", "abc", "a number");
        assert!(err.to_string().contains("abc"));
        assert!(err.suggestion().unwrap().contains("a number"));
    }

    #[test]
    fn enrich_error_api_key() {
        let err = anyhow::anyhow!("api key is invalid");
        let nexor_err = enrich_error(err);
        assert!(matches!(nexor_err, NexorError::Config { .. }));
    }

    #[test]
    fn enrich_error_unauthorized() {
        let err = anyhow::anyhow!("401 unauthorized");
        let nexor_err = enrich_error(err);
        assert!(matches!(nexor_err, NexorError::Config { .. }));
    }

    #[test]
    fn enrich_error_dns() {
        let err = anyhow::anyhow!("dns resolution failed");
        let nexor_err = enrich_error(err);
        assert!(matches!(nexor_err, NexorError::LlmApi { .. }));
        assert!(nexor_err.suggestion().unwrap().contains("internet"));
    }

    #[test]
    fn enrich_error_sqlite() {
        let err = anyhow::anyhow!("sqlite constraint violation");
        let nexor_err = enrich_error(err);
        assert!(matches!(nexor_err, NexorError::Database { .. }));
    }

    #[test]
    fn enrich_error_sqlx() {
        let err = anyhow::anyhow!("sqlx pool timed out");
        let nexor_err = enrich_error(err);
        assert!(matches!(nexor_err, NexorError::Database { .. }));
    }

    #[test]
    fn enrich_error_github_forbidden() {
        let err = anyhow::anyhow!("github 403 forbidden");
        let nexor_err = enrich_error(err);
        assert!(matches!(nexor_err, NexorError::GitHubApi { .. }));
        assert!(nexor_err.suggestion().unwrap().contains("permissions"));
    }

    #[test]
    fn enrich_error_github_generic() {
        let err = anyhow::anyhow!("github server error 500");
        let nexor_err = enrich_error(err);
        assert!(matches!(nexor_err, NexorError::GitHubApi { .. }));
        assert_eq!(nexor_err.suggestion(), None);
    }

    #[test]
    fn enrich_error_429_only() {
        let err = anyhow::anyhow!("got 429 too many requests");
        let nexor_err = enrich_error(err);
        assert!(matches!(nexor_err, NexorError::LlmApi { .. }));
    }

    #[test]
    fn error_is_clone() {
        let err = NexorError::internal("test");
        let cloned = err.clone();
        assert_eq!(format!("{}", err), format!("{}", cloned));
    }

    #[test]
    fn error_is_debug() {
        let err = NexorError::internal("test");
        let debug = format!("{:?}", err);
        assert!(debug.contains("Internal"));
    }
}
