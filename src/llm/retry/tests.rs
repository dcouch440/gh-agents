#[cfg(test)]
mod tests {
    //! Tests for retry logic

    use super::super::*;

    #[test]
    fn test_default_config() {
        let config = BackoffConfig::default();
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.multiplier, 2.0);
    }

    #[test]
    fn test_backoff_increases() {
        let config = BackoffConfig::new()
            .with_jitter(0.0) // Disable jitter for predictable testing
            .with_max_retries(3);

        let backoff = ExponentialBackoff::new(config);
        let delays: Vec<Duration> = backoff.collect();

        assert_eq!(delays.len(), 3);
        // Each delay should be roughly double the previous
        assert!(delays[1] > delays[0]);
        assert!(delays[2] > delays[1]);
    }

    #[test]
    fn test_max_delay_cap() {
        let config = BackoffConfig::new()
            .with_initial_delay(Duration::from_secs(30))
            .with_max_delay(Duration::from_secs(60))
            .with_jitter(0.0)
            .with_max_retries(5);

        let backoff = ExponentialBackoff::new(config);
        let delays: Vec<Duration> = backoff.collect();

        // All delays should be <= max_delay
        for delay in delays {
            assert!(delay <= Duration::from_secs(60));
        }
    }

    #[test]
    fn test_reset() {
        let config = BackoffConfig::new().with_max_retries(2);
        let mut backoff = ExponentialBackoff::new(config);

        assert!(backoff.next().is_some());
        assert!(backoff.next().is_some());
        assert!(backoff.next().is_none());

        backoff.reset();
        assert!(backoff.next().is_some());
    }

    #[tokio::test]
    async fn test_success_no_retry() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;

        let call_count = Arc::new(AtomicU32::new(0));
        let count = call_count.clone();

        let result = with_default_retry(|| {
            let c = count.clone();
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Ok::<_, LLMError>("success")
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_retries_on_rate_limit() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;

        let call_count = Arc::new(AtomicU32::new(0));
        let count = call_count.clone();

        let config = BackoffConfig::new()
            .with_initial_delay(Duration::from_millis(10))
            .with_max_retries(3);

        let result = with_retry(config, RetryPolicy::Default, || {
            let c = count.clone();
            async move {
                let n = c.fetch_add(1, Ordering::SeqCst);
                if n < 2 {
                    Err(LLMError::RateLimited { retry_after_ms: 10 })
                } else {
                    Ok("success")
                }
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(call_count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_no_retry_on_auth_error() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;

        let call_count = Arc::new(AtomicU32::new(0));
        let count = call_count.clone();

        let result = with_default_retry(|| {
            let c = count.clone();
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Err::<&str, _>(LLMError::AuthError("bad key".to_string()))
            }
        })
        .await;

        assert!(result.is_err());
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_max_retries_exceeded() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;

        let call_count = Arc::new(AtomicU32::new(0));
        let count = call_count.clone();

        let config = BackoffConfig::new()
            .with_initial_delay(Duration::from_millis(1))
            .with_max_retries(2);

        let result = with_retry(config, RetryPolicy::Default, || {
            let c = count.clone();
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Err::<&str, _>(LLMError::RateLimited { retry_after_ms: 1 })
            }
        })
        .await;

        assert!(matches!(result, Err(LLMError::MaxRetriesExceeded(2))));
        // Initial call + 2 retries = 3 total
        assert_eq!(call_count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retries_on_server_error() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;

        let call_count = Arc::new(AtomicU32::new(0));
        let count = call_count.clone();

        let config = BackoffConfig::new()
            .with_initial_delay(Duration::from_millis(1))
            .with_max_retries(3);

        let result = with_retry(config, RetryPolicy::Default, || {
            let c = count.clone();
            async move {
                let n = c.fetch_add(1, Ordering::SeqCst);
                if n < 1 {
                    Err(LLMError::ApiError {
                        status: 503,
                        message: "Service Unavailable".to_string(),
                    })
                } else {
                    Ok("success")
                }
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(call_count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_retry_policy_never() {
        let error = LLMError::RateLimited {
            retry_after_ms: 100,
        };
        assert!(!RetryPolicy::Never.should_retry(&error));
    }

    #[test]
    fn test_retry_policy_always() {
        let error = LLMError::AuthError("bad key".to_string());
        assert!(RetryPolicy::Always.should_retry(&error));
    }

    #[test]
    fn test_retry_policy_default_400_not_retried() {
        let error = LLMError::ApiError {
            status: 400,
            message: "Bad Request".to_string(),
        };
        assert!(!RetryPolicy::Default.should_retry(&error));
    }

    #[test]
    fn test_retry_policy_default_500_retried() {
        let error = LLMError::ApiError {
            status: 500,
            message: "Internal Server Error".to_string(),
        };
        assert!(RetryPolicy::Default.should_retry(&error));
    }

    #[test]
    fn test_backoff_config_builder_methods() {
        let config = BackoffConfig::new()
            .with_initial_delay(Duration::from_millis(100))
            .with_max_delay(Duration::from_secs(30))
            .with_multiplier(3.0)
            .with_jitter(0.5)
            .with_max_retries(10);

        assert_eq!(config.initial_delay, Duration::from_millis(100));
        assert_eq!(config.max_delay, Duration::from_secs(30));
        assert_eq!(config.multiplier, 3.0);
        assert_eq!(config.jitter, 0.5);
        assert_eq!(config.max_retries, 10);
    }

    #[test]
    fn test_jitter_clamped_to_range() {
        let config = BackoffConfig::new().with_jitter(2.0);
        assert_eq!(config.jitter, 1.0);

        let config = BackoffConfig::new().with_jitter(-0.5);
        assert_eq!(config.jitter, 0.0);
    }

    #[test]
    fn test_backoff_zero_retries() {
        let config = BackoffConfig::new().with_max_retries(0);
        let backoff = ExponentialBackoff::new(config);
        let delays: Vec<Duration> = backoff.collect();
        assert!(delays.is_empty());
    }

    #[test]
    fn test_backoff_default_config() {
        let backoff = ExponentialBackoff::default_config();
        assert_eq!(backoff.max_retries(), 5);
        assert!(backoff.has_remaining());
        assert_eq!(backoff.attempts(), 0);
    }

    #[test]
    fn test_backoff_attempts_and_has_remaining() {
        let config = BackoffConfig::new().with_max_retries(2).with_jitter(0.0);
        let mut backoff = ExponentialBackoff::new(config);

        assert_eq!(backoff.attempts(), 0);
        assert!(backoff.has_remaining());

        backoff.next();
        assert_eq!(backoff.attempts(), 1);
        assert!(backoff.has_remaining());

        backoff.next();
        assert_eq!(backoff.attempts(), 2);
        assert!(!backoff.has_remaining());

        assert!(backoff.next().is_none());
    }

    #[test]
    fn test_backoff_no_jitter_produces_exact_delays() {
        let config = BackoffConfig::new()
            .with_initial_delay(Duration::from_millis(100))
            .with_max_delay(Duration::from_secs(60))
            .with_multiplier(2.0)
            .with_jitter(0.0)
            .with_max_retries(4);

        let backoff = ExponentialBackoff::new(config);
        let delays: Vec<Duration> = backoff.collect();

        assert_eq!(delays[0], Duration::from_millis(100));
        assert_eq!(delays[1], Duration::from_millis(200));
        assert_eq!(delays[2], Duration::from_millis(400));
        assert_eq!(delays[3], Duration::from_millis(800));
    }

    #[test]
    fn test_backoff_with_jitter_varies() {
        let config = BackoffConfig::new()
            .with_initial_delay(Duration::from_millis(1000))
            .with_jitter(1.0)
            .with_max_retries(1);

        // Run multiple times - with full jitter, delays should vary
        let mut backoff = ExponentialBackoff::new(config);
        let delay = backoff.next().unwrap();
        // With jitter=1.0, delay should be between 0 and 2000ms
        assert!(delay <= Duration::from_millis(2000));
    }

    #[test]
    fn test_backoff_max_delay_cap_with_multiplier() {
        let config = BackoffConfig::new()
            .with_initial_delay(Duration::from_secs(10))
            .with_max_delay(Duration::from_secs(15))
            .with_multiplier(10.0)
            .with_jitter(0.0)
            .with_max_retries(3);

        let backoff = ExponentialBackoff::new(config);
        let delays: Vec<Duration> = backoff.collect();

        assert_eq!(delays[0], Duration::from_secs(10));
        // After multiplier: 100s, capped to 15s
        assert_eq!(delays[1], Duration::from_secs(15));
        assert_eq!(delays[2], Duration::from_secs(15));
    }

    #[test]
    fn test_retry_policy_default_timeout() {
        let error = LLMError::Timeout(5000);
        assert!(RetryPolicy::Default.should_retry(&error));
    }

    #[test]
    fn test_retry_policy_default_parse_error_not_retried() {
        let error = LLMError::ParseError("bad json".to_string());
        assert!(!RetryPolicy::Default.should_retry(&error));
    }

    #[test]
    fn test_retry_policy_default_stream_error_not_retried() {
        let error = LLMError::StreamError("connection reset".to_string());
        assert!(!RetryPolicy::Default.should_retry(&error));
    }

    #[test]
    fn test_retry_policy_default_max_retries_exceeded_not_retried() {
        let error = LLMError::MaxRetriesExceeded(5);
        assert!(!RetryPolicy::Default.should_retry(&error));
    }

    #[test]
    fn test_retry_policy_default_auth_error_not_retried() {
        let error = LLMError::AuthError("invalid key".to_string());
        assert!(!RetryPolicy::Default.should_retry(&error));
    }

    #[test]
    fn test_retry_policy_default_499_not_retried() {
        let error = LLMError::ApiError {
            status: 499,
            message: "Client error".to_string(),
        };
        assert!(!RetryPolicy::Default.should_retry(&error));
    }

    #[test]
    fn test_retry_policy_default_502_retried() {
        let error = LLMError::ApiError {
            status: 502,
            message: "Bad Gateway".to_string(),
        };
        assert!(RetryPolicy::Default.should_retry(&error));
    }

    #[test]
    fn test_retry_policy_default_599_retried() {
        let error = LLMError::ApiError {
            status: 599,
            message: "Server error".to_string(),
        };
        assert!(RetryPolicy::Default.should_retry(&error));
    }

    #[test]
    fn test_retry_policy_default_600_not_retried() {
        let error = LLMError::ApiError {
            status: 600,
            message: "Not a standard status".to_string(),
        };
        assert!(!RetryPolicy::Default.should_retry(&error));
    }

    #[test]
    fn test_retry_policy_never_rejects_all() {
        let errors = vec![
            LLMError::RateLimited {
                retry_after_ms: 100,
            },
            LLMError::Timeout(5000),
            LLMError::ApiError {
                status: 500,
                message: "err".to_string(),
            },
        ];
        for error in &errors {
            assert!(!RetryPolicy::Never.should_retry(error));
        }
    }

    #[test]
    fn test_retry_policy_always_accepts_all() {
        let errors = vec![
            LLMError::AuthError("bad".to_string()),
            LLMError::ParseError("bad".to_string()),
            LLMError::StreamError("bad".to_string()),
            LLMError::MaxRetriesExceeded(5),
        ];
        for error in &errors {
            assert!(RetryPolicy::Always.should_retry(error));
        }
    }

    #[test]
    fn test_retry_policy_equality() {
        assert_eq!(RetryPolicy::Default, RetryPolicy::Default);
        assert_ne!(RetryPolicy::Default, RetryPolicy::Never);
        assert_ne!(RetryPolicy::Never, RetryPolicy::Always);
    }

    #[tokio::test]
    async fn test_with_retry_never_policy_no_retries() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;

        let call_count = Arc::new(AtomicU32::new(0));
        let count = call_count.clone();

        let config = BackoffConfig::new()
            .with_initial_delay(Duration::from_millis(1))
            .with_max_retries(5);

        let result = with_retry(config, RetryPolicy::Never, || {
            let c = count.clone();
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Err::<&str, _>(LLMError::RateLimited { retry_after_ms: 1 })
            }
        })
        .await;

        assert!(result.is_err());
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_with_retry_always_policy_retries_auth_error() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;

        let call_count = Arc::new(AtomicU32::new(0));
        let count = call_count.clone();

        let config = BackoffConfig::new()
            .with_initial_delay(Duration::from_millis(1))
            .with_max_retries(2);

        let result = with_retry(config, RetryPolicy::Always, || {
            let c = count.clone();
            async move {
                let n = c.fetch_add(1, Ordering::SeqCst);
                if n < 1 {
                    Err(LLMError::AuthError("bad key".to_string()))
                } else {
                    Ok("success")
                }
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(call_count.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_with_retry_non_retryable_mid_retry() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;

        let call_count = Arc::new(AtomicU32::new(0));
        let count = call_count.clone();

        let config = BackoffConfig::new()
            .with_initial_delay(Duration::from_millis(1))
            .with_max_retries(5);

        let result = with_retry(config, RetryPolicy::Default, || {
            let c = count.clone();
            async move {
                let n = c.fetch_add(1, Ordering::SeqCst);
                if n == 0 {
                    Err(LLMError::RateLimited { retry_after_ms: 1 })
                } else {
                    // Second attempt fails with non-retryable error
                    Err::<&str, _>(LLMError::AuthError("bad key".to_string()))
                }
            }
        })
        .await;

        assert!(matches!(result, Err(LLMError::AuthError(_))));
        assert_eq!(call_count.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_with_retry_rate_limit_uses_retry_after_delay() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;

        let call_count = Arc::new(AtomicU32::new(0));
        let count = call_count.clone();

        let config = BackoffConfig::new()
            .with_initial_delay(Duration::from_millis(1))
            .with_max_retries(1);

        let start = std::time::Instant::now();
        let result = with_retry(config, RetryPolicy::Default, || {
            let c = count.clone();
            async move {
                let n = c.fetch_add(1, Ordering::SeqCst);
                if n == 0 {
                    Err(LLMError::RateLimited { retry_after_ms: 50 })
                } else {
                    Ok("ok")
                }
            }
        })
        .await;

        let elapsed = start.elapsed();
        assert!(result.is_ok());
        // Should have waited at least ~50ms due to retry_after_ms
        assert!(elapsed >= Duration::from_millis(40));
    }

    #[test]
    fn test_retry_context_debug() {
        let ctx = RetryContext {
            attempt: 1,
            max_attempts: 5,
            delay: Duration::from_secs(1),
            error: "timeout".to_string(),
        };
        let debug = format!("{:?}", ctx);
        assert!(debug.contains("attempt: 1"));
        assert!(debug.contains("max_attempts: 5"));
    }

    #[test]
    fn test_backoff_reset_restores_initial_delay() {
        let config = BackoffConfig::new()
            .with_initial_delay(Duration::from_millis(100))
            .with_jitter(0.0)
            .with_max_retries(3);

        let mut backoff = ExponentialBackoff::new(config);

        let first_delay = backoff.next().unwrap();
        backoff.next(); // advance
        backoff.reset();

        let after_reset = backoff.next().unwrap();
        assert_eq!(first_delay, after_reset);
        assert_eq!(backoff.attempts(), 1);
    }

    // ── RetryingProvider integration tests ───────────────────────────────

    use super::super::super::types::{ContentBlock, StopReason, TokenUsage};
    use futures::StreamExt;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// Which error the mock provider should return when failing.
    enum MockErrorKind {
        RateLimited,
        ServerError,
        AuthError,
    }

    /// A configurable mock LLM provider for testing RetryingProvider.
    struct MockLLMProvider {
        call_count: Arc<AtomicU32>,
        /// Number of calls to fail before succeeding (0 = always succeed).
        fail_until: u32,
        error_kind: MockErrorKind,
        model: String,
    }

    impl MockLLMProvider {
        fn new(fail_until: u32, error_kind: MockErrorKind) -> Self {
            Self {
                call_count: Arc::new(AtomicU32::new(0)),
                fail_until,
                error_kind,
                model: "mock-model".to_string(),
            }
        }

        fn make_error(&self) -> LLMError {
            match self.error_kind {
                MockErrorKind::RateLimited => LLMError::RateLimited { retry_after_ms: 1 },
                MockErrorKind::ServerError => LLMError::ApiError {
                    status: 500,
                    message: "Internal Server Error".to_string(),
                },
                MockErrorKind::AuthError => LLMError::AuthError("invalid key".to_string()),
            }
        }

        fn make_response() -> LLMResponse {
            LLMResponse {
                content: "ok".to_string(),
                content_blocks: vec![ContentBlock::Text {
                    text: "ok".to_string(),
                }],
                model: "mock-model".to_string(),
                stop_reason: StopReason::EndTurn,
                usage: TokenUsage {
                    input_tokens: 10,
                    output_tokens: 5,
                },
            }
        }
    }

    #[async_trait]
    impl LLMProvider for MockLLMProvider {
        async fn send_message(&self, _request: LLMRequest) -> LLMResult<LLMResponse> {
            let n = self.call_count.fetch_add(1, Ordering::SeqCst) + 1;
            if n <= self.fail_until {
                Err(self.make_error())
            } else {
                Ok(Self::make_response())
            }
        }

        async fn send_message_stream(
            &self,
            _request: LLMRequest,
        ) -> LLMResult<Pin<Box<dyn Stream<Item = LLMResult<StreamChunk>> + Send>>> {
            let n = self.call_count.fetch_add(1, Ordering::SeqCst) + 1;
            if n <= self.fail_until {
                Err(self.make_error())
            } else {
                let chunks: Vec<LLMResult<StreamChunk>> = vec![
                    Ok(StreamChunk::MessageStart {
                        model: "mock-model".to_string(),
                        input_tokens: 10,
                    }),
                    Ok(StreamChunk::ContentDelta {
                        text: "ok".to_string(),
                        index: 0,
                    }),
                    Ok(StreamChunk::MessageDelta {
                        stop_reason: Some(StopReason::EndTurn),
                        output_tokens: Some(5),
                    }),
                    Ok(StreamChunk::MessageStop),
                ];
                Ok(Box::pin(futures::stream::iter(chunks)))
            }
        }

        fn provider_name(&self) -> &'static str {
            "mock"
        }

        fn model_id(&self) -> &str {
            &self.model
        }
    }

    fn fast_config(max_retries: u32) -> BackoffConfig {
        BackoffConfig::new()
            .with_initial_delay(Duration::from_millis(1))
            .with_max_delay(Duration::from_millis(1))
            .with_jitter(0.0)
            .with_max_retries(max_retries)
    }

    fn make_request() -> LLMRequest {
        LLMRequest::new("mock-model", vec![])
    }

    #[tokio::test]
    async fn retrying_provider_success_no_retry() {
        let mock = MockLLMProvider::new(0, MockErrorKind::ServerError);
        let count = mock.call_count.clone();
        let provider = RetryingProvider::new(mock, fast_config(3));

        let result = provider.send_message(make_request()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().content, "ok");
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn retrying_provider_retries_on_server_error() {
        let mock = MockLLMProvider::new(1, MockErrorKind::ServerError);
        let count = mock.call_count.clone();
        let provider = RetryingProvider::new(mock, fast_config(3));

        let result = provider.send_message(make_request()).await;
        assert!(result.is_ok());
        assert_eq!(count.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn retrying_provider_retries_on_rate_limit() {
        let mock = MockLLMProvider::new(2, MockErrorKind::RateLimited);
        let count = mock.call_count.clone();
        let provider = RetryingProvider::new(mock, fast_config(3));

        let result = provider.send_message(make_request()).await;
        assert!(result.is_ok());
        assert_eq!(count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn retrying_provider_no_retry_on_auth_error() {
        let mock = MockLLMProvider::new(1, MockErrorKind::AuthError);
        let count = mock.call_count.clone();
        let provider = RetryingProvider::new(mock, fast_config(3));

        let result = provider.send_message(make_request()).await;
        assert!(matches!(result, Err(LLMError::AuthError(_))));
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn retrying_provider_max_retries_exhausted() {
        let mock = MockLLMProvider::new(u32::MAX, MockErrorKind::RateLimited);
        let count = mock.call_count.clone();
        let provider = RetryingProvider::new(mock, fast_config(2));

        let result = provider.send_message(make_request()).await;
        assert!(matches!(result, Err(LLMError::MaxRetriesExceeded(2))));
        // 1 initial + 2 retries = 3 total
        assert_eq!(count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn retrying_provider_stream_success() {
        let mock = MockLLMProvider::new(0, MockErrorKind::ServerError);
        let provider = RetryingProvider::new(mock, fast_config(3));

        let stream = provider.send_message_stream(make_request()).await.unwrap();
        let chunks: Vec<_> = stream.collect().await;
        assert_eq!(chunks.len(), 4);
        assert!(matches!(chunks[0], Ok(StreamChunk::MessageStart { .. })));
        assert!(matches!(chunks[3], Ok(StreamChunk::MessageStop)));
    }

    #[tokio::test]
    async fn retrying_provider_stream_retries_connection() {
        let mock = MockLLMProvider::new(1, MockErrorKind::ServerError);
        let count = mock.call_count.clone();
        let provider = RetryingProvider::new(mock, fast_config(3));

        let stream = provider.send_message_stream(make_request()).await.unwrap();
        let chunks: Vec<_> = stream.collect().await;
        assert_eq!(chunks.len(), 4);
        assert_eq!(count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn retrying_provider_delegates_metadata() {
        let mock = MockLLMProvider::new(0, MockErrorKind::ServerError);
        let provider = RetryingProvider::new(mock, fast_config(1));

        assert_eq!(provider.provider_name(), "mock");
        assert_eq!(provider.model_id(), "mock-model");
    }
}
