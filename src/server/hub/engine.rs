//! ExecutionEngine — the single LLM execution loop for the entire application.
//!
//! Every chat turn, DAG step, and router call flows through
//! `ExecutionEngine::execute()`. The loop is parameterized by an
//! `ExecutionStrategy` that controls prompts, tools, and post-processing.

use std::sync::Arc;

use futures::StreamExt;
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};

use crate::llm::{ContentBlock, LLMProvider, LLMRequest, Message, StopReason, StreamAccumulator, StreamChunk as LLMStreamChunk, TokenUsage};

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
        cancel: Option<&CancellationToken>,
    ) -> Result<ExecutionResult, HubError> {
        let mut messages = strategy.build_messages(input).await?;
        let max_rounds = strategy.max_rounds();
        let budget = strategy.context_budget();
        let mut total_input: u64 = 0;
        let mut total_output: u64 = 0;

        for round in 0..max_rounds {
            // Check cancellation
            if cancel.is_some_and(|t| t.is_cancelled()) {
                return Err(HubError::Cancelled);
            }

            // Check context budget
            let char_count: usize = messages.iter().map(|m| m.estimated_chars()).sum();
            if char_count > budget {
                return Err(HubError::ContextBudgetExceeded { chars: char_count, round });
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
                let stream = self.provider.send_message_stream(request).await.map_err(|e| HubError::LlmCallFailed { round, source: e })?;

                let mut accumulator = StreamAccumulator::new();
                let mut pinned = std::pin::pin!(stream);

                loop {
                    let chunk_result = if let Some(ct) = cancel {
                        tokio::select! {
                            biased;
                            _ = ct.cancelled() => {
                                return Err(HubError::Cancelled);
                            }
                            next = pinned.next() => next,
                        }
                    } else {
                        pinned.next().await
                    };

                    match chunk_result {
                        Some(Ok(chunk)) => {
                            // Forward text tokens to sink
                            if let LLMStreamChunk::ContentDelta { ref text, .. } = chunk {
                                sink.token(text).await;
                            }
                            if let LLMStreamChunk::ToolUseStart { ref name, ref id, .. } = chunk {
                                sink.tool_start(name, id).await;
                            }
                            accumulator.apply(&chunk);
                        }
                        Some(Err(e)) => {
                            let msg = format!("stream error at round {}: {}", round, e);
                            sink.error(&msg).await;
                            return Err(HubError::LlmCallFailed { round, source: e });
                        }
                        None => break,
                    }
                }

                accumulator.build().ok_or_else(|| HubError::Internal(anyhow::anyhow!("incomplete stream at round {}", round)))?
            } else if let Some(ct) = cancel {
                tokio::select! {
                    biased;
                    _ = ct.cancelled() => {
                        return Err(HubError::Cancelled);
                    }
                    result = self.provider.send_message(request) => {
                        result.map_err(|e| HubError::LlmCallFailed { round, source: e })?
                    }
                }
            } else {
                self.provider.send_message(request).await.map_err(|e| HubError::LlmCallFailed { round, source: e })?
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
                            ContentBlock::ToolUse { id, name, input } => Some((id.clone(), name.clone(), input.clone())),
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
                        if cancel.is_some_and(|t| t.is_cancelled()) {
                            return Err(HubError::Cancelled);
                        }
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
mod tests;
