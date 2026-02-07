//! ReasoningTraceFilter — chain-of-thought wrapper for structured outputs.
//!
//! When enabled, this filter instructs the LLM to wrap its response in a
//! `{"reasoning": "...", "result": ...}` structure. The `on_output` hook
//! strips the reasoning and returns only the result, so downstream steps
//! receive clean schema-conformant output. The full response (with reasoning)
//! is already stored in execution_messages for review.

use async_trait::async_trait;
use serde_json::Value as JsonValue;
use tracing::debug;

use crate::llm::Message;

use super::{ExecutionFilter, FilterContext, HubError};

/// Forces chain-of-thought reasoning before structured output.
#[derive(Default)]
pub struct ReasoningTraceFilter;

impl ReasoningTraceFilter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ExecutionFilter for ReasoningTraceFilter {
    fn name(&self) -> &str {
        "reasoning_trace"
    }

    async fn on_start(
        &self,
        ctx: &FilterContext,
        system_prompt: String,
        messages: Vec<Message>,
    ) -> Result<(String, Vec<Message>), HubError> {
        if !ctx.has_output_schema {
            return Ok((system_prompt, messages));
        }

        let mut augmented = system_prompt;
        augmented.push_str(concat!(
            "\n\n<reasoning_format>\n",
            "You MUST wrap your response in the following structure:\n",
            "{\n",
            "  \"reasoning\": \"Your step-by-step thought process for arriving at the answer\",\n",
            "  \"result\": <your response matching the schema above>\n",
            "}\n",
            "Think through the problem carefully in the \"reasoning\" field ",
            "before producing your final answer in \"result\".\n",
            "</reasoning_format>",
        ));

        Ok((augmented, messages))
    }

    async fn on_output(&self, ctx: &FilterContext, content: String) -> Result<String, HubError> {
        if !ctx.has_output_schema {
            return Ok(content);
        }

        let parsed = match serde_json::from_str::<JsonValue>(&content) {
            Ok(v) => v,
            Err(_) => return Ok(content),
        };

        // Extract the result field if present
        if let Some(obj) = parsed.as_object() {
            if let Some(result) = obj.get("result") {
                if let Some(reasoning) = obj.get("reasoning").and_then(|r| r.as_str()) {
                    debug!(
                        filter = "reasoning_trace",
                        reasoning_len = reasoning.len(),
                        "stripped reasoning from output"
                    );
                }
                return Ok(serde_json::to_string(result).unwrap_or(content));
            }
        }

        // LLM didn't follow the wrapper format — pass through unchanged
        Ok(content)
    }
}

mod tests;
