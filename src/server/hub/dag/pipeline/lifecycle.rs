//! Pipeline lifecycle trait and shared context types.
//!
//! Defines the `PipelinePhase` trait for composable before/after phases
//! in pipeline execution. Each phase runs like a DAG element — traceable,
//! with execution records — but is not a physical workflow step.

use std::collections::HashMap;

use uuid::Uuid;

use crate::db::{TaskAgentRosterRow, TaskMissionBriefRow, WorkflowStepRow};
use crate::server::hub::error::HubError;
use crate::types::StepExecutionEnvelope;

use super::super::DagContext;
use super::types::DesignedAgentPrompt;

/// Shared context available to all pipeline phases.
///
/// Built once at pipeline start from the parent step's configuration
/// and upstream DAG state. Immutable during execution — phases read
/// from this, they do not modify it.
pub(crate) struct PipelineExecutionContext {
    /// The parent workforce step being executed.
    pub step: WorkflowStepRow,
    /// Mission brief (task description, failure mode, capabilities).
    pub brief: TaskMissionBriefRow,
    /// Agent roster sorted by execution_order.
    pub roster: Vec<TaskAgentRosterRow>,
    /// Composed base prompt from step template + variable resolution.
    pub base_prompt: String,
    /// Upstream context data from context nodes: `(title, content)` pairs.
    pub upstream_context: Vec<(String, String)>,
    /// Completed envelopes from upstream steps (for port resolution + designer input).
    pub completed_envelopes: HashMap<Uuid, StepExecutionEnvelope>,
}

/// Output from a before-phase, consumed by agent execution.
///
/// Contains the designed prompts for each roster agent, a user notes block
/// to inject into agent task prompts, and token usage for cost tracking.
pub(crate) struct PhaseOutput {
    /// Designed prompts for each roster agent (one per agent).
    pub designed_prompts: Vec<DesignedAgentPrompt>,
    /// User notes block built from upstream context nodes.
    pub user_notes_block: String,
    /// Token usage from this phase.
    pub token_usage: PhaseTokenUsage,
}

/// Token usage from a single lifecycle phase.
pub(crate) struct PhaseTokenUsage {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost_usd: f32,
    /// Phase-specific run ID for linking (e.g., designer_run_id).
    pub run_id: Option<Uuid>,
}

/// A lifecycle phase that runs before or after pipeline agent execution.
///
/// Each implementation:
/// 1. Creates `ProtocolExecutionRow` records for traceability
/// 2. Produces `DesignedAgentPrompt`s (via LLM or static mapping)
/// 3. Reports token usage for cost accumulation
///
/// Phases are registered on a `Pipeline` via `.before()` or `.after()`
/// and execute in registration order.
#[async_trait::async_trait]
pub(crate) trait PipelinePhase: Send + Sync {
    /// Human-readable phase name for tracing and logging.
    fn name(&self) -> &str;

    /// Execute the phase, producing designed prompts for agent execution.
    async fn execute(
        &self,
        dag: &DagContext<'_>,
        ctx: &PipelineExecutionContext,
    ) -> Result<PhaseOutput, HubError>;
}
