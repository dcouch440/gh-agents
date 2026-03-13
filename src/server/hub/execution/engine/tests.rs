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
    }

    impl TestStrategy {
        fn new() -> Self {
            Self {
                system: "You are helpful.".into(),
                model: "test-model".into(),
            }
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
            false
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
                content: "Hello!".into(),
                content_blocks: vec![ContentBlock::Text {
                    text: "Hello!".into(),
                }],
                model: "test-model".into(),
                stop_reason: StopReason::EndTurn,
                usage: TokenUsage {
                    input_tokens: 10,
                    output_tokens: 5,
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
                        },
                    })
                } else {
                    Ok(LLMResponse {
                        content: "Done!".into(),
                        content_blocks: vec![ContentBlock::Text {
                            text: "Done!".into(),
                        }],
                        model: "m".into(),
                        stop_reason: StopReason::EndTurn,
                        usage: TokenUsage {
                            input_tokens: 20,
                            output_tokens: 10,
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
    async fn execute_max_rounds_exhausted() {
        // Provider that always returns tool_use
        struct AlwaysToolUse;

        #[async_trait]
        impl LLMProvider for AlwaysToolUse {
            async fn send_message(&self, _req: LLMRequest) -> Result<LLMResponse, LLMError> {
                Ok(LLMResponse {
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
                content: "callback test".into(),
                content_blocks: vec![ContentBlock::Text {
                    text: "callback test".into(),
                }],
                model: "m".into(),
                stop_reason: StopReason::EndTurn,
                usage: TokenUsage {
                    input_tokens: 10,
                    output_tokens: 5,
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
                content: "partial response".into(),
                content_blocks: vec![ContentBlock::Text {
                    text: "partial response".into(),
                }],
                model: "m".into(),
                stop_reason: StopReason::MaxTokens,
                usage: TokenUsage {
                    input_tokens: 100,
                    output_tokens: 4096,
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
                        },
                    })
                } else {
                    Ok(LLMResponse {
                        content: "All done".into(),
                        content_blocks: vec![ContentBlock::Text {
                            text: "All done".into(),
                        }],
                        model: "m".into(),
                        stop_reason: StopReason::EndTurn,
                        usage: TokenUsage {
                            input_tokens: 80,
                            output_tokens: 20,
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
}
