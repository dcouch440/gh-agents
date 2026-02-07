//! AgentGuidanceFilter — loads per-agent guidance from the database.
//!
//! Queries `agent_guidances` for active suggestions matching the agent
//! (and optionally the workflow step), then appends them to the system
//! prompt. This is the persistence-backed "learning" filter inspired by
//! CrewAI's training approach.

use std::sync::Arc;

use async_trait::async_trait;

use crate::db::traits::ServerRepo;
use crate::llm::Message;

use super::{ExecutionFilter, FilterContext, HubError};

/// Injects stored guidance into the system prompt.
pub struct AgentGuidanceFilter {
    repo: Arc<dyn ServerRepo>,
}

impl AgentGuidanceFilter {
    pub fn new(repo: Arc<dyn ServerRepo>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ExecutionFilter for AgentGuidanceFilter {
    fn name(&self) -> &str {
        "agent_guidance"
    }

    async fn on_start(
        &self,
        ctx: &FilterContext,
        system_prompt: String,
        messages: Vec<Message>,
    ) -> Result<(String, Vec<Message>), HubError> {
        let rows = self
            .repo
            .get_agent_guidances(ctx.agent_id, ctx.step_id)
            .await
            .map_err(HubError::Internal)?;

        if rows.is_empty() {
            return Ok((system_prompt, messages));
        }

        // Collect all suggestion strings from all matching rows
        let mut suggestions: Vec<String> = Vec::new();
        for row in &rows {
            if let Some(arr) = row.suggestions.as_array() {
                for item in arr {
                    if let Some(s) = item.as_str() {
                        suggestions.push(s.to_string());
                    }
                }
            }
        }

        if suggestions.is_empty() {
            return Ok((system_prompt, messages));
        }

        let mut augmented = system_prompt;
        augmented.push_str("\n\n## Agent Guidance\nYou MUST follow these instructions derived from prior feedback:\n");
        for suggestion in &suggestions {
            augmented.push_str(&format!("- {}\n", suggestion));
        }

        Ok((augmented, messages))
    }
}

mod tests;
