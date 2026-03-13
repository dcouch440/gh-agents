//! Shared types for the workforce module.

use uuid::Uuid;

use crate::execution::container::ContainerHandle;
use crate::server::hub::error::HubError;
use crate::server::state::AppState;

/// Output from the Agent Designer — system prompt + assignment + tool selection per agent.
#[derive(Debug, Clone)]
pub(crate) struct DesignedAgentPrompt {
    pub agent_roster_entry_id: Uuid,
    pub agent_name: String,
    pub tools: Vec<String>,
    pub system_prompt: String,
    pub assignment: String,
    pub execution_order: i32,
    pub receives_from: Vec<String>,
}

/// Result from executing a single agent — returned by `execute_single_agent`.
pub(crate) struct AgentExecutionResult {
    pub name: String,
    pub content: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost: f32,
}

/// Per-step execution environment for workforce agent dispatch.
///
/// Bundles values constant across all agents in a workforce step.
/// Clone is required for JoinSet spawning in the parallel path.
#[derive(Clone)]
pub(super) struct WorkforceStepEnv {
    pub state: AppState,
    pub ctx: super::super::utils::types::WorkflowExecutionContext,
    pub user_notes_block: String,
    pub original_prompt: String,
    pub step_id: Uuid,
    pub workflow_id: Uuid,
    pub designer_run_id: Option<Uuid>,
    pub total_agents: usize,
    pub container_handle: Option<ContainerHandle>,
    pub cancel: Option<tokio_util::sync::CancellationToken>,
    pub task_description: String,
    /// Base64-encoded PNG rasterized at runtime from stroke coordinates in board_context.
    pub stroke_image: Option<String>,
    /// Pre-formatted block of upstream DAG step outputs (workforce, single).
    /// Excludes context-mode steps (handled by user_notes_block).
    pub upstream_outputs_block: String,
}

/// Aggregated results from executing all agent levels.
pub(super) struct LevelExecutionResult {
    pub agent_outputs: Vec<(String, String)>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost_usd: f32,
}

/// Result of handling an agent failure based on the configured failure mode.
pub(super) enum AgentFailureAction {
    /// Skip this agent, recording an error output in its place.
    Skip { name: String, error_output: String },
    /// Abort the entire workforce step with this error.
    Abort(HubError),
}
