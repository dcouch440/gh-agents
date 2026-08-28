#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::super::*;
    use crate::llm::{LLMError, LLMRequest, LLMResponse, Message, StreamChunk, TokenUsage};
    use futures::stream;
    use futures::StreamExt;
    use tokio_util::bytes::Bytes;

    // ── safe_line_stream tests ───────────────────────────────────────────

    #[tokio::test]
    async fn splits_lines_on_newline() {
        let chunks = vec![Ok(Bytes::from("line1\nline2\nline3\n"))];
        let byte_stream = stream::iter(chunks);

        let lines: Vec<_> = safe_line_stream(byte_stream, DEFAULT_MAX_STREAM_BUFFER)
            .collect()
            .await;

        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].as_deref().unwrap(), "line1");
        assert_eq!(lines[1].as_deref().unwrap(), "line2");
        assert_eq!(lines[2].as_deref().unwrap(), "line3");
    }

    #[tokio::test]
    async fn handles_lines_split_across_chunks() {
        let chunks = vec![
            Ok(Bytes::from("hel")),
            Ok(Bytes::from("lo\nwor")),
            Ok(Bytes::from("ld\n")),
        ];
        let byte_stream = stream::iter(chunks);

        let lines: Vec<_> = safe_line_stream(byte_stream, DEFAULT_MAX_STREAM_BUFFER)
            .collect()
            .await;

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].as_deref().unwrap(), "hello");
        assert_eq!(lines[1].as_deref().unwrap(), "world");
    }

    #[tokio::test]
    async fn preserves_multibyte_utf8_split_across_chunks() {
        // "Hello 😊 world\n" in UTF-8.
        // The emoji U+1F60A is 4 bytes: F0 9F 98 8A.
        // Split the emoji across two chunks.
        let mut full = "Hello 😊 world\n".as_bytes().to_vec();
        let split_point = 8; // In the middle of the emoji (after F0 9F)
        let chunk2 = full.split_off(split_point);
        let chunk1 = full;

        let chunks = vec![Ok(Bytes::from(chunk1)), Ok(Bytes::from(chunk2))];
        let byte_stream = stream::iter(chunks);

        let lines: Vec<_> = safe_line_stream(byte_stream, DEFAULT_MAX_STREAM_BUFFER)
            .collect()
            .await;

        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].as_deref().unwrap(), "Hello 😊 world");
    }

    #[tokio::test]
    async fn preserves_cjk_characters_split_across_chunks() {
        // "你好世界\n" — each CJK char is 3 bytes in UTF-8.
        // Split in the middle of 好 (bytes: E5 A5 BD).
        let full = "你好世界\n".as_bytes().to_vec();
        let split_point = 4; // After 你 (3 bytes) + first byte of 好
        let chunk1 = full[..split_point].to_vec();
        let chunk2 = full[split_point..].to_vec();

        let chunks = vec![Ok(Bytes::from(chunk1)), Ok(Bytes::from(chunk2))];
        let byte_stream = stream::iter(chunks);

        let lines: Vec<_> = safe_line_stream(byte_stream, DEFAULT_MAX_STREAM_BUFFER)
            .collect()
            .await;

        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].as_deref().unwrap(), "你好世界");
    }

    #[tokio::test]
    async fn yields_empty_lines() {
        let chunks = vec![Ok(Bytes::from("a\n\nb\n"))];
        let byte_stream = stream::iter(chunks);

        let lines: Vec<_> = safe_line_stream(byte_stream, DEFAULT_MAX_STREAM_BUFFER)
            .collect()
            .await;

        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].as_deref().unwrap(), "a");
        assert_eq!(lines[1].as_deref().unwrap(), "");
        assert_eq!(lines[2].as_deref().unwrap(), "b");
    }

    #[tokio::test]
    async fn buffer_cap_triggers_error() {
        // Set a tiny 10-byte cap
        let chunks = vec![Ok(Bytes::from("this line is way too long for the buffer"))];
        let byte_stream = stream::iter(chunks);

        let lines: Vec<_> = safe_line_stream(byte_stream, 10).collect().await;

        assert_eq!(lines.len(), 1);
        assert!(lines[0].is_err());
        let err = lines[0].as_ref().unwrap_err();
        match err {
            LLMError::StreamError(msg) => assert!(msg.contains("exceeded")),
            _ => panic!("expected StreamError"),
        }
    }

    /// Produce a genuine `reqwest::Error` without touching the network — port 1
    /// on loopback is never listening, so the connect attempt always fails.
    async fn connect_error() -> reqwest::Error {
        reqwest::Client::new()
            .get("http://127.0.0.1:1/")
            .send()
            .await
            .expect_err("connect to 127.0.0.1:1 must fail")
    }

    #[tokio::test]
    async fn transport_error_forwarded() {
        let chunks: Vec<Result<Bytes, reqwest::Error>> =
            vec![Ok(Bytes::from("ok\n")), Err(connect_error().await)];
        let byte_stream = stream::iter(chunks);

        let lines: Vec<_> = safe_line_stream(byte_stream, DEFAULT_MAX_STREAM_BUFFER)
            .collect()
            .await;

        // The complete line arrives, then the transport failure.
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].as_deref().unwrap(), "ok");

        // The reqwest error must survive intact — classification downstream
        // depends on it, and stringifying it here is what previously reduced
        // every mid-stream failure to "error decoding response body".
        match lines[1].as_ref().unwrap_err() {
            LLMError::StreamTransport(e) => {
                assert!(
                    e.is_connect() || e.is_request(),
                    "lost classification: {e:?}"
                )
            }
            other => panic!("expected StreamTransport, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn buffer_overflow_still_yields_stream_error() {
        // Protocol-level faults keep the plain `StreamError` variant — only
        // transport failures carry a `reqwest::Error`.
        let big = Bytes::from(vec![b'x'; 128]);
        let byte_stream = stream::iter(vec![Ok(big)]);

        let lines: Vec<_> = safe_line_stream(byte_stream, 16).collect().await;

        assert_eq!(lines.len(), 1);
        match lines[0].as_ref().unwrap_err() {
            LLMError::StreamError(msg) => assert!(msg.contains("exceeded")),
            other => panic!("expected StreamError, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn incomplete_line_at_end_not_yielded() {
        // No trailing newline — line stays in buffer and is never yielded
        let chunks = vec![Ok(Bytes::from("complete\nincomplete"))];
        let byte_stream = stream::iter(chunks);

        let lines: Vec<_> = safe_line_stream(byte_stream, DEFAULT_MAX_STREAM_BUFFER)
            .collect()
            .await;

        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].as_deref().unwrap(), "complete");
    }

    // ── SafeStreamProvider tests ─────────────────────────────────────────

    /// Mock provider that yields a fixed sequence of stream results.
    /// Uses Mutex<Option<Vec<...>>> so items can be taken without Clone.
    struct MockStreamProvider {
        items: Mutex<Option<Vec<LLMResult<StreamChunk>>>>,
    }

    impl MockStreamProvider {
        fn new(items: Vec<LLMResult<StreamChunk>>) -> Self {
            Self {
                items: Mutex::new(Some(items)),
            }
        }
    }

    #[async_trait::async_trait]
    impl LLMProvider for MockStreamProvider {
        async fn send_message(&self, _request: LLMRequest) -> LLMResult<LLMResponse> {
            Ok(LLMResponse {
                content: String::new(),
                content_blocks: vec![],
                model: "mock".to_string(),
                stop_reason: crate::llm::StopReason::EndTurn,
                usage: TokenUsage {
                    input_tokens: 0,
                    output_tokens: 0,
                    ..Default::default()
                },
            })
        }

        async fn send_message_stream(
            &self,
            _request: LLMRequest,
        ) -> LLMResult<Pin<Box<dyn Stream<Item = LLMResult<StreamChunk>> + Send>>> {
            let items = self.items.lock().unwrap().take().unwrap_or_default();
            let stream = stream::iter(items);
            Ok(Box::pin(stream))
        }

        fn provider_name(&self) -> &'static str {
            "mock"
        }

        fn model_id(&self) -> &str {
            "mock-model"
        }
    }

    fn ok_delta(text: &str) -> LLMResult<StreamChunk> {
        Ok(StreamChunk::ContentDelta {
            text: text.to_string(),
            index: 0,
        })
    }

    fn stream_error(msg: &str) -> LLMResult<StreamChunk> {
        Err(LLMError::StreamError(msg.to_string()))
    }

    #[tokio::test]
    async fn safe_stream_passes_through_all_ok_items() {
        let provider = SafeStreamProvider::new(MockStreamProvider::new(vec![
            ok_delta("a"),
            ok_delta("b"),
            ok_delta("c"),
        ]));
        let request = LLMRequest::new("mock", vec![Message::user("hi")]);

        let stream = provider.send_message_stream(request).await.unwrap();
        let results: Vec<_> = stream.collect().await;

        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|r| r.is_ok()));
    }

    #[tokio::test]
    async fn safe_stream_stops_after_first_error() {
        let provider = SafeStreamProvider::new(MockStreamProvider::new(vec![
            ok_delta("a"),
            ok_delta("b"),
            stream_error("boom"),
            ok_delta("c"), // should NOT be yielded
            ok_delta("d"), // should NOT be yielded
        ]));
        let request = LLMRequest::new("mock", vec![Message::user("hi")]);

        let stream = provider.send_message_stream(request).await.unwrap();
        let results: Vec<_> = stream.collect().await;

        assert_eq!(results.len(), 3); // a, b, error
        assert!(results[0].is_ok());
        assert!(results[1].is_ok());
        assert!(results[2].is_err());
    }

    #[tokio::test]
    async fn safe_stream_stops_on_immediate_error() {
        let provider = SafeStreamProvider::new(MockStreamProvider::new(vec![
            stream_error("fail"),
            ok_delta("never"),
        ]));
        let request = LLMRequest::new("mock", vec![Message::user("hi")]);

        let stream = provider.send_message_stream(request).await.unwrap();
        let results: Vec<_> = stream.collect().await;

        assert_eq!(results.len(), 1);
        assert!(results[0].is_err());
    }

    #[tokio::test]
    async fn safe_stream_delegates_provider_name() {
        let provider = SafeStreamProvider::new(MockStreamProvider::new(vec![]));
        assert_eq!(provider.provider_name(), "mock");
        assert_eq!(provider.model_id(), "mock-model");
    }

    #[tokio::test]
    async fn safe_stream_send_message_passthrough() {
        let provider = SafeStreamProvider::new(MockStreamProvider::new(vec![]));
        let request = LLMRequest::new("mock", vec![Message::user("hi")]);

        let result = provider.send_message(request).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().model, "mock");
    }
}
