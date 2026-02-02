//! ExecutionEngine — the single LLM execution loop for the entire application.
//!
//! Every chat turn, DAG step, and router call flows through
//! `ExecutionEngine::execute()`. The loop is parameterized by an
//! `ExecutionStrategy` that controls prompts, tools, and post-processing.

use std::sync::Arc;

use futures::StreamExt;
use tracing::{debug, warn};

use crate::llm::{
    ContentBlock, LLMProvider, LLMRequest, Message, StreamAccumulator,
    StreamChunk as LLMStreamChunk, StopReason, TokenUsage,
};

use super::error::HubError;
use super::recorder::ExecutionRecorder;
use super::strategy::ExecutionStrategy;
use super::streaming::StreamSink;

/// Result of a completed execution.
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// Final text content from the LLM.
    pub content: String,
    /// All content blocks (text + tool use) from the final response.
    pub content_blocks: Vec<ContentBlock>,
    /// Total input tokens across all rounds.
    pub input_tokens: u64,
    /// Total output tokens across all rounds.
    pub output_tokens: u64,
    /// Estimated cost in USD.
    pub cost_usd: f32,
    /// Number of tool-use rounds executed.
    pub rounds_used: u32,
}

/// The unified execution engine.
pub struct ExecutionEngine {
    provider: Arc<dyn LLMProvider>,
}

impl ExecutionEngine {
    pub fn new(provider: Arc<dyn LLMProvider>) -> Self {
        Self { provider }
    }

    /// Run the execution loop.
    ///
    /// 1. Build messages from strategy
    /// 2. Check context budget
    /// 3. Call LLM (streaming or non-streaming)
    /// 4. If tool use → execute tools, append results, loop
    /// 5. On end turn → record, call on_complete, return
    pub async fn execute(
        &self,
        strategy: &dyn ExecutionStrategy,
        input: &str,
        sink: &dyn StreamSink,
        recorder: &ExecutionRecorder<'_>,
    ) -> Result<ExecutionResult, HubError> {
        let mut messages = strategy.build_messages(input).await?;
        let max_rounds = strategy.max_rounds();
        let budget = strategy.context_budget();
        let mut total_input: u64 = 0;
        let mut total_output: u64 = 0;

        for round in 0..max_rounds {
            // Check context budget
            let char_count: usize = messages.iter().map(|m| m.estimated_chars()).sum();
            if char_count > budget {
                return Err(HubError::ContextBudgetExceeded {
                    chars: char_count,
                    round,
                });
            }

            // Build LLM request
            let mut request = LLMRequest::new(strategy.model_id(), messages.clone())
                .with_system(strategy.system_prompt())
                .with_max_tokens(crate::constants::DEFAULT_MAX_TOKENS_WORKER);
            request.temperature = strategy.temperature();
            let tools = strategy.tools();
            if !tools.is_empty() {
                request = request.with_tools(tools);
            }

            // Call LLM
            let response = if strategy.streaming() {
                request = request.with_streaming();
                let stream = self
                    .provider
                    .send_message_stream(request)
                    .await
                    .map_err(|e| HubError::LlmCallFailed { round, source: e })?;

                let mut accumulator = StreamAccumulator::new();
                let mut pinned = std::pin::pin!(stream);

                while let Some(chunk_result) = pinned.next().await {
                    match chunk_result {
                        Ok(chunk) => {
                            // Forward text tokens to sink
                            if let LLMStreamChunk::ContentDelta { ref text, .. } = chunk {
                                sink.token(text).await;
                            }
                            if let LLMStreamChunk::ToolUseStart { ref name, ref id, .. } = chunk {
                                sink.tool_start(name, id).await;
                            }
                            accumulator.apply(&chunk);
                        }
                        Err(e) => {
                            let msg = format!("stream error at round {}: {}", round, e);
                            sink.error(&msg).await;
                            return Err(HubError::LlmCallFailed { round, source: e });
                        }
                    }
                }

                accumulator.build().ok_or_else(|| {
                    HubError::Internal(anyhow::anyhow!("incomplete stream at round {}", round))
                })?
            } else {
                self.provider
                    .send_message(request)
                    .await
                    .map_err(|e| HubError::LlmCallFailed { round, source: e })?
            };

            total_input += response.usage.input_tokens as u64;
            total_output += response.usage.output_tokens as u64;

            // Check stop reason
            match response.stop_reason {
                StopReason::ToolUse => {
                    // Extract tool use blocks, execute them, build results
                    let tool_uses: Vec<_> = response
                        .content_blocks
                        .iter()
                        .filter_map(|b| match b {
                            ContentBlock::ToolUse { id, name, input } => {
                                Some((id.clone(), name.clone(), input.clone()))
                            }
                            _ => None,
                        })
                        .collect();

                    if tool_uses.is_empty() {
                        warn!("StopReason::ToolUse but no tool_use blocks at round {}", round);
                        break;
                    }

                    // Append assistant message with all blocks
                    messages.push(Message::assistant_with_blocks(response.content_blocks.clone()));

                    // Execute each tool and build result blocks
                    let mut result_blocks = Vec::new();
                    for (tool_id, tool_name, tool_input) in &tool_uses {
                        debug!(round, tool = %tool_name, "executing tool");
                        sink.tool_start(tool_name, tool_id).await;
                        let result = strategy.execute_tool(tool_name, tool_input).await;
                        sink.tool_end(tool_name, tool_id).await;

                        let result_str = match &result {
                            serde_json::Value::String(s) => s.clone(),
                            other => other.to_string(),
                        };
                        result_blocks.push(ContentBlock::ToolResult {
                            tool_use_id: tool_id.clone(),
                            content: result_str,
                        });
                    }

                    messages.push(Message::tool_results(result_blocks));
                }
                StopReason::EndTurn | StopReason::MaxTokens | StopReason::StopSequence => {
                    // Execution complete
                    let usage = TokenUsage {
                        input_tokens: total_input as u32,
                        output_tokens: total_output as u32,
                    };

                    // Let strategy do post-processing
                    strategy.on_complete(&response.content, &usage).await?;

                    sink.done().await;

                    // Use _ prefix to acknowledge recorder is available for strategies
                    // that need it — actual recording happens in on_complete callbacks
                    let _ = recorder;

                    return Ok(ExecutionResult {
                        content: response.content,
                        content_blocks: response.content_blocks,
                        input_tokens: total_input,
                        output_tokens: total_output,
                        cost_usd: 0.0, // Strategies compute cost in on_complete
                        rounds_used: round + 1,
                    });
                }
            }
        }

        Err(HubError::MaxRoundsExhausted { max: max_rounds })
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::{LLMError, LLMResponse, Tool};
    use crate::server::hub::streaming::NullSink;
    use async_trait::async_trait;
    use futures::Stream;
    use std::pin::Pin;

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
        ) -> Result<Pin<Box<dyn Stream<Item = Result<LLMStreamChunk, LLMError>> + Send>>, LLMError>
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
        fn tools(&self) -> Vec<Tool> {
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
        async fn on_complete(&self, _response: &str, _usage: &TokenUsage) -> Result<(), HubError> {
            Ok(())
        }
    }

    fn make_mock_recorder() -> (crate::db::traits::MockServerRepo, ExecutionRecorder<'static>) {
        // We leak the mock to get a 'static reference for testing.
        // This is fine in tests.
        let mock = Box::leak(Box::new(crate::db::traits::MockServerRepo::new()));
        let recorder = ExecutionRecorder::new(mock, None, None);
        // Return a separate mock for expectations (unused since recorder has its own ref)
        (crate::db::traits::MockServerRepo::new(), recorder)
    }

    #[tokio::test]
    async fn execute_simple_response() {
        let provider = Arc::new(FixedProvider {
            response: LLMResponse {
                content: "Hello!".into(),
                content_blocks: vec![ContentBlock::Text { text: "Hello!".into() }],
                model: "test-model".into(),
                stop_reason: StopReason::EndTurn,
                usage: TokenUsage {
                    input_tokens: 10,
                    output_tokens: 5,
                },
            },
        });

        let engine = ExecutionEngine::new(provider);
        let strategy = TestStrategy::new();
        let sink = NullSink;
        let (_mock, recorder) = make_mock_recorder();

        let result = engine.execute(&strategy, "Hi", &sink, &recorder).await.unwrap();
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

        let engine = ExecutionEngine::new(provider);
        let sink = NullSink;
        let (_mock, recorder) = make_mock_recorder();

        // Strategy with a tiny budget
        struct TinyBudgetStrategy;

        #[async_trait]
        impl ExecutionStrategy for TinyBudgetStrategy {
            fn system_prompt(&self) -> &str { "sys" }
            fn tools(&self) -> Vec<Tool> { vec![] }
            fn model_id(&self) -> &str { "m" }
            fn max_rounds(&self) -> u32 { 10 }
            fn context_budget(&self) -> usize { 1 } // 1 char budget
            fn streaming(&self) -> bool { false }
            fn temperature(&self) -> f32 { 0.7 }
            async fn build_messages(&self, input: &str) -> Result<Vec<Message>, HubError> {
                Ok(vec![Message::user(input)])
            }
            async fn execute_tool(&self, _: &str, _: &serde_json::Value) -> serde_json::Value {
                serde_json::Value::Null
            }
            async fn on_complete(&self, _: &str, _: &TokenUsage) -> Result<(), HubError> {
                Ok(())
            }
        }

        let result = engine.execute(&TinyBudgetStrategy, "Hello world", &sink, &recorder).await;
        assert!(matches!(result, Err(HubError::ContextBudgetExceeded { .. })));
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
                        usage: TokenUsage { input_tokens: 10, output_tokens: 5 },
                    })
                } else {
                    Ok(LLMResponse {
                        content: "Done!".into(),
                        content_blocks: vec![ContentBlock::Text { text: "Done!".into() }],
                        model: "m".into(),
                        stop_reason: StopReason::EndTurn,
                        usage: TokenUsage { input_tokens: 20, output_tokens: 10 },
                    })
                }
            }
            async fn send_message_stream(
                &self,
                _req: LLMRequest,
            ) -> Result<Pin<Box<dyn Stream<Item = Result<LLMStreamChunk, LLMError>> + Send>>, LLMError>
            {
                Err(LLMError::StreamError("not implemented".into()))
            }
            fn provider_name(&self) -> &'static str { "tool-test" }
            fn model_id(&self) -> &str { "m" }
        }

        let provider = Arc::new(ToolThenDone { calls: call_count_clone });
        let engine = ExecutionEngine::new(provider);
        let strategy = TestStrategy::new();
        let sink = NullSink;
        let (_mock, recorder) = make_mock_recorder();

        let result = engine.execute(&strategy, "search for test", &sink, &recorder).await.unwrap();
        assert_eq!(result.content, "Done!");
        assert_eq!(result.rounds_used, 2);
        assert_eq!(result.input_tokens, 30);
        assert_eq!(result.output_tokens, 15);
        assert_eq!(call_count.load(Ordering::SeqCst), 2);
    }
}
