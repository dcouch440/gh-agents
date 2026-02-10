//! FewShotFilter — injects exemplary execution traces as demonstration pairs.
//!
//! When a workflow step has exemplary executions (marked by the user), this
//! filter loads their input/output pairs and prepends them as user/assistant
//! messages before the actual prompt. The LLM sees concrete examples of the
//! expected behaviour, improving output quality and format consistency.
//!
//! No effect when there are no exemplary executions for the agent+step.

use std::sync::Arc;

use async_trait::async_trait;
use tracing::debug;

use crate::db::traits::AgentExecutionRepo;
use crate::llm::Message;

use super::{ExecutionFilter, FilterContext, HubError};

/// Injects few-shot examples from exemplary execution traces.
pub struct FewShotFilter {
    repo: Arc<dyn AgentExecutionRepo>,
}

impl FewShotFilter {
    pub fn new(repo: Arc<dyn AgentExecutionRepo>) -> Self {
        Self { repo }
    }
}

/// Maximum number of few-shot examples to inject.
const MAX_EXAMPLES: u32 = 3;

#[async_trait]
impl ExecutionFilter for FewShotFilter {
    fn name(&self) -> &str {
        "few_shot"
    }

    async fn on_start(
        &self,
        ctx: &FilterContext,
        system_prompt: String,
        messages: Vec<Message>,
    ) -> Result<(String, Vec<Message>), HubError> {
        let rows = self
            .repo
            .list_exemplary_executions(ctx.agent_id, ctx.step_id, MAX_EXAMPLES)
            .await
            .map_err(HubError::Internal)?;

        // Build example pairs, skipping rows with no output.
        let examples: Vec<_> = rows
            .into_iter()
            .filter_map(|row| row.output.map(|output| (row.input, output)))
            .collect();

        if examples.is_empty() {
            return Ok((system_prompt, messages));
        }

        debug!(
            filter = "few_shot",
            count = examples.len(),
            agent_id = %ctx.agent_id,
            step_id = ?ctx.step_id,
            "injecting few-shot examples"
        );

        // Augment system prompt with a note about examples.
        let mut augmented = system_prompt;
        augmented.push_str(concat!(
            "\n\n<examples>\n",
            "The following conversation turns demonstrate successful input/output ",
            "examples for this task. Use them as reference for format and quality.\n",
            "</examples>",
        ));

        // Prepend example pairs before existing messages.
        let mut new_messages = Vec::with_capacity(examples.len() * 2 + messages.len());
        for (input, output) in &examples {
            new_messages.push(Message::user(input));
            new_messages.push(Message::assistant(output));
        }
        new_messages.extend(messages);

        Ok((augmented, new_messages))
    }
}

#[cfg(test)]
mod tests;
