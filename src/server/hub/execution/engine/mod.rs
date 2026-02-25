//! ExecutionEngine — the single LLM execution loop for the entire application.
//!
//! Every chat turn, DAG step, and router call flows through
//! `ExecutionEngine::execute()`. The loop is parameterized by an
//! `ExecutionStrategy` that controls prompts, tools, and post-processing.

use std::sync::Arc;

use futures::StreamExt;
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};

use crate::llm::{
    ContentBlock, LLMProvider, LLMRequest, LLMResponse, Message, StopReason, StreamAccumulator,
    StreamChunk as LLMStreamChunk, TokenUsage,
};

use super::recorder::ExecutionRecorder;
use super::strategy::ExecutionStrategy;
use super::streaming::StreamSink;
use crate::server::hub::error::HubError;

pub mod filters;

use filters::{ExecutionFilter, FilterContext, ResponseAction};

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
    filters: Vec<Arc<dyn ExecutionFilter>>,
    filter_ctx: Option<FilterContext>,
    debug_stream: bool,
}

impl ExecutionEngine {
    pub fn new(provider: Arc<dyn LLMProvider>, debug_stream: bool) -> Self {
        Self {
            provider,
            filters: Vec::new(),
            filter_ctx: None,
            debug_stream,
        }
    }

    /// Attach execution filters to the engine.
    pub fn with_filters(mut self, filters: Vec<Arc<dyn ExecutionFilter>>) -> Self {
        self.filters = filters;
        self
    }

    /// Set the filter context for this execution.
    pub fn with_filter_context(mut self, ctx: FilterContext) -> Self {
        self.filter_ctx = Some(ctx);
        self
    }

    /// Create a new `ExecutionEngine` sharing the same LLM provider.
    /// Useful for spawning parallel subtask executions.
    pub fn clone_with_provider(&self) -> Self {
        Self {
            provider: Arc::clone(&self.provider),
            filters: self.filters.clone(),
            filter_ctx: self.filter_ctx.clone(),
            debug_stream: self.debug_stream,
        }
    }

    /// Get a clone of the underlying LLM provider.
    /// Used by room execution which needs `Arc<dyn LLMProvider>` directly.
    pub fn provider(&self) -> Arc<dyn LLMProvider> {
        Arc::clone(&self.provider)
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

        // ── on_start filters ──
        let mut system_prompt = if let Some(ref filter_ctx) = self.filter_ctx {
            let mut sys = strategy.system_prompt().to_string();
            let mut msgs = messages;
            for f in &self.filters {
                (sys, msgs) = f.on_start(filter_ctx, sys, msgs).await?;
            }
            messages = msgs;
            sys
        } else {
            strategy.system_prompt().to_string()
        };

        // ── Debug: emit system prompt + user message ──
        if self.debug_stream {
            if let Some(ae_id) = strategy.agent_execution_id() {
                sink.debug_system_prompt(ae_id, &system_prompt).await;
                sink.debug_user_message(ae_id, input).await;
            }
        }

        // Track filter retries (max 1 retry per filter per execution)
        let mut filter_retried = vec![false; self.filters.len()];

        for round in 0..max_rounds {
            // Check cancellation
            if cancel.is_some_and(|t| t.is_cancelled()) {
                return Err(HubError::Cancelled);
            }

            // Refresh system prompt if the strategy needs it (e.g. after tool mutations)
            if round > 0 {
                if let Some(fresh) = strategy.rebuild_system_prompt().await? {
                    system_prompt = fresh;
                }
            }

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
                .with_system(&system_prompt)
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
                            // Note: tool_start is NOT emitted here. The execution loop
                            // (below) sends tool_start/tool_end when the tool actually
                            // runs, avoiding duplicate events on the frontend.
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

                accumulator.build().ok_or_else(|| {
                    HubError::Internal(anyhow::anyhow!("incomplete stream at round {}", round))
                })?
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
                    let has_tool_blocks = response
                        .content_blocks
                        .iter()
                        .any(|b| matches!(b, ContentBlock::ToolUse { .. }));

                    if !has_tool_blocks {
                        warn!(
                            "StopReason::ToolUse but no tool_use blocks at round {}",
                            round
                        );
                        break;
                    }

                    if let Some(result) = self
                        .handle_tool_use_round(
                            strategy,
                            sink,
                            recorder,
                            cancel,
                            round,
                            &response,
                            &mut messages,
                            total_input,
                            total_output,
                        )
                        .await?
                    {
                        return Ok(result);
                    }
                }
                StopReason::EndTurn | StopReason::MaxTokens | StopReason::StopSequence => {
                    if let Some(result) = self
                        .handle_end_turn(
                            strategy,
                            sink,
                            recorder,
                            round,
                            &response,
                            &mut messages,
                            total_input,
                            total_output,
                            &mut filter_retried,
                        )
                        .await?
                    {
                        return Ok(result);
                    }
                    // None means a filter requested retry — continue the loop
                }
            }
        }

        Err(HubError::MaxRoundsExhausted { max: max_rounds })
    }

    /// Execute all tool calls from a tool-use round, record them, and check
    /// whether the strategy wants to stop early.
    ///
    /// Returns `Some(ExecutionResult)` if the strategy signalled stop (e.g.
    /// `complete_task` was called), or `None` to continue the loop.
    async fn handle_tool_use_round(
        &self,
        strategy: &dyn ExecutionStrategy,
        sink: &dyn StreamSink,
        recorder: &ExecutionRecorder<'_>,
        cancel: Option<&CancellationToken>,
        round: u32,
        response: &LLMResponse,
        messages: &mut Vec<Message>,
        total_input: u64,
        total_output: u64,
    ) -> Result<Option<ExecutionResult>, HubError> {
        // Extract tool use blocks (caller already verified at least one exists)
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

        // Append assistant message with all blocks
        messages.push(Message::assistant_with_blocks(
            response.content_blocks.clone(),
        ));

        // Execute each tool and build result blocks
        let mut result_blocks = Vec::new();
        for (tool_id, tool_name, tool_input) in &tool_uses {
            if cancel.is_some_and(|t| t.is_cancelled()) {
                return Err(HubError::Cancelled);
            }
            debug!(round, tool = %tool_name, "executing tool");
            sink.tool_start(tool_name, tool_id, tool_input).await;
            let result = strategy.execute_tool(tool_name, tool_input).await;
            sink.tool_end(tool_name, tool_id, &result).await;

            let result_str = match &result {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            result_blocks.push(ContentBlock::ToolResult {
                tool_use_id: tool_id.clone(),
                content: result_str,
            });
        }

        messages.push(Message::tool_results(result_blocks.clone()));

        // Persist assistant response (tool calls) + tool results
        if let Some(ae_id) = strategy.agent_execution_id() {
            let assistant_content = response
                .content_blocks
                .iter()
                .filter_map(|b| match b {
                    ContentBlock::ToolUse { name, input, .. } => {
                        Some(format!("tool_use: {} {}", name, input))
                    }
                    ContentBlock::Text { text } => Some(text.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");
            let _ = recorder
                .record_execution_message(
                    ae_id,
                    "assistant",
                    &assistant_content,
                    None,
                    response.usage.input_tokens as i64,
                    response.usage.output_tokens as i64,
                )
                .await;

            // Debug: emit tool calls with full input payloads
            if self.debug_stream {
                for (tool_id, tool_name, tool_input) in &tool_uses {
                    sink.debug_tool_call(ae_id, tool_name, tool_id, tool_input)
                        .await;
                }
            }

            for block in &result_blocks {
                if let ContentBlock::ToolResult {
                    tool_use_id,
                    content,
                } = block
                {
                    let _ = recorder
                        .record_execution_message(
                            ae_id,
                            "tool",
                            content,
                            Some(tool_use_id.clone()),
                            0,
                            0,
                        )
                        .await;

                    // Debug: emit tool result with full content
                    if self.debug_stream {
                        sink.debug_tool_result(ae_id, "", tool_use_id, content)
                            .await;
                    }
                }
            }
        }

        // Check if strategy wants to stop (e.g. complete_task was called)
        if strategy.should_stop() {
            let usage = TokenUsage {
                input_tokens: total_input as u32,
                output_tokens: total_output as u32,
            };
            strategy.on_complete("", &usage).await?;
            sink.done().await;
            return Ok(Some(ExecutionResult {
                content: String::new(),
                content_blocks: response.content_blocks.clone(),
                input_tokens: total_input,
                output_tokens: total_output,
                cost_usd: 0.0,
                rounds_used: round + 1,
            }));
        }

        Ok(None)
    }

    /// Run response/output filters, persist the final assistant message,
    /// and call the strategy's `on_complete` callback.
    ///
    /// Returns `Some(ExecutionResult)` when the turn is complete, or `None`
    /// if a filter requested a retry (the caller should `continue` the loop).
    async fn handle_end_turn(
        &self,
        strategy: &dyn ExecutionStrategy,
        sink: &dyn StreamSink,
        recorder: &ExecutionRecorder<'_>,
        round: u32,
        response: &LLMResponse,
        messages: &mut Vec<Message>,
        total_input: u64,
        total_output: u64,
        filter_retried: &mut [bool],
    ) -> Result<Option<ExecutionResult>, HubError> {
        // ── on_response filters ──
        if let Some(ref filter_ctx) = self.filter_ctx {
            for (i, f) in self.filters.iter().enumerate() {
                if filter_retried[i] {
                    continue;
                }
                let mut ctx = filter_ctx.clone();
                ctx.round = round;
                match f.on_response(&ctx, response).await? {
                    ResponseAction::Retry { feedback } => {
                        debug!(filter = f.name(), round, "filter requested retry");
                        messages.push(Message::assistant(&response.content));
                        messages.push(Message::user(&feedback));
                        filter_retried[i] = true;
                        return Ok(None);
                    }
                    ResponseAction::Accept => {}
                }
            }
        }

        // ── on_output filters ──
        let mut final_content = response.content.clone();
        if let Some(ref filter_ctx) = self.filter_ctx {
            let mut ctx = filter_ctx.clone();
            ctx.round = round;
            for f in &self.filters {
                final_content = f.on_output(&ctx, final_content).await?;
            }
        }

        // Persist final assistant response
        if let Some(ae_id) = strategy.agent_execution_id() {
            let _ = recorder
                .record_execution_message(
                    ae_id,
                    "assistant",
                    &final_content,
                    None,
                    response.usage.input_tokens as i64,
                    response.usage.output_tokens as i64,
                )
                .await;

            // Debug: emit complete assistant response
            if self.debug_stream {
                sink.debug_assistant_message(ae_id, &final_content).await;
            }
        }

        // Execution complete
        let usage = TokenUsage {
            input_tokens: total_input as u32,
            output_tokens: total_output as u32,
        };

        // Let strategy do post-processing
        strategy.on_complete(&final_content, &usage).await?;

        sink.done().await;

        Ok(Some(ExecutionResult {
            content: final_content,
            content_blocks: response.content_blocks.clone(),
            input_tokens: total_input,
            output_tokens: total_output,
            cost_usd: 0.0, // Strategies compute cost in on_complete
            rounds_used: round + 1,
        }))
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
