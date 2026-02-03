//! Global rate limiter for LLM API calls.
//!
//! Wraps any [`LLMProvider`] with concurrency limiting (semaphore),
//! requests-per-minute throttling (token bucket), and global backoff
//! when any caller receives a 429.

use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock, Semaphore};

use super::provider::{LLMProvider, LLMResult};
use super::types::{LLMError, LLMRequest, LLMResponse, StreamChunk};

/// Configuration for the rate limiter.
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum concurrent LLM API calls across all agents.
    pub max_concurrent_calls: usize,
    /// Requests per minute (0 = unlimited).
    pub requests_per_minute: usize,
    /// Initial global backoff delay on 429.
    pub global_backoff_initial_ms: u64,
    /// Maximum global backoff delay.
    pub global_backoff_max_ms: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_concurrent_calls: crate::constants::RATE_LIMIT_MAX_CONCURRENT_CALLS,
            requests_per_minute: crate::constants::RATE_LIMIT_REQUESTS_PER_MINUTE,
            global_backoff_initial_ms: crate::constants::RATE_LIMIT_GLOBAL_BACKOFF_INITIAL_MS,
            global_backoff_max_ms: crate::constants::RATE_LIMIT_GLOBAL_BACKOFF_MAX_MS,
        }
    }
}

/// Simple token bucket for RPM limiting.
struct TokenBucket {
    tokens: f64,
    capacity: f64,
    refill_rate: f64, // tokens per second
    last_refill: Instant,
}

impl TokenBucket {
    fn new(requests_per_minute: usize) -> Self {
        let capacity = requests_per_minute as f64;
        Self {
            tokens: capacity,
            capacity,
            refill_rate: capacity / 60.0,
            last_refill: Instant::now(),
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.capacity);
        self.last_refill = now;
    }

    /// Wait until a token is available, then consume it.
    async fn acquire(&mut self) {
        loop {
            self.refill();
            if self.tokens >= 1.0 {
                self.tokens -= 1.0;
                return;
            }
            let wait = (1.0 - self.tokens) / self.refill_rate;
            tokio::time::sleep(Duration::from_secs_f64(wait)).await;
        }
    }
}

/// Shared state for global backoff across all callers.
struct GlobalBackoff {
    /// If set, all callers should wait until this instant.
    until: Option<Instant>,
    /// Current backoff delay (escalates on repeated 429s).
    current_delay_ms: u64,
    /// Base delay for reset.
    initial_delay_ms: u64,
    /// Cap.
    max_delay_ms: u64,
}

impl GlobalBackoff {
    fn new(initial_ms: u64, max_ms: u64) -> Self {
        Self {
            until: None,
            current_delay_ms: initial_ms,
            initial_delay_ms: initial_ms,
            max_delay_ms: max_ms,
        }
    }

    fn record_rate_limit(&mut self, retry_after_ms: u64) {
        // Always respect the server's retry-after hint — never cap it below what was asked.
        // Only cap our own escalating delay.
        let our_delay = self.current_delay_ms.min(self.max_delay_ms);
        let delay = retry_after_ms.max(our_delay);
        self.until = Some(Instant::now() + Duration::from_millis(delay));
        self.current_delay_ms = (our_delay * 2).min(self.max_delay_ms);
        tracing::warn!("Global rate limit backoff set for {}ms (server asked {}ms)", delay, retry_after_ms);
    }

    fn record_success(&mut self) {
        if let Some(until) = self.until {
            if Instant::now() >= until {
                self.until = None;
                self.current_delay_ms = self.initial_delay_ms;
            }
        }
    }

    fn remaining_wait(&self) -> Option<Duration> {
        self.until.and_then(|until| {
            let now = Instant::now();
            if now < until {
                Some(until - now)
            } else {
                None
            }
        })
    }
}

/// Rate-limiting wrapper around any [`LLMProvider`].
pub struct RateLimitedProvider<P: LLMProvider> {
    inner: Arc<P>,
    semaphore: Arc<Semaphore>,
    token_bucket: Option<Arc<Mutex<TokenBucket>>>,
    global_backoff: Arc<RwLock<GlobalBackoff>>,
}

impl<P: LLMProvider + 'static> RateLimitedProvider<P> {
    pub fn new(provider: P, config: RateLimitConfig) -> Self {
        let token_bucket = if config.requests_per_minute > 0 {
            Some(Arc::new(Mutex::new(TokenBucket::new(config.requests_per_minute))))
        } else {
            None
        };

        Self {
            inner: Arc::new(provider),
            semaphore: Arc::new(Semaphore::new(config.max_concurrent_calls)),
            token_bucket,
            global_backoff: Arc::new(RwLock::new(GlobalBackoff::new(config.global_backoff_initial_ms, config.global_backoff_max_ms))),
        }
    }

    pub fn with_defaults(provider: P) -> Self {
        Self::new(provider, RateLimitConfig::default())
    }

    async fn wait_for_backoff(&self) {
        let wait = self.global_backoff.read().await.remaining_wait();
        if let Some(duration) = wait {
            tracing::debug!("Waiting {:?} for global rate limit backoff", duration);
            tokio::time::sleep(duration).await;
        }
    }

    async fn acquire_rpm_token(&self) {
        if let Some(ref bucket) = self.token_bucket {
            bucket.lock().await.acquire().await;
        }
    }

    async fn on_rate_limited(&self, retry_after_ms: u64) {
        self.global_backoff.write().await.record_rate_limit(retry_after_ms);
    }

    async fn on_success(&self) {
        self.global_backoff.write().await.record_success();
    }
}

#[async_trait]
impl<P: LLMProvider + 'static> LLMProvider for RateLimitedProvider<P> {
    async fn send_message(&self, request: LLMRequest) -> LLMResult<LLMResponse> {
        self.wait_for_backoff().await;
        let _permit = self.semaphore.acquire().await.expect("semaphore closed");
        self.acquire_rpm_token().await;

        match self.inner.send_message(request).await {
            Ok(resp) => {
                self.on_success().await;
                Ok(resp)
            }
            Err(LLMError::RateLimited { retry_after_ms }) => {
                self.on_rate_limited(retry_after_ms).await;
                Err(LLMError::RateLimited { retry_after_ms })
            }
            Err(e) => Err(e),
        }
    }

    async fn send_message_stream(&self, request: LLMRequest) -> LLMResult<Pin<Box<dyn Stream<Item = LLMResult<StreamChunk>> + Send>>> {
        self.wait_for_backoff().await;
        let _permit = self.semaphore.acquire().await.expect("semaphore closed");
        self.acquire_rpm_token().await;

        match self.inner.send_message_stream(request).await {
            Ok(stream) => {
                self.on_success().await;
                Ok(stream)
            }
            Err(LLMError::RateLimited { retry_after_ms }) => {
                self.on_rate_limited(retry_after_ms).await;
                Err(LLMError::RateLimited { retry_after_ms })
            }
            Err(e) => Err(e),
        }
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
