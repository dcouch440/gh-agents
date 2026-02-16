//! Agent Designer pre-lifecycle for task force execution.
//!
//! Builds a `DesignerInput` from the task force's mission brief and roster,
//! then delegates to the generic `run_agent_designer()`. Maps results back
//! to the task-force-specific `DesignedAgentPrompt` type.

mod tests;

use std::collections::HashMap;

use anyhow::anyhow;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::db::{TaskAgentRosterRow, TaskMissionBriefRow, WorkflowStepRow};
use crate::server::hub::error::HubError;
use crate::types::StepExecutionEnvelope;

use super::super::agent_designer;
use super::super::designer_input::task_force::build_task_force_designer_input;
use super::super::WorkflowExecutionContext;

use crate::server::hub::engine::ExecutionEngine;
use crate::server::state::AppState;

// ── Output types ────────────────────────────────────────────────────────────

/// Output from the Agent Designer — one prompt pair + tool assignment per agent.
#[derive(Debug, Clone)]
pub struct DesignedAgentPrompt {
    pub agent_roster_entry_id: Uuid,
    pub agent_name: String,
    pub tools: Vec<String>,
    pub system_prompt: String,
    pub task_prompt: String,
    pub reasoning: String,
    pub execution_order: i32,
    pub receives_from: Vec<String>,
}

/// Token usage from the designer call, for accumulating into step totals.
pub struct DesignerTokenUsage {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost_usd: f32,
    /// The designer run ID for linking agent phases back to their designer.
    pub run_id: Uuid,
}

// ── Main execution function ─────────────────────────────────────────────────

/// Run the Agent Designer pre-lifecycle for a task force.
///
/// Builds a `DesignerInput` from the mission brief and roster, delegates to
/// the generic `run_agent_designer()`, then maps results back to task-force
/// types including roster entry ID matching.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_agent_designer(
    engine: &ExecutionEngine,
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    step: &WorkflowStepRow,
    brief: &TaskMissionBriefRow,
    roster: &[TaskAgentRosterRow],
    completed_envelopes: &HashMap<Uuid, StepExecutionEnvelope>,
    steps: &[WorkflowStepRow],
    cancel: Option<&CancellationToken>,
    protocol_execution_id: Option<Uuid>,
) -> Result<(Vec<DesignedAgentPrompt>, DesignerTokenUsage), HubError> {
    // Load assistant notes for the designer
    let assistant_notes = state
        .repos()
        .workflows
        .get_assistant_notes(step.id)
        .await
        .unwrap_or_default();

    // Build the generic DesignerInput from task force config
    let input = build_task_force_designer_input(
        brief,
        roster,
        completed_envelopes,
        steps,
        assistant_notes.as_deref(),
    );

    // Delegate to the generic designer
    let result =
        agent_designer::run_agent_designer(engine, state, ctx, step, input, "", cancel, protocol_execution_id).await?;

    // Map generic results back to task-force-specific types
    let mut designed_prompts = Vec::with_capacity(result.prompts.len());

    for entry in &result.prompts {
        // Find matching roster entry by agent_id
        let roster_entry = roster
            .iter()
            .find(|r| r.id.to_string() == entry.agent_id)
            .ok_or_else(|| {
                HubError::Internal(anyhow!(
                    "Designer referenced unknown agent_id: {}",
                    entry.agent_id
                ))
            })?;

        designed_prompts.push(DesignedAgentPrompt {
            agent_roster_entry_id: roster_entry.id,
            agent_name: entry.agent_name.clone(),
            tools: entry.tools.clone(),
            system_prompt: entry.system_prompt.clone(),
            task_prompt: entry.task_prompt.clone(),
            reasoning: entry.reasoning.clone(),
            execution_order: roster_entry.execution_order,
            receives_from: entry.receives_from.clone(),
        });
    }

    // Sort by execution_order
    designed_prompts.sort_by_key(|p| p.execution_order);

    let token_usage = DesignerTokenUsage {
        input_tokens: result.input_tokens,
        output_tokens: result.output_tokens,
        cost_usd: result.cost_usd,
        run_id: result.run_id,
    };

    Ok((designed_prompts, token_usage))
}
