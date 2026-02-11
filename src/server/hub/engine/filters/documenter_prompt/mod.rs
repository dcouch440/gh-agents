//! DocumenterPromptFilter — augments system prompt with document definitions.
//!
//! At execution time, this filter fetches the protocol document definitions
//! for the current step and appends them to the system prompt so the
//! Documenter Strategist agent knows what documents to plan for.

use std::sync::Arc;

use async_trait::async_trait;
use tracing::debug;

use crate::db::traits::WorkflowRepo;
use crate::llm::Message;
use crate::server::hub::protocols::compilers::documenter::prompt::format_document_defs_section;

use super::{ExecutionFilter, FilterContext, HubError};

/// Injects document definitions into the system prompt for documenter steps.
pub struct DocumenterPromptFilter {
    repo: Arc<dyn WorkflowRepo>,
}

impl DocumenterPromptFilter {
    pub fn new(repo: Arc<dyn WorkflowRepo>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ExecutionFilter for DocumenterPromptFilter {
    fn name(&self) -> &str {
        "documenter_prompt"
    }

    async fn on_start(
        &self,
        ctx: &FilterContext,
        system_prompt: String,
        messages: Vec<Message>,
    ) -> Result<(String, Vec<Message>), HubError> {
        let step_id = match ctx.step_id {
            Some(id) => id,
            None => return Ok((system_prompt, messages)),
        };

        let defs = self.repo.list_document_defs(step_id).await.map_err(|e| {
            HubError::Internal(anyhow::anyhow!("Failed to fetch document defs: {e}"))
        })?;

        if defs.is_empty() {
            debug!(
                filter = "documenter_prompt",
                step_id = %step_id,
                "no document definitions found, skipping augmentation"
            );
            return Ok((system_prompt, messages));
        }

        let mut augmented = system_prompt;
        augmented.push_str("\n\n");
        augmented.push_str(&format_document_defs_section(&defs));

        debug!(
            filter = "documenter_prompt",
            step_id = %step_id,
            doc_count = defs.len(),
            "augmented system prompt with document definitions"
        );

        Ok((augmented, messages))
    }
}

#[cfg(test)]
mod tests;
