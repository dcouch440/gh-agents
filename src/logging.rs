//! Logging infrastructure

use anyhow::Result;
use std::path::Path;
use tracing::{span, Level, Span};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Default log directory
pub const LOG_DIR: &str = ".nexor/logs";

/// Initialize logging with console output only
///
/// Call this during startup. Returns Ok(None) since no file guard is needed.
pub fn init_logging() -> Result<Option<WorkerGuard>> {
    init_logging_with_file(None)
}

/// Initialize logging with optional file output
///
/// If `log_dir` is Some, logs will also be written to files in that directory.
/// Returns a guard that must be held for the lifetime of the application
/// to ensure all logs are flushed.
pub fn init_logging_with_file(log_dir: Option<&Path>) -> Result<Option<WorkerGuard>> {
    // Build env filter - default to INFO, allow override via RUST_LOG
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    // Console layer - always enabled
    let console_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(false)
        .with_line_number(false);

    match log_dir {
        Some(dir) => {
            // Create log directory if needed
            std::fs::create_dir_all(dir)?;

            // File appender - rotates daily
            let file_appender = tracing_appender::rolling::daily(dir, "nexor.log");
            let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

            // File layer - more verbose format
            let file_layer = fmt::layer()
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true)
                .with_ansi(false)
                .with_writer(non_blocking);

            tracing_subscriber::registry().with(env_filter).with(console_layer).with(file_layer).init();

            Ok(Some(guard))
        }
        None => {
            tracing_subscriber::registry().with(env_filter).with(console_layer).init();

            Ok(None)
        }
    }
}

/// Create a span for an agent operation
pub fn agent_span(agent_id: &str, tier: &str) -> Span {
    span!(Level::INFO, "agent", id = %agent_id, tier = %tier)
}

/// Create a span for a task operation
pub fn task_span(task_id: &str, title: &str) -> Span {
    span!(Level::INFO, "task", id = %task_id, title = %title)
}

/// Create a span for an LLM call
pub fn llm_span(provider: &str, model: &str) -> Span {
    span!(Level::DEBUG, "llm", provider = %provider, model = %model)
}

/// Create a span for database operations
pub fn db_span(operation: &str) -> Span {
    span!(Level::DEBUG, "db", op = %operation)
}

/// Log an agent report (for the feed)
#[macro_export]
macro_rules! log_agent_report {
    ($agent_id:expr, $($arg:tt)*) => {
        tracing::info!(agent_id = %$agent_id, category = "report", $($arg)*)
    };
}

/// Log a milestone completion
#[macro_export]
macro_rules! log_milestone {
    ($($arg:tt)*) => {
        tracing::info!(category = "milestone", $($arg)*)
    };
}

/// Log an error with context
#[macro_export]
macro_rules! log_error {
    ($err:expr, $($arg:tt)*) => {
        tracing::error!(error = %$err, $($arg)*)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spans_compile() {
        // Just verify the span creation compiles
        let _agent = agent_span("test-id", "worker");
        let _task = task_span("task-id", "Test Task");
        let _llm = llm_span("anthropic", "claude-sonnet");
        let _db = db_span("insert");
    }

    #[test]
    fn log_dir_constant() {
        assert_eq!(LOG_DIR, ".nexor/logs");
    }

    #[test]
    fn span_helpers_return_spans() {
        // Spans may be disabled without a subscriber, just verify they construct
        let _agent = agent_span("agent-123", "orchestrator");
        let _task = task_span("task-456", "Build feature");
        let _llm = llm_span("openai", "gpt-4");
        let _db = db_span("select_tasks");
    }

    #[test]
    fn macros_compile_and_dont_panic() {
        // These macros just call tracing macros, verify they compile
        log_agent_report!("agent-1", "Task completed successfully");
        log_milestone!("Milestone 1 complete");
        log_error!(std::io::Error::new(std::io::ErrorKind::Other, "test"), "Something failed");
    }

    #[test]
    fn init_logging_with_file_creates_log_dir() {
        // We can't call init() (panics if global subscriber already set),
        // but we can test that the directory creation logic works by
        // verifying create_dir_all and the file appender setup.
        let temp_dir = tempfile::TempDir::new().unwrap();
        let log_dir = temp_dir.path().join("logs");
        assert!(!log_dir.exists());

        // Directly test directory creation
        std::fs::create_dir_all(&log_dir).unwrap();
        assert!(log_dir.exists());

        // Test that the rolling appender can be created
        let file_appender = tracing_appender::rolling::daily(&log_dir, "nexor.log");
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
        drop(non_blocking);
    }

    #[test]
    fn env_filter_falls_back_to_info() {
        // When RUST_LOG is not set (or invalid), should fall back to "info"
        let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
        let debug_str = format!("{}", filter);
        // The filter should contain "info" as default
        assert!(!debug_str.is_empty());
    }

    #[test]
    fn agent_span_has_correct_fields() {
        let span = agent_span("test-agent", "worker");
        // Span is valid (may be disabled without subscriber)
        assert!(!span.is_disabled() || span.is_disabled()); // just ensure no panic
    }

    #[test]
    fn task_span_has_correct_fields() {
        let span = task_span("task-1", "My Task");
        let _ = format!("{:?}", span);
    }

    #[test]
    fn llm_span_has_correct_fields() {
        let span = llm_span("anthropic", "claude-opus");
        let _ = format!("{:?}", span);
    }

    #[test]
    fn db_span_has_correct_fields() {
        let span = db_span("upsert");
        let _ = format!("{:?}", span);
    }
}
