#[cfg(test)]
mod tests {
    use reqwest::header::HeaderMap;

    use crate::llm::sse_provider::SseProviderAdapter;
    use crate::llm::types::{LLMError, LLMRequest, LLMResponse, StreamChunk};
    use crate::llm::LLMResult;

    /// Minimal test adapter to verify default trait methods and construction.
    #[derive(Clone)]
    struct TestAdapter;

    impl SseProviderAdapter for TestAdapter {
        fn provider_name(&self) -> &'static str {
            "test"
        }

        fn model_id(&self) -> &str {
            "test-model"
        }

        fn endpoint_url(&self) -> String {
            "https://test.example.com/v1/chat".to_string()
        }

        fn default_headers(&self) -> Result<HeaderMap, LLMError> {
            Ok(HeaderMap::new())
        }

        fn timeout_secs(&self) -> u64 {
            30
        }

        fn build_request_body(&self, _request: &LLMRequest, _stream: bool) -> serde_json::Value {
            serde_json::json!({})
        }

        fn parse_response(&self, _body: &[u8]) -> Result<LLMResponse, LLMError> {
            Err(LLMError::ParseError("not implemented".to_string()))
        }

        fn parse_sse_line(&self, _line: &str) -> Option<LLMResult<StreamChunk>> {
            None
        }

        fn handle_error(&self, status: u16, body: &str, retry_after_ms: Option<u64>) -> LLMError {
            match status {
                429 => LLMError::RateLimited {
                    retry_after_ms: retry_after_ms.unwrap_or(60000),
                },
                _ => LLMError::ApiError {
                    status,
                    message: body.to_string(),
                },
            }
        }
    }

    // ── Default trait method tests ───────────────────────────────────────

    #[test]
    fn default_pre_stream_events_empty() {
        let adapter = TestAdapter;
        assert!(adapter.pre_stream_events().is_empty());
    }

    #[test]
    fn default_post_stream_events_empty() {
        let adapter = TestAdapter;
        assert!(adapter.post_stream_events().is_empty());
    }

    // ── Adapter method tests ─────────────────────────────────────────────

    #[test]
    fn adapter_provider_name() {
        let adapter = TestAdapter;
        assert_eq!(adapter.provider_name(), "test");
    }

    #[test]
    fn adapter_model_id() {
        let adapter = TestAdapter;
        assert_eq!(adapter.model_id(), "test-model");
    }

    #[test]
    fn adapter_endpoint_url() {
        let adapter = TestAdapter;
        assert_eq!(
            adapter.endpoint_url(),
            "https://test.example.com/v1/chat"
        );
    }

    // ── SseHttpProvider construction ─────────────────────────────────────

    #[test]
    fn sse_http_provider_new_succeeds() {
        use crate::llm::sse_provider::SseHttpProvider;

        let adapter = TestAdapter;
        let provider = SseHttpProvider::new(adapter);
        assert!(provider.is_ok());
    }

    #[test]
    fn sse_http_provider_delegates_names() {
        use crate::llm::sse_provider::SseHttpProvider;
        use crate::llm::LLMProvider;

        let adapter = TestAdapter;
        let provider = SseHttpProvider::new(adapter).unwrap();
        assert_eq!(provider.provider_name(), "test");
        assert_eq!(provider.model_id(), "test-model");
    }

    #[test]
    fn sse_http_provider_is_clone() {
        use crate::llm::sse_provider::SseHttpProvider;

        let adapter = TestAdapter;
        let provider = SseHttpProvider::new(adapter).unwrap();
        let cloned = provider.clone();
        assert_eq!(cloned.adapter().provider_name(), "test");
    }

    // ── Error mapping via adapter ────────────────────────────────────────

    #[test]
    fn adapter_handle_error_429_rate_limited() {
        let adapter = TestAdapter;
        let err = adapter.handle_error(429, "too many requests", Some(5000));
        match err {
            LLMError::RateLimited { retry_after_ms } => assert_eq!(retry_after_ms, 5000),
            other => panic!("expected RateLimited, got: {other}"),
        }
    }

    #[test]
    fn adapter_handle_error_429_default_retry() {
        let adapter = TestAdapter;
        let err = adapter.handle_error(429, "too many requests", None);
        match err {
            LLMError::RateLimited { retry_after_ms } => assert_eq!(retry_after_ms, 60000),
            other => panic!("expected RateLimited, got: {other}"),
        }
    }

    #[test]
    fn adapter_handle_error_500_api_error() {
        let adapter = TestAdapter;
        let err = adapter.handle_error(500, "internal error", None);
        match err {
            LLMError::ApiError { status, message } => {
                assert_eq!(status, 500);
                assert_eq!(message, "internal error");
            }
            other => panic!("expected ApiError, got: {other}"),
        }
    }
}
