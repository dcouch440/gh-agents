//! SchemaEnhancementFilter — adds formatting guidance to schema instructions.
//!
//! When a step has an output_schema, this filter augments the system prompt
//! with positive formatting rules and WHY context, reducing common LLM output
//! mistakes like wrapping JSON in markdown fences or adding explanatory text.

use async_trait::async_trait;

use crate::llm::Message;

use super::{ExecutionFilter, FilterContext, HubError};

/// Enhances schema enforcement instructions in the system prompt.
#[derive(Default)]
pub struct SchemaEnhancementFilter;

impl SchemaEnhancementFilter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ExecutionFilter for SchemaEnhancementFilter {
    fn name(&self) -> &str {
        "schema_enhancement"
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
            "\n\n<output_rules>\n",
            "- Respond with the raw JSON object only. A downstream parser calls JSON.parse() on your entire response, so any surrounding text causes a parse failure.\n",
            "- Use null for optional fields with no value. Omitting fields causes schema validation errors.\n",
            "- Escape all string values as valid JSON. Unescaped newlines or quotes break the parser.\n",
            "</output_rules>",
        ));

        Ok((augmented, messages))
    }
}

mod tests;
