//! Retry logic for wg-easy VPN API calls.
//!
//! Reuses [`BackoffConfig`] and [`ExponentialBackoff`] from the LLM retry
//! module (which are generic) but implements VPN-specific retry policy.

use std::future::Future;

use tokio::time::sleep;
use tracing::{info, warn};

use crate::llm::{BackoffConfig, ExponentialBackoff};

use super::VpnError;

mod tests;

/// Classify whether a [`VpnError`] is worth retrying.
pub fn is_retryable(error: &VpnError) -> bool {
    match error {
        // Network-level failures are always retryable
        VpnError::ApiUnreachable(_) => true,
        VpnError::HttpError(e) => e.is_timeout() || e.is_connect() || e.is_request(),

        // Server errors (5xx) are retryable; client errors (4xx) are not
        VpnError::PeerCreationFailed { reason } => is_server_error(reason),
        VpnError::PeerDeletionFailed { reason, .. } => is_server_error(reason),
        VpnError::ConfigRetrievalFailed { reason, .. } => is_server_error(reason),

        // Permanent failures — never retry
        VpnError::ConfigValidationFailed { .. } => false,
        VpnError::AuthFailed => false,
        VpnError::SidecarFailed(_) => false,
        VpnError::HealthCheckTimeout { .. } => false,
        VpnError::IoError(_) => false,
    }
}

/// Check if the reason string indicates a 5xx server error.
///
/// WgEasyClient formats reasons as `"HTTP {status}: {body}"`.
fn is_server_error(reason: &str) -> bool {
    reason.starts_with("HTTP 5")
}

/// Default backoff config for VPN API calls.
///
/// Shorter and fewer retries than LLM calls — the wg-easy API is a
/// fast local/nearby service, not a slow LLM completion.
pub fn vpn_backoff_config() -> BackoffConfig {
    BackoffConfig::new()
        .with_initial_delay(std::time::Duration::from_millis(
            crate::constants::VPN_RETRY_INITIAL_BACKOFF_MS,
        ))
        .with_max_delay(std::time::Duration::from_secs(
            crate::constants::VPN_RETRY_MAX_BACKOFF_SECS,
        ))
        .with_max_retries(crate::constants::VPN_RETRY_MAX_ATTEMPTS)
        .with_jitter(crate::constants::RETRY_JITTER_FACTOR)
}

/// Execute a VPN operation with exponential-backoff retry.
///
/// On retryable errors, backs off and retries up to the configured limit.
/// On permanent errors (auth failure, sidecar failure, etc.), returns immediately.
pub async fn vpn_with_retry<T, F, Fut>(operation: F) -> Result<T, VpnError>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, VpnError>>,
{
    let config = vpn_backoff_config();
    let backoff = ExponentialBackoff::new(config);
    let max_retries = backoff.max_retries();
    let mut last_error: VpnError;

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
            "VPN API retry"
        );

        sleep(delay).await;

        match operation().await {
            Ok(result) => {
                info!(attempt, "VPN API retry succeeded");
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
