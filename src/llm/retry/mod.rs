//! Retry logic with exponential backoff

use async_trait::async_trait;
use futures::Stream;
use rand::Rng;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

use super::provider::{LLMProvider, LLMResult};
use super::types::{LLMError, LLMRequest, LLMResponse, StreamChunk};

/// Configuration for exponential backoff
#[derive(Debug, Clone)]
pub struct BackoffConfig {
    /// Initial delay before first retry
    pub initial_delay: Duration,

    /// Maximum delay between retries
    pub max_delay: Duration,

    /// Multiplier for each retry (typically 2.0)
    pub multiplier: f64,

    /// Jitter factor (0.0 = no jitter, 1.0 = up to 100% jitter)
    pub jitter: f64,

    /// Maximum number of retries
    pub max_retries: u32,
}

impl Default for BackoffConfig {
    fn default() -> Self {
        Self {
            initial_delay: Duration::from_millis(crate::constants::RETRY_INITIAL_BACKOFF_MS),
            max_delay: Duration::from_secs(crate::constants::RETRY_MAX_BACKOFF_SECS),
            multiplier: 2.0,
            jitter: crate::constants::RETRY_JITTER_FACTOR,
            max_retries: crate::constants::RETRY_MAX_ATTEMPTS,
        }
    }
}

impl BackoffConfig {
    /// Create a new backoff config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set initial delay
    pub fn with_initial_delay(mut self, delay: Duration) -> Self {
        self.initial_delay = delay;
        self
    }

    /// Set maximum delay
    pub fn with_max_delay(mut self, delay: Duration) -> Self {
        self.max_delay = delay;
        self
    }

    /// Set retry multiplier
    pub fn with_multiplier(mut self, multiplier: f64) -> Self {
        self.multiplier = multiplier;
        self
    }

    /// Set jitter factor (0.0 - 1.0)
    pub fn with_jitter(mut self, jitter: f64) -> Self {
        self.jitter = jitter.clamp(0.0, 1.0);
        self
    }

    /// Set maximum retries
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }
}

/// Exponential backoff iterator
pub struct ExponentialBackoff {
    config: BackoffConfig,
    current_attempt: u32,
    current_delay: Duration,
}

impl ExponentialBackoff {
    /// Create a new backoff iterator from config
    pub fn new(config: BackoffConfig) -> Self {
        Self {
            current_delay: config.initial_delay,
            config,
            current_attempt: 0,
        }
    }

    /// Create with default configuration
    pub fn default_config() -> Self {
        Self::new(BackoffConfig::default())
    }

    /// Reset the backoff state
    pub fn reset(&mut self) {
        self.current_attempt = 0;
        self.current_delay = self.config.initial_delay;
    }

    /// Get current attempt number (0-indexed)
    pub fn attempts(&self) -> u32 {
        self.current_attempt
    }

    /// Check if more retries are available
    pub fn has_remaining(&self) -> bool {
        self.current_attempt < self.config.max_retries
    }

    /// Get max retries configured
    pub fn max_retries(&self) -> u32 {
        self.config.max_retries
    }

    /// Calculate delay with jitter
    fn apply_jitter(&self, base: Duration) -> Duration {
        if self.config.jitter <= 0.0 {
            return base;
        }

        let mut rng = rand::thread_rng();
        let jitter_range = base.as_secs_f64() * self.config.jitter;
        let jitter = rng.gen_range(-jitter_range..jitter_range);
        let adjusted = base.as_secs_f64() + jitter;

        Duration::from_secs_f64(adjusted.max(0.0))
    }
}

impl Iterator for ExponentialBackoff {
    type Item = Duration;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_attempt >= self.config.max_retries {
            return None;
        }

        let delay = self.apply_jitter(self.current_delay);

        // Prepare for next iteration
        self.current_attempt += 1;
        let next_delay_ms = (self.current_delay.as_millis() as f64 * self.config.multiplier) as u64;
        self.current_delay = Duration::from_millis(next_delay_ms).min(self.config.max_delay);

        Some(delay)
    }
}

/// Policy for which errors should be retried
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RetryPolicy {
    /// Retry rate limits and server errors
    Default,
    /// Never retry
    Never,
    /// Always retry (use with caution)
    Always,
}

impl RetryPolicy {
    /// Check if an error should be retried
    pub fn should_retry(&self, error: &LLMError) -> bool {
        match self {
            RetryPolicy::Never => false,
            RetryPolicy::Always => true,
            RetryPolicy::Default => Self::is_retryable(error),
        }
    }

    /// Default logic for retryable errors
    fn is_retryable(error: &LLMError) -> bool {
        match error {
            // Rate limits should always be retried
            LLMError::RateLimited { .. } => true,

            // Server errors are often transient
            LLMError::ApiError { status, .. } => *status >= 500 && *status < 600,

            // Timeouts might succeed on retry
            LLMError::Timeout(_) => true,

            // HTTP errors might be transient
            LLMError::HttpError(e) => {
                // Check if it's a connection error or timeout
                e.is_timeout() || e.is_connect() || e.is_request()
            }

            // These should NOT be retried
            LLMError::AuthError(_) => false,
            LLMError::ParseError(_) => false,
            LLMError::StreamError(_) => false,
            LLMError::MaxRetriesExceeded(_) => false,
        }
    }
}

/// Retry context for logging and tracking
#[derive(Debug)]
pub struct RetryContext {
    pub attempt: u32,
    pub max_attempts: u32,
    pub delay: Duration,
    pub error: String,
}

/// Execute an async operation with retry logic
pub async fn with_retry<T, F, Fut>(
    config: BackoffConfig,
    policy: RetryPolicy,
    operation: F,
) -> Result<T, LLMError>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, LLMError>>,
{
    let backoff = ExponentialBackoff::new(config);
    let max_retries = backoff.max_retries();
    let mut last_error: LLMError;

    // First attempt (not a retry)
    match operation().await {
        Ok(result) => return Ok(result),
        Err(e) => {
            if !policy.should_retry(&e) {
                return Err(e);
            }
            last_error = e;
        }
    }

    // Retry loop
    let mut attempt = 0u32;
    for delay in backoff {
        attempt += 1;

        // Check for rate limit with specific delay
        let actual_delay = match &last_error {
            LLMError::RateLimited { retry_after_ms } => {
                Duration::from_millis(*retry_after_ms).max(delay)
            }
            _ => delay,
        };

        tracing::warn!(
            "Retry {}/{}: {} (waiting {:?})",
            attempt,
            max_retries,
            last_error,
            actual_delay
        );

        sleep(actual_delay).await;

        match operation().await {
            Ok(result) => {
                tracing::info!("Retry {} succeeded", attempt);
                return Ok(result);
            }
            Err(e) => {
                if !policy.should_retry(&e) {
                    return Err(e);
                }
                last_error = e;
            }
        }
    }

    // Max retries exceeded
    Err(LLMError::MaxRetriesExceeded(max_retries))
}

/// Convenience wrapper with default config
pub async fn with_default_retry<T, F, Fut>(operation: F) -> Result<T, LLMError>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, LLMError>>,
{
    with_retry(BackoffConfig::default(), RetryPolicy::Default, operation).await
}

/// A wrapper that adds retry logic to any LLM provider
pub struct RetryingProvider<P: LLMProvider> {
    inner: Arc<P>,
    config: BackoffConfig,
    policy: RetryPolicy,
}

impl<P: LLMProvider + 'static> RetryingProvider<P> {
    /// Create a new retrying provider wrapper
    pub fn new(provider: P, config: BackoffConfig) -> Self {
        Self {
            inner: Arc::new(provider),
            config,
            policy: RetryPolicy::Default,
        }
    }

    /// Create with default config
    pub fn with_defaults(provider: P) -> Self {
        Self::new(provider, BackoffConfig::default())
    }

    /// Set the retry policy
    pub fn with_policy(mut self, policy: RetryPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Set the backoff config
    pub fn with_config(mut self, config: BackoffConfig) -> Self {
        self.config = config;
        self
    }

    /// Get a reference to the inner provider
    pub fn inner(&self) -> &P {
        &self.inner
    }
}

#[async_trait]
impl<P: LLMProvider + 'static> LLMProvider for RetryingProvider<P> {
    async fn send_message(&self, request: LLMRequest) -> LLMResult<LLMResponse> {
        let inner = self.inner.clone();
        let req = request.clone();

        with_retry(self.config.clone(), self.policy, move || {
            let inner = inner.clone();
            let req = req.clone();
            async move { inner.send_message(req).await }
        })
        .await
    }

    async fn send_message_stream(
        &self,
        request: LLMRequest,
    ) -> LLMResult<Pin<Box<dyn Stream<Item = LLMResult<StreamChunk>> + Send>>> {
        // For streaming, we retry the connection but not individual chunks
        let inner = self.inner.clone();
        let req = request.clone();

        with_retry(self.config.clone(), self.policy, move || {
            let inner = inner.clone();
            let req = req.clone();
            async move { inner.send_message_stream(req).await }
        })
        .await
    }

    fn provider_name(&self) -> &'static str {
        self.inner.provider_name()
    }

    fn model_id(&self) -> &str {
        self.inner.model_id()
    }
}

#[cfg(test)]
mod tests;
