//! Retry logic for transient Docker container creation failures.
//!
//! Reuses [`BackoffConfig`] and [`ExponentialBackoff`] from the LLM retry
//! module but implements container-specific retry classification.

use std::future::Future;

use tokio::time::sleep;
use tracing::{info, warn};

use crate::llm::{BackoffConfig, ExponentialBackoff};

use super::ContainerError;

mod tests;

/// Classify whether a [`ContainerError`] is worth retrying.
///
/// Retryable errors are transient Docker daemon or network issues.
/// Permanent errors (bad config, clone auth, path violations) are never retried.
pub fn is_retryable(error: &ContainerError) -> bool {
    match error {
        // Docker daemon unreachable or failed to spawn — always retryable
        ContainerError::DockerNotAvailable(_) => true,
        ContainerError::DockerSpawnFailed { .. } => true,

        // Creation timeout — the daemon may have been temporarily overloaded
        ContainerError::CreateTimeout { .. } => true,

        // Generic creation failure — only if it smells like a daemon issue
        ContainerError::CreationFailed(msg) => is_transient_docker_error(msg),

        // Permanent failures — never retry
        ContainerError::CloneFailed { .. } => false,
        ContainerError::CommandFailed { .. } => false,
        ContainerError::Timeout { .. } => false,
        ContainerError::NotRunning { .. } => false,
        ContainerError::PathNotAllowed { .. } => false,
        ContainerError::NetworkDisconnectFailed { .. } => false,
        ContainerError::IoError(_) => false,
    }
}

/// Heuristic check for transient Docker daemon errors in a creation failure message.
fn is_transient_docker_error(msg: &str) -> bool {
    let lower = msg.to_lowercase();
    lower.contains("daemon")
        || lower.contains("503")
        || lower.contains("connection refused")
        || lower.contains("timeout")
        || lower.contains("temporarily unavailable")
}

/// Default backoff config for container creation retries.
pub fn container_backoff_config() -> BackoffConfig {
    BackoffConfig::new()
        .with_initial_delay(std::time::Duration::from_millis(
            crate::constants::CONTAINER_RETRY_INITIAL_BACKOFF_MS,
        ))
        .with_max_delay(std::time::Duration::from_secs(
            crate::constants::CONTAINER_RETRY_MAX_BACKOFF_SECS,
        ))
        .with_max_retries(crate::constants::CONTAINER_RETRY_MAX_ATTEMPTS)
        .with_jitter(crate::constants::RETRY_JITTER_FACTOR)
}

/// Execute a container operation with exponential-backoff retry.
///
/// On retryable errors, backs off and retries up to the configured limit.
/// On permanent errors (clone failure, path violation, etc.), returns immediately.
pub async fn container_with_retry<T, F, Fut>(operation: F) -> Result<T, ContainerError>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, ContainerError>>,
{
    let config = container_backoff_config();
    let backoff = ExponentialBackoff::new(config);
    let max_retries = backoff.max_retries();
    let mut last_error: ContainerError;

    // First attempt (not a retry)
    match operation().await {
        Ok(result) => return Ok(result),
        Err(e) => {
            if !is_retryable(&e) {
                return Err(e);
            }
            last_error = e;
        }
    }

    // Retry loop
    let mut attempt = 0u32;
    for delay in backoff {
        attempt += 1;

        warn!(
            attempt,
            max_retries,
            error = %last_error,
            delay_ms = %delay.as_millis(),
            "Container creation retry"
        );

        sleep(delay).await;

        match operation().await {
            Ok(result) => {
                info!(attempt, "Container creation retry succeeded");
                return Ok(result);
            }
            Err(e) => {
                if !is_retryable(&e) {
                    return Err(e);
                }
                last_error = e;
            }
        }
    }

    Err(last_error)
}
