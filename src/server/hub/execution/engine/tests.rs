#[cfg(test)]
mod tests {
    //! Tests for execution engine

    use crate::llm::{
        ContentBlock, LLMError, LLMProvider, LLMRequest, LLMResponse, Message, StopReason,
        StreamChunk, TokenUsage,
    };
    use crate::server::hub::{
        engine::ExecutionEngine, error::HubError, recorder::ExecutionRecorder,
        strategy::ExecutionStrategy, streaming::NullSink,
    };
    use async_trait::async_trait;
    use futures::Stream;
    use std::pin::Pin;
    use std::sync::Arc;
    use tokio_util::sync::CancellationToken;

    /// Mock LLM that returns a fixed response.
    struct FixedProvider {
        response: LLMResponse,
    }

    #[async_trait]
    impl LLMProvider for FixedProvider {
        async fn send_message(&self, _req: LLMRequest) -> Result<LLMResponse, LLMError> {
            Ok(self.response.clone())
        }
        async fn send_message_stream(
            &self,
            _req: LLMRequest,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, LLMError>> + Send>>, LLMError>
        {
            Err(LLMError::StreamError("not implemented".into()))
        }
        fn provider_name(&self) -> &'static str {
            "fixed"
        }
        fn model_id(&self) -> &str {
            "test-model"
        }
    }

    /// Minimal strategy for testing.
    struct TestStrategy {
        system: String,
        model: String,
        streaming: bool,
    }

    impl TestStrategy {
        fn new() -> Self {
            Self {
                system: "You are helpful.".into(),
                model: "test-model".into(),
                streaming: false,
            }
        }

        fn with_streaming(mut self) -> Self {
            self.streaming = true;
            self
        }
    }

    #[async_trait]
    impl ExecutionStrategy for TestStrategy {
        fn system_prompt(&self) -> &str {
            &self.system
        }
        fn tools(&self) -> Vec<crate::llm::Tool> {
            vec![]
        }
        fn model_id(&self) -> &str {
            &self.model
        }
        fn max_rounds(&self) -> u32 {
            10
        }
        fn context_budget(&self) -> usize {
            480_000
        }
        fn streaming(&self) -> bool {
            self.streaming
        }
        fn temperature(&self) -> f32 {
            0.7
        }
        async fn build_messages(&self, input: &str) -> Result<Vec<Message>, HubError> {
            Ok(vec![Message::user(input)])
        }
        async fn execute_tool(&self, _name: &str, _input: &serde_json::Value) -> serde_json::Value {
            serde_json::json!({"error": "no tools"})
        }
    }

    fn make_mock_recorder() -> ExecutionRecorder<'static> {
        // We leak the mocks to get 'static references for testing.
        // This is fine in tests.
        let session_mock = Box::leak(Box::new(crate::db::traits::MockSessionRepo::new()));
        let chat_mock = Box::leak(Box::new(crate::db::traits::MockChatMessageRepo::new()));
        ExecutionRecorder::new(session_mock, chat_mock, None, None)
    }

    #[tokio::test]
    async fn execute_simple_response() {
        let provider = Arc::new(FixedProvider {
            response: LLMResponse {
                reasoning: None,
                content: "Hello!".into(),
                content_blocks: vec![ContentBlock::Text {
                    text: "Hello!".into(),
                }],
                model: "test-model".into(),
                stop_reason: StopReason::EndTurn,
                usage: TokenUsage {
                    input_tokens: 10,
                    output_tokens: 5,
                    ..Default::default()
                },
            },
        });

        let engine = ExecutionEngine::new(provider, false);
        let strategy = TestStrategy::new();
        let sink = NullSink;
        let recorder = make_mock_recorder();

        let result = engine
            .execute(&strategy, "Hi", &sink, &recorder, None)
            .await
            .unwrap();
        assert_eq!(result.content, "Hello!");
        assert_eq!(result.input_tokens, 10);
        assert_eq!(result.output_tokens, 5);
        assert_eq!(result.rounds_used, 1);
    }

    #[tokio::test]
    async fn execute_context_budget_exceeded() {
        let provider = Arc::new(FixedProvider {
            response: LLMResponse {
                reasoning: None,
                content: String::new(),
                content_blocks: vec![],
                model: "test-model".into(),
                stop_reason: StopReason::EndTurn,
                usage: TokenUsage::default(),
            },
        });

        let engine = ExecutionEngine::new(provider, false);
        let sink = NullSink;
        let recorder = make_mock_recorder();

        // Strategy with a tiny budget
        struct TinyBudgetStrategy;

        #[async_trait]
        impl ExecutionStrategy for TinyBudgetStrategy {
            fn system_prompt(&self) -> &str {
                "sys"
            }
            fn tools(&self) -> Vec<crate::llm::Tool> {
                vec![]
            }
            fn model_id(&self) -> &str {
                "m"
            }
            fn max_rounds(&self) -> u32 {
                10
            }
            fn context_budget(&self) -> usize {
                1
            } // 1 char budget
            fn streaming(&self) -> bool {
                false
            }
            fn temperature(&self) -> f32 {
                0.7
            }
            async fn build_messages(&self, input: &str) -> Result<Vec<Message>, HubError> {
                Ok(vec![Message::user(input)])
            }
            async fn execute_tool(&self, _: &str, _: &serde_json::Value) -> serde_json::Value {
                serde_json::Value::Null
            }
        }

        let result = engine
            .execute(&TinyBudgetStrategy, "Hello world", &sink, &recorder, None)
            .await;
        assert!(matches!(
            result,
            Err(HubError::ContextBudgetExceeded { .. })
        ));
    }

    #[tokio::test]
    async fn execute_with_tool_use() {
        use std::sync::atomic::{AtomicU32, Ordering};

        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        // Provider that returns tool_use on first call, then end_turn
        struct ToolThenDone {
            calls: Arc<AtomicU32>,
        }

        #[async_trait]
        impl LLMProvider for ToolThenDone {
            async fn send_message(&self, _req: LLMRequest) -> Result<LLMResponse, LLMError> {
                let n = self.calls.fetch_add(1, Ordering::SeqCst);
                if n == 0 {
                    Ok(LLMResponse {
                        reasoning: None,
                        content: String::new(),
                        content_blocks: vec![ContentBlock::ToolUse {
                            id: "t1".into(),
                            name: "search".into(),
                            input: serde_json::json!({"q": "test"}),
                        }],
                        model: "m".into(),
                        stop_reason: StopReason::ToolUse,
                        usage: TokenUsage {
                            input_tokens: 10,
                            output_tokens: 5,
                            ..Default::default()
                        },
                    })
                } else {
                    Ok(LLMResponse {
                        reasoning: None,
                        content: "Done!".into(),
                        content_blocks: vec![ContentBlock::Text {
                            text: "Done!".into(),
                        }],
                        model: "m".into(),
                        stop_reason: StopReason::EndTurn,
                        usage: TokenUsage {
                            input_tokens: 20,
                            output_tokens: 10,
                            ..Default::default()
                        },
                    })
                }
            }
            async fn send_message_stream(
                &self,
                _req: LLMRequest,
            ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, LLMError>> + Send>>, LLMError>
            {
                Err(LLMError::StreamError("not implemented".into()))
            }
            fn provider_name(&self) -> &'static str {
                "tool-test"
            }
            fn model_id(&self) -> &str {
                "m"
            }
        }

        let provider = Arc::new(ToolThenDone {
            calls: call_count_clone,
        });
        let engine = ExecutionEngine::new(provider, false);
        let strategy = TestStrategy::new();
        let sink = NullSink;
        let recorder = make_mock_recorder();

        let result = engine
            .execute(&strategy, "search for test", &sink, &recorder, None)
            .await
            .unwrap();
        assert_eq!(result.content, "Done!");
        assert_eq!(result.rounds_used, 2);
        assert_eq!(result.input_tokens, 30);
        assert_eq!(result.output_tokens, 15);
        assert_eq!(call_count.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn tool_use_reasoning_is_carried_into_next_round_request() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Mutex;

        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();
        let second_request_messages = Arc::new(Mutex::new(None));
        let second_request_messages_clone = second_request_messages.clone();

        // Provider that returns tool_use with reasoning on the first call,
        // captures what it's sent on the second call, then ends the turn.
        struct ToolThenDone {
            calls: Arc<AtomicU32>,
            second_request_messages: Arc<Mutex<Option<Vec<Message>>>>,
        }

        #[async_trait]
        impl LLMProvider for ToolThenDone {
            async fn send_message(&self, req: LLMRequest) -> Result<LLMResponse, LLMError> {
                let n = self.calls.fetch_add(1, Ordering::SeqCst);
                if n == 0 {
                    Ok(LLMResponse {
                        reasoning: Some("I should search for this".into()),
                        content: String::new(),
                        content_blocks: vec![ContentBlock::ToolUse {
                            id: "t1".into(),
                            name: "search".into(),
                            input: serde_json::json!({"q": "test"}),
                        }],
                        model: "m".into(),
                        stop_reason: StopReason::ToolUse,
                        usage: TokenUsage {
                            input_tokens: 10,
                            output_tokens: 5,
                            ..Default::default()
                        },
                    })
                } else {
                    *self.second_request_messages.lock().unwrap() = Some(req.messages.clone());
                    Ok(LLMResponse {
                        reasoning: None,
                        content: "Done!".into(),
                        content_blocks: vec![ContentBlock::Text {
                            text: "Done!".into(),
                        }],
                        model: "m".into(),
                        stop_reason: StopReason::EndTurn,
                        usage: TokenUsage {
                            input_tokens: 20,
                            output_tokens: 10,
                            ..Default::default()
                        },
                    })
                }
            }
            async fn send_message_stream(
                &self,
                _req: LLMRequest,
            ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, LLMError>> + Send>>, LLMError>
            {
                Err(LLMError::StreamError("not implemented".into()))
            }
            fn provider_name(&self) -> &'static str {
                "tool-test"
            }
            fn model_id(&self) -> &str {
                "m"
            }
        }

        let provider = Arc::new(ToolThenDone {
            calls: call_count_clone,
            second_request_messages: second_request_messages_clone,
        });
        let engine = ExecutionEngine::new(provider, false);
        let strategy = TestStrategy::new();
        let sink = NullSink;
        let recorder = make_mock_recorder();

        let result = engine
            .execute(&strategy, "search for test", &sink, &recorder, None)
            .await
            .unwrap();
        assert_eq!(result.content, "Done!");

        let messages = second_request_messages.lock().unwrap().take().unwrap();
        let assistant_msg = messages
            .iter()
            .find(|m| m.role == crate::llm::Role::Assistant)
            .expect("round 2 request should include round 1's assistant turn");
        assert_eq!(
            assistant_msg.reasoning.as_deref(),
            Some("I should search for this")
        );
    }

    #[tokio::test]
    async fn execute_max_rounds_exhausted() {
        // Provider that always returns tool_use
        struct AlwaysToolUse;

        #[async_trait]
        impl LLMProvider for AlwaysToolUse {
            async fn send_message(&self, _req: LLMRequest) -> Result<LLMResponse, LLMError> {
                Ok(LLMResponse {
                    reasoning: None,
                    content: String::new(),
                    content_blocks: vec![ContentBlock::ToolUse {
                        id: "t1".into(),
                        name: "loop_tool".into(),
                        input: serde_json::json!({}),
                    }],
                    model: "m".into(),
                    stop_reason: StopReason::ToolUse,
                    usage: TokenUsage {
                        input_tokens: 5,
                        output_tokens: 5,
                        ..Default::default()
                    },
                })
            }
            async fn send_message_stream(
                &self,
                _req: LLMRequest,
            ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, LLMError>> + Send>>, LLMError>
            {
                Err(LLMError::StreamError("not implemented".into()))
            }
            fn provider_name(&self) -> &'static str {
                "loop"
            }
            fn model_id(&self) -> &str {
                "m"
            }
        }

        // Strategy with max_rounds = 3
        struct LimitedStrategy;

        #[async_trait]
        impl ExecutionStrategy for LimitedStrategy {
            fn system_prompt(&self) -> &str {
                "sys"
            }
            fn tools(&self) -> Vec<crate::llm::Tool> {
                vec![]
            }
            fn model_id(&self) -> &str {
                "m"
            }
            fn max_rounds(&self) -> u32 {
                3
            }
            fn context_budget(&self) -> usize {
                480_000
            }
            fn streaming(&self) -> bool {
                false
            }
            fn temperature(&self) -> f32 {
                0.7
            }
            async fn build_messages(&self, input: &str) -> Result<Vec<Message>, HubError> {
                Ok(vec![Message::user(input)])
            }
            async fn execute_tool(&self, _: &str, _: &serde_json::Value) -> serde_json::Value {
                serde_json::json!({"ok": true})
            }
        }

        let engine = ExecutionEngine::new(Arc::new(AlwaysToolUse), false);
        let sink = NullSink;
        let recorder = make_mock_recorder();

        let result = engine
            .execute(&LimitedStrategy, "go", &sink, &recorder, None)
            .await;
        assert!(matches!(
            result,
            Err(HubError::MaxRoundsExhausted { max: 3 })
        ));
    }

    #[tokio::test]
    async fn execute_on_complete_called() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let completed = Arc::new(AtomicBool::new(false));
        let completed_clone = completed.clone();

        struct CallbackStrategy {
            completed: Arc<AtomicBool>,
        }

        #[async_trait]
        impl ExecutionStrategy for CallbackStrategy {
            fn system_prompt(&self) -> &str {
                "sys"
            }
            fn tools(&self) -> Vec<crate::llm::Tool> {
                vec![]
            }
            fn model_id(&self) -> &str {
                "m"
            }
            fn max_rounds(&self) -> u32 {
                10
            }
            fn context_budget(&self) -> usize {
                480_000
            }
            fn streaming(&self) -> bool {
                false
            }
            fn temperature(&self) -> f32 {
                0.7
            }
            async fn build_messages(&self, input: &str) -> Result<Vec<Message>, HubError> {
                Ok(vec![Message::user(input)])
            }
            async fn execute_tool(&self, _: &str, _: &serde_json::Value) -> serde_json::Value {
                serde_json::Value::Null
            }
            async fn on_complete(
                &self,
                response: &str,
                usage: &TokenUsage,
            ) -> Result<(), HubError> {
                assert_eq!(response, "callback test");
                assert_eq!(usage.input_tokens, 10);
                self.completed.store(true, Ordering::SeqCst);
                Ok(())
            }
        }

        let provider = Arc::new(FixedProvider {
            response: LLMResponse {
                reasoning: None,
                content: "callback test".into(),
                content_blocks: vec![ContentBlock::Text {
                    text: "callback test".into(),
                }],
                model: "m".into(),
                stop_reason: StopReason::EndTurn,
                usage: TokenUsage {
                    input_tokens: 10,
                    output_tokens: 5,
                    ..Default::default()
                },
            },
        });

        let engine = ExecutionEngine::new(provider, false);
        let strategy = CallbackStrategy {
            completed: completed_clone,
        };
        let sink = NullSink;
        let recorder = make_mock_recorder();

        engine
            .execute(&strategy, "test", &sink, &recorder, None)
            .await
            .unwrap();
        assert!(completed.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn execute_max_tokens_stop_reason() {
        let provider = Arc::new(FixedProvider {
            response: LLMResponse {
                reasoning: None,
                content: "partial response".into(),
                content_blocks: vec![ContentBlock::Text {
                    text: "partial response".into(),
                }],
                model: "m".into(),
                stop_reason: StopReason::MaxTokens,
                usage: TokenUsage {
                    input_tokens: 100,
                    output_tokens: 4096,
                    ..Default::default()
                },
            },
        });

        let engine = ExecutionEngine::new(provider, false);
        let strategy = TestStrategy::new();
        let sink = NullSink;
        let recorder = make_mock_recorder();

        let result = engine
            .execute(&strategy, "long question", &sink, &recorder, None)
            .await
            .unwrap();
        assert_eq!(result.content, "partial response");
        assert_eq!(result.rounds_used, 1);
        assert_eq!(result.output_tokens, 4096);
    }

    #[tokio::test]
    async fn execute_multiple_tools_single_round() {
        use std::sync::atomic::{AtomicU32, Ordering};

        let call_count = Arc::new(AtomicU32::new(0));
        let tool_exec_count = Arc::new(AtomicU32::new(0));
        let tool_exec_clone = tool_exec_count.clone();

        struct MultiToolProvider {
            calls: Arc<AtomicU32>,
        }

        #[async_trait]
        impl LLMProvider for MultiToolProvider {
            async fn send_message(&self, _req: LLMRequest) -> Result<LLMResponse, LLMError> {
                let n = self.calls.fetch_add(1, Ordering::SeqCst);
                if n == 0 {
                    Ok(LLMResponse {
                        reasoning: None,
                        content: String::new(),
                        content_blocks: vec![
                            ContentBlock::ToolUse {
                                id: "t1".into(),
                                name: "search".into(),
                                input: serde_json::json!({"q": "a"}),
                            },
                            ContentBlock::ToolUse {
                                id: "t2".into(),
                                name: "read".into(),
                                input: serde_json::json!({"file": "b"}),
                            },
                            ContentBlock::ToolUse {
                                id: "t3".into(),
                                name: "write".into(),
                                input: serde_json::json!({"data": "c"}),
                            },
                        ],
                        model: "m".into(),
                        stop_reason: StopReason::ToolUse,
                        usage: TokenUsage {
                            input_tokens: 50,
                            output_tokens: 30,
                            ..Default::default()
                        },
                    })
                } else {
                    Ok(LLMResponse {
                        reasoning: None,
                        content: "All done".into(),
                        content_blocks: vec![ContentBlock::Text {
                            text: "All done".into(),
                        }],
                        model: "m".into(),
                        stop_reason: StopReason::EndTurn,
                        usage: TokenUsage {
                            input_tokens: 80,
                            output_tokens: 20,
                            ..Default::default()
                        },
                    })
                }
            }
            async fn send_message_stream(
                &self,
                _req: LLMRequest,
            ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, LLMError>> + Send>>, LLMError>
            {
                Err(LLMError::StreamError("not implemented".into()))
            }
            fn provider_name(&self) -> &'static str {
                "multi-tool"
            }
            fn model_id(&self) -> &str {
                "m"
            }
        }

        struct ToolCountingStrategy {
            count: Arc<AtomicU32>,
        }

        #[async_trait]
        impl ExecutionStrategy for ToolCountingStrategy {
            fn system_prompt(&self) -> &str {
                "sys"
            }
            fn tools(&self) -> Vec<crate::llm::Tool> {
                vec![]
            }
            fn model_id(&self) -> &str {
                "m"
            }
            fn max_rounds(&self) -> u32 {
                10
            }
            fn context_budget(&self) -> usize {
                480_000
            }
            fn streaming(&self) -> bool {
                false
            }
            fn temperature(&self) -> f32 {
                0.7
            }
            async fn build_messages(&self, input: &str) -> Result<Vec<Message>, HubError> {
                Ok(vec![Message::user(input)])
            }
            async fn execute_tool(&self, _: &str, _: &serde_json::Value) -> serde_json::Value {
                self.count.fetch_add(1, Ordering::SeqCst);
                serde_json::json!({"ok": true})
            }
        }

        let provider = Arc::new(MultiToolProvider { calls: call_count });
        let engine = ExecutionEngine::new(provider, false);
        let strategy = ToolCountingStrategy {
            count: tool_exec_clone,
        };
        let sink = NullSink;
        let recorder = make_mock_recorder();

        let result = engine
            .execute(&strategy, "do things", &sink, &recorder, None)
            .await
            .unwrap();
        assert_eq!(result.content, "All done");
        assert_eq!(result.rounds_used, 2);
        assert_eq!(result.input_tokens, 130); // 50 + 80
        assert_eq!(result.output_tokens, 50); // 30 + 20
        assert_eq!(tool_exec_count.load(Ordering::SeqCst), 3); // 3 tools executed
    }

    #[test]
    fn display_cancelled() {
        let err = HubError::Cancelled;
        assert_eq!(err.to_string(), "execution cancelled");
    }

    #[tokio::test]
    async fn execute_cancelled_before_start() {
        let provider = Arc::new(FixedProvider {
            response: LLMResponse {
                reasoning: None,
                content: "should not reach".into(),
                content_blocks: vec![],
                model: "m".into(),
                stop_reason: StopReason::EndTurn,
                usage: TokenUsage::default(),
            },
        });

        let engine = ExecutionEngine::new(provider, false);
        let strategy = TestStrategy::new();
        let sink = NullSink;
        let recorder = make_mock_recorder();

        let token = CancellationToken::new();
        token.cancel();

        let result = engine
            .execute(&strategy, "Hi", &sink, &recorder, Some(&token))
            .await;
        assert!(matches!(result, Err(HubError::Cancelled)));
    }

    #[tokio::test]
    async fn execute_cancelled_between_tool_rounds() {
        use std::sync::atomic::{AtomicU32, Ordering};

        let call_count = Arc::new(AtomicU32::new(0));
        let token = CancellationToken::new();
        let token_for_strategy = token.clone();

        struct CancelAfterToolProvider;

        #[async_trait]
        impl LLMProvider for CancelAfterToolProvider {
            async fn send_message(&self, _req: LLMRequest) -> Result<LLMResponse, LLMError> {
                Ok(LLMResponse {
                    reasoning: None,
                    content: String::new(),
                    content_blocks: vec![ContentBlock::ToolUse {
                        id: "t1".into(),
                        name: "do_thing".into(),
                        input: serde_json::json!({}),
                    }],
                    model: "m".into(),
                    stop_reason: StopReason::ToolUse,
                    usage: TokenUsage {
                        input_tokens: 5,
                        output_tokens: 5,
                        ..Default::default()
                    },
                })
            }
            async fn send_message_stream(
                &self,
                _req: LLMRequest,
            ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, LLMError>> + Send>>, LLMError>
            {
                Err(LLMError::StreamError("not implemented".into()))
            }
            fn provider_name(&self) -> &'static str {
                "cancel-test"
            }
            fn model_id(&self) -> &str {
                "m"
            }
        }

        // Strategy that cancels the token after first tool execution
        struct CancellingStrategy {
            token: CancellationToken,
            calls: Arc<AtomicU32>,
        }

        #[async_trait]
        impl ExecutionStrategy for CancellingStrategy {
            fn system_prompt(&self) -> &str {
                "sys"
            }
            fn tools(&self) -> Vec<crate::llm::Tool> {
                vec![]
            }
            fn model_id(&self) -> &str {
                "m"
            }
            fn max_rounds(&self) -> u32 {
                10
            }
            fn context_budget(&self) -> usize {
                480_000
            }
            fn streaming(&self) -> bool {
                false
            }
            fn temperature(&self) -> f32 {
                0.7
            }
            async fn build_messages(&self, input: &str) -> Result<Vec<Message>, HubError> {
                Ok(vec![Message::user(input)])
            }
            async fn execute_tool(&self, _: &str, _: &serde_json::Value) -> serde_json::Value {
                self.calls.fetch_add(1, Ordering::SeqCst);
                // Cancel after executing first tool
                self.token.cancel();
                serde_json::json!({"ok": true})
            }
        }

        let provider = Arc::new(CancelAfterToolProvider);
        let engine = ExecutionEngine::new(provider, false);
        let strategy = CancellingStrategy {
            token: token_for_strategy,
            calls: call_count.clone(),
        };
        let sink = NullSink;
        let recorder = make_mock_recorder();

        let result = engine
            .execute(&strategy, "go", &sink, &recorder, Some(&token))
            .await;
        assert!(matches!(result, Err(HubError::Cancelled)));
    }

    // ── Mid-stream transport failure ────────────────────────────────────

    /// Produce a genuine `reqwest::Error` without touching the network — port 1
    /// on loopback is never listening, so the connect attempt always fails.
    async fn connect_error() -> reqwest::Error {
        reqwest::Client::new()
            .get("http://127.0.0.1:1/")
            .send()
            .await
            .expect_err("connect to 127.0.0.1:1 must fail")
    }

    fn ok_chunks(text: &str) -> Vec<Result<StreamChunk, LLMError>> {
        vec![
            Ok(StreamChunk::MessageStart {
                model: "test-model".into(),
                input_tokens: 1,
            }),
            Ok(StreamChunk::ContentDelta {
                text: text.into(),
                index: 0,
            }),
            Ok(StreamChunk::MessageDelta {
                stop_reason: Some(StopReason::EndTurn),
                output_tokens: Some(1),
            }),
        ]
    }

    /// Fails the stream `fail_times` times before succeeding. Records how many
    /// times a stream was requested so retries are observable.
    struct FlakyStreamProvider {
        fail_times: u32,
        calls: std::sync::atomic::AtomicU32,
        /// Emit a token before failing — makes the failure unsafe to replay.
        emit_before_failing: bool,
    }

    impl FlakyStreamProvider {
        fn new(fail_times: u32, emit_before_failing: bool) -> Self {
            Self {
                fail_times,
                calls: std::sync::atomic::AtomicU32::new(0),
                emit_before_failing,
            }
        }
        fn call_count(&self) -> u32 {
            self.calls.load(std::sync::atomic::Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl LLMProvider for FlakyStreamProvider {
        async fn send_message(&self, _req: LLMRequest) -> Result<LLMResponse, LLMError> {
            Err(LLMError::StreamError("not implemented".into()))
        }
        async fn send_message_stream(
            &self,
            _req: LLMRequest,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, LLMError>> + Send>>, LLMError>
        {
            let n = self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

            if n < self.fail_times {
                let mut chunks: Vec<Result<StreamChunk, LLMError>> =
                    vec![Ok(StreamChunk::MessageStart {
                        model: "test-model".into(),
                        input_tokens: 1,
                    })];
                if self.emit_before_failing {
                    chunks.push(Ok(StreamChunk::ContentDelta {
                        text: "partial".into(),
                        index: 0,
                    }));
                }
                chunks.push(Err(LLMError::StreamTransport(connect_error().await)));
                return Ok(Box::pin(futures::stream::iter(chunks)));
            }

            Ok(Box::pin(futures::stream::iter(ok_chunks("recovered"))))
        }
        fn provider_name(&self) -> &'static str {
            "flaky"
        }
        fn model_id(&self) -> &str {
            "test-model"
        }
    }

    #[tokio::test]
    async fn stream_retried_when_nothing_emitted() {
        let provider = Arc::new(FlakyStreamProvider::new(1, false));
        let engine = ExecutionEngine::new(provider.clone(), false);
        let strategy = TestStrategy::new().with_streaming();
        let recorder = make_mock_recorder();

        let result = engine
            .execute(&strategy, "Hi", &NullSink, &recorder, None)
            .await
            .expect("transient stream failure should be retried");

        assert_eq!(result.content, "recovered");
        assert_eq!(provider.call_count(), 2, "expected exactly one re-issue");
    }

    #[tokio::test]
    async fn stream_not_retried_once_tokens_emitted() {
        // Replaying a round after output has reached the client would duplicate
        // it, so the error must surface instead.
        let provider = Arc::new(FlakyStreamProvider::new(1, true));
        let engine = ExecutionEngine::new(provider.clone(), false);
        let strategy = TestStrategy::new().with_streaming();
        let recorder = make_mock_recorder();

        let err = engine
            .execute(&strategy, "Hi", &NullSink, &recorder, None)
            .await
            .expect_err("must not replay a round that already streamed output");

        assert!(matches!(err, HubError::LlmCallFailed { round: 0, .. }));
        assert_eq!(provider.call_count(), 1, "must not re-issue");
    }

    /// Fails at `send_message_stream` itself — the stream is never established.
    struct FailingEstablishProvider {
        calls: std::sync::atomic::AtomicU32,
    }

    impl FailingEstablishProvider {
        fn new() -> Self {
            Self {
                calls: std::sync::atomic::AtomicU32::new(0),
            }
        }
        fn call_count(&self) -> u32 {
            self.calls.load(std::sync::atomic::Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl LLMProvider for FailingEstablishProvider {
        async fn send_message(&self, _req: LLMRequest) -> Result<LLMResponse, LLMError> {
            Err(LLMError::StreamError("not implemented".into()))
        }
        async fn send_message_stream(
            &self,
            _req: LLMRequest,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, LLMError>> + Send>>, LLMError>
        {
            self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Err(LLMError::StreamTransport(connect_error().await))
        }
        fn provider_name(&self) -> &'static str {
            "failing-establish"
        }
        fn model_id(&self) -> &str {
            "test-model"
        }
    }

    /// The engine's retry loop exists to cover errors yielded *by* an established
    /// stream, which fall outside `RetryingProvider::with_retry`. A failure to
    /// establish is already inside it, so retrying here would multiply the
    /// provider's own attempts rather than cover anything new.
    #[tokio::test]
    async fn establish_failure_is_not_retried_by_the_engine() {
        let provider = Arc::new(FailingEstablishProvider::new());
        let engine = ExecutionEngine::new(provider.clone(), false);
        let strategy = TestStrategy::new().with_streaming();
        let recorder = make_mock_recorder();

        let err = engine
            .execute(&strategy, "Hi", &NullSink, &recorder, None)
            .await
            .expect_err("an unestablishable stream must surface the error");

        assert!(matches!(err, HubError::LlmCallFailed { round: 0, .. }));
        assert_eq!(
            provider.call_count(),
            1,
            "RetryingProvider owns retries for this path — the engine must not add its own"
        );
    }

    #[tokio::test]
    async fn stream_retries_are_bounded() {
        let provider = Arc::new(FlakyStreamProvider::new(u32::MAX, false));
        let engine = ExecutionEngine::new(provider.clone(), false);
        let strategy = TestStrategy::new().with_streaming();
        let recorder = make_mock_recorder();

        let err = engine
            .execute(&strategy, "Hi", &NullSink, &recorder, None)
            .await
            .expect_err("a permanently failing stream must give up");

        assert!(matches!(err, HubError::LlmCallFailed { round: 0, .. }));
        assert_eq!(
            provider.call_count(),
            crate::constants::MAX_STREAM_RETRY_ATTEMPTS + 1,
            "one initial attempt plus MAX_STREAM_RETRY_ATTEMPTS re-issues"
        );
    }
    use std::sync::atomic::{AtomicU32, Ordering};

    // ── Tool calls that carry no arguments ──────────────────────────────────
    //
    // A model can emit a well-formed call with an empty arguments object.
    // Nothing is malformed, so the provider's recovery path never sees it, and
    // dispatching it makes the tool report its first required parameter
    // missing — which is not what went wrong and leaves the model nothing to
    // change. Observed on the system node agent: `run_command {}` two to four
    // rounds running before it recovered on its own.

    /// One tool-use round carrying `input`, then a text round to end the loop.
    struct OneToolCallProvider {
        tool: &'static str,
        input: serde_json::Value,
        calls: Arc<AtomicU32>,
    }

    #[async_trait]
    impl LLMProvider for OneToolCallProvider {
        async fn send_message(&self, _req: LLMRequest) -> Result<LLMResponse, LLMError> {
            let first = self.calls.fetch_add(1, Ordering::SeqCst) == 0;
            Ok(if first {
                LLMResponse {
                    reasoning: None,
                    content: String::new(),
                    content_blocks: vec![ContentBlock::ToolUse {
                        id: "t1".into(),
                        name: self.tool.into(),
                        input: self.input.clone(),
                    }],
                    model: "m".into(),
                    stop_reason: StopReason::ToolUse,
                    usage: TokenUsage::default(),
                }
            } else {
                LLMResponse {
                    reasoning: None,
                    content: "done".into(),
                    content_blocks: vec![ContentBlock::Text {
                        text: "done".into(),
                    }],
                    model: "m".into(),
                    stop_reason: StopReason::EndTurn,
                    usage: TokenUsage::default(),
                }
            })
        }
        async fn send_message_stream(
            &self,
            _req: LLMRequest,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, LLMError>> + Send>>, LLMError>
        {
            Err(LLMError::StreamError("not implemented".into()))
        }
        fn provider_name(&self) -> &'static str {
            "one-tool-call"
        }
        fn model_id(&self) -> &str {
            "m"
        }
    }

    /// Declares the two shapes that matter: a tool with a required parameter
    /// and one without.
    struct SchemaStrategy {
        executed: Arc<AtomicU32>,
    }

    #[async_trait]
    impl ExecutionStrategy for SchemaStrategy {
        fn system_prompt(&self) -> &str {
            "sys"
        }
        fn tools(&self) -> Vec<crate::llm::Tool> {
            vec![
                crate::llm::Tool {
                    name: "run_command".into(),
                    description: "run a command".into(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": { "command": { "type": "string" } },
                        "required": ["command"]
                    }),
                },
                crate::llm::Tool {
                    name: "list_files".into(),
                    description: "list files".into(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": { "path": { "type": "string" } }
                    }),
                },
            ]
        }
        fn model_id(&self) -> &str {
            "m"
        }
        fn max_rounds(&self) -> u32 {
            10
        }
        fn context_budget(&self) -> usize {
            480_000
        }
        fn streaming(&self) -> bool {
            false
        }
        fn temperature(&self) -> f32 {
            0.7
        }
        async fn build_messages(&self, input: &str) -> Result<Vec<Message>, HubError> {
            Ok(vec![Message::user(input)])
        }
        async fn execute_tool(&self, _: &str, _: &serde_json::Value) -> serde_json::Value {
            self.executed.fetch_add(1, Ordering::SeqCst);
            serde_json::json!({"ok": true})
        }
    }

    async fn run_one_call(tool: &'static str, input: serde_json::Value) -> u32 {
        let executed = Arc::new(AtomicU32::new(0));
        let provider = Arc::new(OneToolCallProvider {
            tool,
            input,
            calls: Arc::new(AtomicU32::new(0)),
        });
        let engine = ExecutionEngine::new(provider, false);
        let strategy = SchemaStrategy {
            executed: executed.clone(),
        };
        let recorder = make_mock_recorder();

        engine
            .execute(&strategy, "go", &NullSink, &recorder, None)
            .await
            .expect("the loop must finish");

        executed.load(Ordering::SeqCst)
    }

    #[tokio::test]
    async fn a_call_with_no_arguments_does_not_reach_a_tool_that_requires_some() {
        assert_eq!(
            run_one_call("run_command", serde_json::json!({})).await,
            0,
            "an empty call must be answered by the engine, not dispatched"
        );
    }

    #[tokio::test]
    async fn a_call_with_no_arguments_reaches_a_tool_that_requires_none() {
        assert_eq!(
            run_one_call("list_files", serde_json::json!({})).await,
            1,
            "`list_files {{}}` is a legitimate call"
        );
    }

    #[tokio::test]
    async fn a_call_that_carries_its_required_argument_is_dispatched() {
        assert_eq!(
            run_one_call("run_command", serde_json::json!({"command": "ls"})).await,
            1
        );
    }

    #[test]
    fn the_no_arguments_error_names_the_parameters_and_the_shape() {
        let msg =
            crate::server::hub::engine::no_arguments_error("run_command", &["command".to_string()])
                ["error"]
                .as_str()
                .expect("error is a string")
                .to_string();

        assert!(msg.contains("run_command"), "{msg}");
        assert!(msg.contains("It requires: command"), "{msg}");
        assert!(msg.contains(r#"{"command":"..."}"#), "{msg}");
    }
}
