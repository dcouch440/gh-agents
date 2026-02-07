//! SchemaEnhancementFilter — adds negative examples to schema instructions.
//!
//! When a step has an output_schema, this filter augments the system prompt
//! with explicit "do NOT" patterns, reducing common LLM output mistakes
//! like wrapping JSON in markdown fences or adding explanatory text.

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
            "- Output ONLY the raw JSON object. No markdown code fences (```), no commentary.\n",
            "- Do NOT wrap your response in ```json ... ```. The consumer parses raw JSON directly.\n",
            "- Do NOT include any text before or after the JSON object.\n",
            "- Do NOT include explanatory sentences like \"Here is the JSON:\".\n",
            "- If a field is optional and you have no value, use null rather than omitting it.\n",
            "- Every string value must be properly escaped JSON.\n",
            "</output_rules>",
        ));

        Ok((augmented, messages))
    }
}

mod tests;
