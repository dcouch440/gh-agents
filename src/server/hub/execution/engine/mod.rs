//! ExecutionEngine — the single LLM execution loop for the entire application.
//!
//! Every chat turn, DAG step, and router call flows through
//! `ExecutionEngine::execute()`. The loop is parameterized by an
//! `ExecutionStrategy` that controls prompts, tools, and post-processing.

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

use futures::StreamExt;
use serde_json::{json, Value};
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};

use crate::llm::{
    BackoffConfig, ContentBlock, ExponentialBackoff, LLMError, LLMProvider, LLMRequest,
    LLMResponse, Message, RetryPolicy, StopReason, StreamAccumulator,
    StreamChunk as LLMStreamChunk, TokenUsage, UNPARSED_ARGUMENTS_KEY,
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
    /// Portion of `input_tokens` served from the provider's prompt cache.
    /// A subset of `input_tokens`, billed at a lower rate.
    pub cached_input_tokens: u64,
    /// Estimated cost in USD.
    pub cost_usd: f32,
    /// Number of tool-use rounds executed.
    pub rounds_used: u32,
}

/// Immutable context shared across all rounds of an execution loop.
struct LoopContext<'a> {
    strategy: &'a dyn ExecutionStrategy,
    sink: &'a dyn StreamSink,
    recorder: &'a ExecutionRecorder<'a>,
    cancel: Option<&'a CancellationToken>,
}

/// The unified execution engine.
pub struct ExecutionEngine {
    provider: Arc<dyn LLMProvider>,
    filters: Vec<Arc<dyn ExecutionFilter>>,
    filter_ctx: Option<FilterContext>,
    debug_stream: bool,
}

/// Maximum consecutive identical tool failures before injecting a hint.
const TOOL_FAILURE_HINT_THRESHOLD: u32 = 3;

/// Maximum premature EndTurn re-prompts before allowing completion.
const MAX_END_TURN_RETRIES: u32 = 2;

/// Check whether a tool result indicates failure.
///
/// Returns `Some(error_summary)` for clear failures, `None` for success or ambiguous results.
fn is_tool_failure(result: &Value) -> Option<String> {
    if result.get("success") == Some(&Value::Bool(false)) {
        let stderr = result["stderr"].as_str().unwrap_or("");
        let snippet = if stderr.len() > 200 {
            &stderr[..200]
        } else {
            stderr
        };
        return Some(format!(
            "exit_code {}, stderr: {}",
            result["exit_code"], snippet
        ));
    }
    if let Some(err) = result.get("error") {
        if !err.is_null() {
            let msg = err.as_str().unwrap_or("unknown error");
            return Some(msg.to_string());
        }
    }
    None
}

/// The raw text of a tool call whose arguments the provider could not parse.
///
/// `None` for a normal call. `Some(raw)` means the model sent something that
/// was neither JSON nor any recoverable dialect, and the provider preserved it
/// under [`UNPARSED_ARGUMENTS_KEY`] rather than discarding it.
fn unparsed_arguments(input: &Value) -> Option<&str> {
    input.get(UNPARSED_ARGUMENTS_KEY)?.as_str()
}

/// The required parameters of `tool_name`, when the call carried no arguments
/// at all.
///
/// A model can emit a syntactically perfect tool call with an empty arguments
/// object — the system node agent does it in bursts, several rounds running.
/// Nothing is malformed, so [`unparsed_arguments`] does not see it, and the
/// tool answers "Missing required parameter: command": true, useless, and
/// indistinguishable from the message it would get for sending the wrong
/// parameter. It has nothing to correct, so it re-sends the same empty call.
///
/// `None` when the call carried arguments, when the tool is not in this
/// strategy's set, or when the tool has no required parameters — `list_files
/// {}` is a legitimate call and must reach the tool.
fn missing_all_arguments(
    strategy: &dyn ExecutionStrategy,
    tool_name: &str,
    input: &Value,
) -> Option<Vec<String>> {
    if !input.as_object().is_some_and(|o| o.is_empty()) {
        return None;
    }
    let tools = strategy.tools();
    let tool = tools.iter().find(|t| t.name == tool_name)?;
    let required: Vec<String> = tool
        .input_schema
        .get("required")?
        .as_array()?
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();
    (!required.is_empty()).then_some(required)
}

/// The corrective answer for a call that arrived with no arguments: name the
/// parameters and show the object shape, so the next attempt has somewhere to
/// go that the previous one did not.
fn no_arguments_error(tool_name: &str, required: &[String]) -> Value {
    let shape: serde_json::Map<String, Value> = required
        .iter()
        .map(|name| (name.clone(), json!("...")))
        .collect();
    json!({
        "error": format!(
            "Called `{tool_name}` with no arguments. It requires: {}. Send the \
             call again with a JSON object holding them, for example {}.",
            required.join(", "),
            Value::Object(shape)
        )
    })
}

/// Hash a tool call (name + serialized input) for deduplication.
fn tool_call_hash(name: &str, input: &Value) -> u64 {
    let mut hasher = DefaultHasher::new();
    name.hash(&mut hasher);
    input.to_string().hash(&mut hasher);
    hasher.finish()
}

/// Why a single streaming round ended without producing a response.
enum StreamRoundError {
    /// The request never got off the ground — `send_message_stream` itself
    /// failed. Not re-issued here: `RetryingProvider` already wraps that call in
    /// `with_retry`, so retrying it again would multiply the provider's attempts
    /// rather than add coverage.
    Establish(LLMError),
    /// The stream failed after it had been established. `emitted` records
    /// whether any token had already reached the client — if it had, the round
    /// cannot be safely re-issued because the output would be duplicated.
    Stream { source: LLMError, emitted: bool },
    /// An engine-level failure that must propagate untouched (cancellation,
    /// or a stream that ended without a complete response).
    Fatal(HubError),
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

    /// Run one streaming LLM round, re-issuing it on a transient transport
    /// failure so long as nothing has reached the client yet.
    ///
    /// `RetryingProvider` only retries *establishing* a stream — errors yielded
    /// by the returned stream land outside its `with_retry` wrapper entirely.
    /// This closes that gap, and only that gap: an `Establish` failure is passed
    /// straight through, because retrying it here would stack on top of the
    /// provider's own attempts instead of covering anything new.
    ///
    /// The `!emitted` guard is what makes the re-issue safe: once a token has
    /// been streamed, replaying the round would duplicate output, so the error
    /// is surfaced instead.
    async fn stream_round_with_retry(
        &self,
        request: LLMRequest,
        sink: &dyn StreamSink,
        cancel: Option<&CancellationToken>,
        round: u32,
    ) -> Result<LLMResponse, HubError> {
        let mut attempts: u32 = 0;
        let mut backoff = ExponentialBackoff::new(BackoffConfig::default());

        loop {
            match self
                .stream_round(request.clone(), sink, cancel, round)
                .await
            {
                Ok(response) => return Ok(response),
                Err(StreamRoundError::Fatal(e)) => return Err(e),
                Err(StreamRoundError::Establish(source)) => {
                    let msg = format!("stream error at round {}: {}", round, source);
                    sink.error(&msg).await;
                    return Err(HubError::LlmCallFailed { round, source });
                }
                Err(StreamRoundError::Stream { source, emitted }) => {
                    let retryable = !emitted
                        && attempts < crate::constants::MAX_STREAM_RETRY_ATTEMPTS
                        && RetryPolicy::Default.should_retry(&source);

                    if retryable {
                        attempts += 1;
                        warn!(
                            round,
                            attempts,
                            error = %source,
                            "Stream failed before any output — re-issuing round"
                        );
                        if let Some(delay) = backoff.next() {
                            tokio::time::sleep(delay).await;
                        }
                        continue;
                    }

                    let msg = format!("stream error at round {}: {}", round, source);
                    sink.error(&msg).await;
                    return Err(HubError::LlmCallFailed { round, source });
                }
            }
        }
    }

    /// Consume a single streaming response into an `LLMResponse`.
    async fn stream_round(
        &self,
        request: LLMRequest,
        sink: &dyn StreamSink,
        cancel: Option<&CancellationToken>,
        round: u32,
    ) -> Result<LLMResponse, StreamRoundError> {
        let stream = self
            .provider
            .send_message_stream(request)
            .await
            .map_err(StreamRoundError::Establish)?;

        let mut accumulator = StreamAccumulator::new();
        let mut pinned = std::pin::pin!(stream);
        let mut emitted = false;

        loop {
            let chunk_result = if let Some(ct) = cancel {
                tokio::select! {
                    biased;
                    _ = ct.cancelled() => {
                        return Err(StreamRoundError::Fatal(HubError::Cancelled));
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
                        emitted = true;
                    }
                    // Note: tool_start is NOT emitted here. The execution loop
                    // (below) sends tool_start/tool_end when the tool actually
                    // runs, avoiding duplicate events on the frontend.
                    accumulator.apply(&chunk);
                }
                Some(Err(source)) => {
                    return Err(StreamRoundError::Stream { source, emitted });
                }
                None => break,
            }
        }

        accumulator.build().ok_or_else(|| {
            StreamRoundError::Fatal(HubError::Internal(anyhow::anyhow!(
                "incomplete stream at round {}",
                round
            )))
        })
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
        let ctx = LoopContext {
            strategy,
            sink,
            recorder,
            cancel,
        };
        let mut messages = strategy.build_messages(input).await?;
        let max_rounds = strategy.max_rounds();
        let budget = strategy.context_budget();
        let mut total_input: u64 = 0;
        let mut total_output: u64 = 0;
        let mut total_cached: u64 = 0;

        // ── on_start filters ──
        let system_prompt = if let Some(ref filter_ctx) = self.filter_ctx {
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

        // Track consecutive identical tool failures for loop-break detection
        let mut consecutive_failures: HashMap<u64, u32> = HashMap::new();

        // Track premature EndTurn re-prompts (for strategies with terminal tools)
        let mut end_turn_retries: u32 = 0;

        for round in 0..max_rounds {
            // Check cancellation
            if cancel.is_some_and(|t| t.is_cancelled()) {
                return Err(HubError::Cancelled);
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
                .with_max_tokens(strategy.max_tokens());
            request.temperature = strategy.temperature();
            request.effort = strategy.effort();
            let tools = strategy.tools();
            if !tools.is_empty() {
                request = request.with_tools(tools);
            }

            // Call LLM
            let response = if strategy.streaming() {
                request = request.with_streaming();
                self.stream_round_with_retry(request, sink, cancel, round)
                    .await?
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
            total_cached += response.usage.cached_input_tokens as u64;

            // Check stop reason
            match response.stop_reason {
                StopReason::ToolUse => {
                    let has_tool_blocks = response
                        .content_blocks
                        .iter()
                        .any(|b| matches!(b, ContentBlock::ToolUse { .. }));

                    if !has_tool_blocks {
                        // Providers occasionally report ToolUse with no blocks.
                        // The turn is over either way, so treat it exactly like
                        // EndTurn. This used to `break`, falling through to
                        // MaxRoundsExhausted below — labelling a clean finish as
                        // a budget failure and discarding the response text.
                        warn!(
                            "StopReason::ToolUse but no tool_use blocks at round {}",
                            round
                        );
                        if let Some(result) = self
                            .handle_end_turn(
                                &ctx,
                                round,
                                &response,
                                &mut messages,
                                total_input,
                                total_output,
                                total_cached,
                                &mut filter_retried,
                                &mut end_turn_retries,
                            )
                            .await?
                        {
                            return Ok(result);
                        }
                        continue;
                    }

                    if let Some(result) = self
                        .handle_tool_use_round(
                            &ctx,
                            round,
                            &response,
                            &mut messages,
                            total_input,
                            total_output,
                            total_cached,
                            &mut consecutive_failures,
                        )
                        .await?
                    {
                        return Ok(result);
                    }
                }
                // A filtered completion terminates the turn like any other
                // terminal reason; it is warned about rather than treated as a
                // clean finish, because the text is truncated by policy.
                StopReason::ContentFiltered
                | StopReason::EndTurn
                | StopReason::MaxTokens
                | StopReason::StopSequence => {
                    if response.stop_reason == StopReason::ContentFiltered {
                        tracing::warn!(
                            round,
                            "provider blocked the completion on its content policy; \
                             the assistant message is truncated"
                        );
                    }
                    if let Some(result) = self
                        .handle_end_turn(
                            &ctx,
                            round,
                            &response,
                            &mut messages,
                            total_input,
                            total_output,
                            total_cached,
                            &mut filter_retried,
                            &mut end_turn_retries,
                        )
                        .await?
                    {
                        return Ok(result);
                    }
                    // None means a filter or terminal-tool check requested retry — continue the loop
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
    #[allow(clippy::too_many_arguments)]
    async fn handle_tool_use_round(
        &self,
        ctx: &LoopContext<'_>,
        round: u32,
        response: &LLMResponse,
        messages: &mut Vec<Message>,
        total_input: u64,
        total_output: u64,
        total_cached: u64,
        consecutive_failures: &mut HashMap<u64, u32>,
    ) -> Result<Option<ExecutionResult>, HubError> {
        let LoopContext {
            strategy,
            sink,
            recorder,
            cancel,
        } = ctx;
        let cancel = *cancel;
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

            // Arguments the provider could not parse never reach the tool.
            // Dispatching them would run the tool with `{}` and report the
            // first required parameter missing — which is not the model's
            // actual mistake, gives it nothing to correct, and had agents
            // re-sending the same broken form until the round budget ran out.
            // Quote the raw text back instead and name the shape wanted.
            let result = match unparsed_arguments(tool_input) {
                Some(raw) => {
                    warn!(round, tool = %tool_name, "tool call had unparsable arguments");
                    json!({
                        "error": format!(
                            "Could not parse the arguments to `{tool_name}`. Received: {raw}\n\
                             Arguments must be a JSON object, for example \
                             {{\"command\": \"ls -la\"}} — not keyword arguments and not \
                             a bare string. Send the call again in that form."
                        )
                    })
                }
                // The same dead end from the other direction: nothing arrived
                // rather than something unreadable. Answer it here for the
                // same reason — the tool's own "missing parameter" error
                // cannot tell the model that it sent no arguments at all.
                None => match missing_all_arguments(*strategy, tool_name, tool_input) {
                    Some(required) => {
                        warn!(round, tool = %tool_name, "tool call arrived with no arguments");
                        no_arguments_error(tool_name, &required)
                    }
                    None => strategy.execute_tool(tool_name, tool_input).await,
                },
            };

            sink.tool_end(tool_name, tool_id, &result).await;

            // Track consecutive identical failures
            let hash = tool_call_hash(tool_name, tool_input);
            if let Some(error_summary) = is_tool_failure(&result) {
                let count = consecutive_failures.entry(hash).or_insert(0);
                *count += 1;

                if *count >= TOOL_FAILURE_HINT_THRESHOLD * 2 {
                    return Err(HubError::RepeatedToolFailure {
                        tool_name: tool_name.clone(),
                        count: *count,
                    });
                }

                if *count == TOOL_FAILURE_HINT_THRESHOLD {
                    warn!(
                        round,
                        tool = %tool_name,
                        count = *count,
                        "consecutive identical tool failure — injecting hint"
                    );
                    // Inject the hint into the tool result so the LLM sees it
                    let hint = format!(
                        "{}\n\n[System: This tool has failed {} consecutive times \
                         with identical input. Error: {}. You MUST try a \
                         fundamentally different approach.]",
                        result, TOOL_FAILURE_HINT_THRESHOLD, error_summary,
                    );
                    result_blocks.push(ContentBlock::ToolResult {
                        tool_use_id: tool_id.clone(),
                        content: hint,
                    });
                    continue;
                }
            } else {
                consecutive_failures.remove(&hash);
            }

            let result_str = match &result {
                Value::String(s) => s.clone(),
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
                    response.reasoning.clone(),
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
                            None,
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
                input_tokens: total_input.min(u32::MAX as u64) as u32,
                output_tokens: total_output.min(u32::MAX as u64) as u32,
                cached_input_tokens: total_cached.min(u32::MAX as u64) as u32,
            };
            strategy.on_complete("", &usage).await?;
            sink.done().await;
            return Ok(Some(ExecutionResult {
                content: String::new(),
                content_blocks: response.content_blocks.clone(),
                input_tokens: total_input,
                output_tokens: total_output,
                cached_input_tokens: total_cached,
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
    #[allow(clippy::too_many_arguments)]
    async fn handle_end_turn(
        &self,
        ctx: &LoopContext<'_>,
        round: u32,
        response: &LLMResponse,
        messages: &mut Vec<Message>,
        total_input: u64,
        total_output: u64,
        total_cached: u64,
        filter_retried: &mut [bool],
        end_turn_retries: &mut u32,
    ) -> Result<Option<ExecutionResult>, HubError> {
        let LoopContext {
            strategy,
            sink,
            recorder,
            ..
        } = ctx;

        // ── Premature EndTurn check ──
        // If the strategy requires a terminal tool (e.g. complete_system) and it
        // hasn't been called yet, re-prompt the LLM instead of completing.
        if let Some(terminal_tool) = strategy.requires_terminal_tool() {
            if !strategy.should_stop() && *end_turn_retries < MAX_END_TURN_RETRIES {
                *end_turn_retries += 1;
                warn!(
                    round,
                    terminal_tool,
                    attempt = *end_turn_retries,
                    "LLM returned EndTurn without calling terminal tool — re-prompting"
                );
                messages.push(Message::assistant(&response.content));
                messages.push(Message::user(format!(
                    "You must call the `{terminal_tool}` tool to complete this task. \
                     Do not end your turn without calling it. Review your work and \
                     call `{terminal_tool}` now."
                )));
                return Ok(None);
            }
        }

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
                    response.reasoning.clone(),
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
            input_tokens: total_input.min(u32::MAX as u64) as u32,
            output_tokens: total_output.min(u32::MAX as u64) as u32,
            cached_input_tokens: total_cached.min(u32::MAX as u64) as u32,
        };

        // Let strategy do post-processing
        strategy.on_complete(&final_content, &usage).await?;

        sink.done().await;

        Ok(Some(ExecutionResult {
            content: final_content,
            content_blocks: response.content_blocks.clone(),
            input_tokens: total_input,
            output_tokens: total_output,
            cached_input_tokens: total_cached,
            cost_usd: 0.0, // Strategies compute cost in on_complete
            rounds_used: round + 1,
        }))
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
