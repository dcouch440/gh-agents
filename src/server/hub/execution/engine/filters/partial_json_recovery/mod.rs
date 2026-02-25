//! PartialJsonRecoveryFilter — fixes truncated JSON by auto-closing brackets.
//!
//! When an LLM response is cut short (MaxTokens), JSON output may be
//! incomplete. This filter tracks bracket/brace nesting and appends
//! the necessary closing characters to make the JSON parseable.

use async_trait::async_trait;
use serde_json::Value as JsonValue;
use tracing::debug;

use super::{ExecutionFilter, FilterContext, HubError};

/// Recovers partial JSON by closing unclosed brackets and braces.
#[derive(Default)]
pub struct PartialJsonRecoveryFilter;

impl PartialJsonRecoveryFilter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ExecutionFilter for PartialJsonRecoveryFilter {
    fn name(&self) -> &str {
        "partial_json_recovery"
    }

    async fn on_output(&self, ctx: &FilterContext, content: String) -> Result<String, HubError> {
        if !ctx.has_output_schema {
            return Ok(content);
        }

        // If it already parses, nothing to do
        if serde_json::from_str::<JsonValue>(&content).is_ok() {
            return Ok(content);
        }

        // Try to recover
        match recover_truncated_json(&content) {
            Some(recovered) => {
                debug!(filter = "partial_json_recovery", "recovered truncated JSON");
                Ok(recovered)
            }
            None => Ok(content),
        }
    }
}

/// Attempt to close unclosed JSON brackets/braces.
///
/// Tracks a stack of open delimiters (`{`, `[`) while respecting string
/// literals and escape sequences. Appends closing delimiters in reverse order.
fn recover_truncated_json(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Find the start of JSON content
    let json_start = trimmed.find(['{', '['])?;
    let json_content = &trimmed[json_start..];

    let mut stack: Vec<char> = Vec::new();
    let mut in_string = false;
    let mut escape_next = false;

    for ch in json_content.chars() {
        if escape_next {
            escape_next = false;
            continue;
        }

        if in_string {
            match ch {
                '\\' => escape_next = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '{' => stack.push('}'),
            '[' => stack.push(']'),
            '}' | ']' => {
                if let Some(expected) = stack.last() {
                    if *expected == ch {
                        stack.pop();
                    }
                }
            }
            _ => {}
        }
    }

    if stack.is_empty() {
        return None; // Already balanced — parse failure is something else
    }

    let mut result = json_content.to_string();

    // Handle unclosed string
    if in_string {
        result.push('"');
    }

    // Close in reverse order
    while let Some(closer) = stack.pop() {
        result.push(closer);
    }

    // Verify the recovery actually produces valid JSON
    if serde_json::from_str::<JsonValue>(&result).is_ok() {
        Some(result)
    } else {
        None
    }
}

#[cfg(test)]
mod tests;
