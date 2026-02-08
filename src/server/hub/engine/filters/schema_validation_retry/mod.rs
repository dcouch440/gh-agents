//! SchemaValidationRetryFilter — retries when LLM output doesn't match schema.
//!
//! If the execution expects structured JSON output but the LLM's response
//! is not valid JSON, this filter returns `Retry` with error context so
//! the LLM can correct itself. Limited to 1 retry (enforced by the engine).

use async_trait::async_trait;

use crate::llm::LLMResponse;

use super::{ExecutionFilter, FilterContext, HubError, ResponseAction};

/// Validates LLM output against the expected JSON schema and retries on failure.
#[derive(Default)]
pub struct SchemaValidationRetryFilter;

impl SchemaValidationRetryFilter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ExecutionFilter for SchemaValidationRetryFilter {
    fn name(&self) -> &str {
        "schema_validation_retry"
    }

    async fn on_response(
        &self,
        ctx: &FilterContext,
        response: &LLMResponse,
    ) -> Result<ResponseAction, HubError> {
        if !ctx.has_output_schema {
            return Ok(ResponseAction::Accept);
        }

        let content = response.content.trim();

        match serde_json::from_str::<serde_json::Value>(content) {
            Ok(value) => {
                if value.is_object() || value.is_array() {
                    Ok(ResponseAction::Accept)
                } else {
                    Ok(ResponseAction::Retry {
                        feedback: format!(
                            "Your response parsed as JSON but is a primitive value ({}). \
                             The downstream system expects a JSON object or array matching the schema. \
                             Respond with the JSON object only.",
                            json_type_name(&value),
                        ),
                    })
                }
            }
            Err(parse_err) => {
                // Check if content is wrapped in markdown code fences
                if let Some(json_str) = try_extract_from_fence(content) {
                    if serde_json::from_str::<serde_json::Value>(&json_str).is_ok() {
                        return Ok(ResponseAction::Retry {
                            feedback: "Your JSON content is valid but you wrapped it in markdown \
                                       code fences. The parser reads your entire response as raw JSON, \
                                       so the fence markers cause a parse error. Respond with the \
                                       JSON object only."
                                .to_string(),
                        });
                    }
                }

                Ok(ResponseAction::Retry {
                    feedback: format!(
                        "The downstream parser failed to parse your response. Error: {}\n\n\
                         Respond with a valid JSON object matching the schema. \
                         The parser reads your entire response as raw JSON, \
                         so it must contain only the JSON object.",
                        parse_err,
                    ),
                })
            }
        }
    }
}

/// Try to extract JSON from markdown code fences.
fn try_extract_from_fence(content: &str) -> Option<String> {
    let start = content.find("```json").or_else(|| content.find("```"))?;
    let after_fence = if content[start..].starts_with("```json") {
        &content[start + 7..]
    } else {
        &content[start + 3..]
    };
    let end = after_fence.find("```")?;
    Some(after_fence[..end].trim().to_string())
}

fn json_type_name(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

mod tests;
