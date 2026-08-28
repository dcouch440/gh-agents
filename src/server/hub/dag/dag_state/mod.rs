//! Shared state, types, and helpers for DAG execution.
//!
//! Contains the mutable execution state accumulated during DAG traversal,
//! port metadata for step I/O, and helper functions used across all
//! step execution modules.

use std::collections::HashMap;

use serde_json::Value as JsonValue;
use tracing::{debug, warn};
use uuid::Uuid;

use tokio_util::sync::CancellationToken;

use crate::db::{
    AgentRow, StepInputRow, StepOutputRow, StepRoutingRuleRow, WorkflowStepEdgeRow, WorkflowStepRow,
};
use crate::server::hub::engine::ExecutionEngine;
use crate::server::hub::error::HubError;
use crate::server::state::AppState;
use crate::server::ws::events::WorkflowEventKind;
use crate::types::{ExecutionMetadata, ExecutionStatus, StepExecutionEnvelope};

use super::broadcast_workflow_event;
use super::utils::{resolve_port_inputs, StepOutput, WorkflowExecutionContext};

mod tests;

// ── DagContext ──────────────────────────────────────────────────────────────

/// Immutable execution context threaded through all DAG step executors.
///
/// Bundles the read-only references that every step needs. The mutable
/// `DagExecutionState` is passed separately to satisfy the borrow checker.
#[derive(Clone, Copy)]
pub(crate) struct DagContext<'a> {
    pub engine: &'a ExecutionEngine,
    pub state: &'a AppState,
    pub ctx: &'a WorkflowExecutionContext,
    pub steps: &'a [WorkflowStepRow],
    pub edges: &'a [WorkflowStepEdgeRow],
    pub port_meta: &'a PortMetadata,
    pub cancel: Option<&'a CancellationToken>,
}

// ── DagExecutionState ────────────────────────────────────────────────────────

/// Mutable execution state accumulated during DAG traversal.
///
/// Bundles the six `&mut` arguments that were previously passed individually
/// to every `execute_*` function, reducing argument counts from 14–15 to ~8–9.
pub(crate) struct DagExecutionState {
    pub var_outputs: HashMap<String, JsonValue>,
    pub completed: HashMap<Uuid, StepOutput>,
    pub completed_envelopes: HashMap<Uuid, StepExecutionEnvelope>,
    /// Steps that failed during execution (step_id → error message).
    /// Used by workshop hydration to surface prior failures on page reload.
    pub failed: HashMap<Uuid, String>,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_cost_usd: f32,
    /// Overlay diff extracted from the step's container (if overlay enabled).
    /// Set by single/pipeline executors. Consumed by orchestration layer.
    pub step_overlay: Option<super::merge::types::StepOverlay>,
}

impl DagExecutionState {
    pub fn new() -> Self {
        Self {
            var_outputs: HashMap::new(),
            completed: HashMap::new(),
            completed_envelopes: HashMap::new(),
            failed: HashMap::new(),
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_cost_usd: 0.0,
            step_overlay: None,
        }
    }

    /// Create from pre-populated state (used by resume path).
    pub fn with_completed(
        completed: HashMap<Uuid, StepOutput>,
        var_outputs: HashMap<String, JsonValue>,
    ) -> Self {
        Self {
            var_outputs,
            completed,
            completed_envelopes: HashMap::new(),
            failed: HashMap::new(),
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_cost_usd: 0.0,
            step_overlay: None,
        }
    }

    /// Create from pre-populated state including envelopes (used by staging path).
    pub fn from_snapshots(
        completed: HashMap<Uuid, StepOutput>,
        var_outputs: HashMap<String, JsonValue>,
        completed_envelopes: HashMap<Uuid, StepExecutionEnvelope>,
        failed: HashMap<Uuid, String>,
    ) -> Self {
        Self {
            var_outputs,
            completed,
            completed_envelopes,
            failed,
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_cost_usd: 0.0,
            step_overlay: None,
        }
    }

    /// Accumulate token and cost values from a step execution.
    pub fn accumulate_tokens(&mut self, input: i64, output: i64, cost: f32) {
        self.total_input_tokens += input;
        self.total_output_tokens += output;
        self.total_cost_usd += cost;
    }

    /// Create a snapshot for parallel dispatch: prior completed data is cloned,
    /// output accumulators are zeroed. Each parallel task writes its own step's
    /// result into this snapshot, and results are merged back after the level completes.
    pub fn snapshot_for_parallel(&self) -> Self {
        Self {
            var_outputs: self.var_outputs.clone(),
            completed: self.completed.clone(),
            completed_envelopes: self.completed_envelopes.clone(),
            failed: HashMap::new(),
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_cost_usd: 0.0,
            step_overlay: None,
        }
    }

    /// Merge results from a parallel task's snapshot back into the main state.
    /// Only new entries (steps executed by the parallel task) are added.
    pub fn merge_parallel_result(&mut self, other: Self) {
        for (key, value) in other.var_outputs {
            self.var_outputs.entry(key).or_insert(value);
        }
        for (id, output) in other.completed {
            self.completed.entry(id).or_insert(output);
        }
        for (id, envelope) in other.completed_envelopes {
            self.completed_envelopes.entry(id).or_insert(envelope);
        }
        self.failed.extend(other.failed);
        self.total_input_tokens += other.total_input_tokens;
        self.total_output_tokens += other.total_output_tokens;
        self.total_cost_usd += other.total_cost_usd;
    }

    /// Store a step's output in the variable map, completed map, and envelope map.
    pub fn record_step_output(
        &mut self,
        step_id: Uuid,
        output: StepOutput,
        envelope: StepExecutionEnvelope,
    ) {
        if !output.variable_name.is_empty() {
            if let Some(ref structured) = output.structured_output {
                self.var_outputs
                    .insert(output.variable_name.clone(), structured.clone());
            }
        }
        self.completed_envelopes.insert(step_id, envelope);
        self.completed.insert(step_id, output);
    }
}

// ── PortMetadata ─────────────────────────────────────────────────────────────

/// Pre-fetched port metadata for all steps in a workflow.
#[derive(Clone)]
pub(crate) struct PortMetadata {
    pub step_inputs: HashMap<Uuid, Vec<StepInputRow>>,
    pub step_outputs: HashMap<Uuid, Vec<StepOutputRow>>,
    pub routing_rules: HashMap<Uuid, Vec<StepRoutingRuleRow>>,
    /// Adjacency index: target step_id → incoming edges.
    /// Built once at DAG start to avoid O(V × E) scanning in port resolution.
    pub incoming_edges: HashMap<Uuid, Vec<WorkflowStepEdgeRow>>,
}

impl PortMetadata {
    /// Construct from pre-loaded data (used by template-based execution).
    pub fn new(
        step_inputs: HashMap<Uuid, Vec<StepInputRow>>,
        step_outputs: HashMap<Uuid, Vec<StepOutputRow>>,
        routing_rules: HashMap<Uuid, Vec<StepRoutingRuleRow>>,
        incoming_edges: HashMap<Uuid, Vec<WorkflowStepEdgeRow>>,
    ) -> Self {
        Self {
            step_inputs,
            step_outputs,
            routing_rules,
            incoming_edges,
        }
    }
}

/// Build an adjacency index from edges, grouping by target step_id.
pub(crate) fn build_incoming_edge_index(
    edges: &[WorkflowStepEdgeRow],
) -> HashMap<Uuid, Vec<WorkflowStepEdgeRow>> {
    let mut index: HashMap<Uuid, Vec<WorkflowStepEdgeRow>> = HashMap::new();
    for edge in edges {
        index.entry(edge.to_step_id).or_default().push(edge.clone());
    }
    index
}

/// Pre-fetch port metadata (inputs, outputs, routing rules) for all steps,
/// and build the incoming-edge adjacency index.
pub(crate) async fn prefetch_port_metadata(
    state: &AppState,
    steps: &[WorkflowStepRow],
    edges: &[WorkflowStepEdgeRow],
) -> PortMetadata {
    let mut step_inputs: HashMap<Uuid, Vec<StepInputRow>> = HashMap::new();
    let mut step_outputs: HashMap<Uuid, Vec<StepOutputRow>> = HashMap::new();
    let mut routing_rules: HashMap<Uuid, Vec<StepRoutingRuleRow>> = HashMap::new();

    let wf_repo = &state.repos().workflows;
    for step in steps {
        if let Ok(inputs) = wf_repo.get_step_inputs(step.id).await {
            if !inputs.is_empty() {
                step_inputs.insert(step.id, inputs);
            }
        }
        if let Ok(outputs) = wf_repo.get_step_outputs(step.id).await {
            if !outputs.is_empty() {
                step_outputs.insert(step.id, outputs);
            }
        }
        if step.routing_mode.as_deref() == Some("label") {
            if let Ok(rules) = wf_repo.get_step_routing_rules(step.id).await {
                if !rules.is_empty() {
                    routing_rules.insert(step.id, rules);
                }
            }
        }
    }

    PortMetadata {
        step_inputs,
        step_outputs,
        routing_rules,
        incoming_edges: build_incoming_edge_index(edges),
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Determine the output key for a step: prefer the first output port name,
/// then `output_variable_name`, then auto-derive from the step name.
pub(crate) fn resolve_output_key(
    step: &WorkflowStepRow,
    step_outputs: &HashMap<Uuid, Vec<StepOutputRow>>,
) -> String {
    if let Some(ports) = step_outputs.get(&step.id) {
        if let Some(first) = ports.first() {
            if !first.port_name.is_empty() {
                return first.port_name.clone();
            }
        }
    }
    if let Some(ref var_name) = step.output_variable_name {
        if !var_name.is_empty() {
            return var_name.clone();
        }
    }
    // Auto-derive from step name (snake_case)
    let source = step.name.as_deref().unwrap_or(&step.execution_mode);
    to_snake_case(source)
}

/// Convert a name to snake_case for auto-derived variable names.
pub(crate) fn to_snake_case(name: &str) -> String {
    name.trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .split('_')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

/// Wrap a step output into a StepExecutionEnvelope for port-based data flow.
pub(crate) fn wrap_in_envelope(
    output: &StepOutput,
    agent: &AgentRow,
    execution_id: Uuid,
    input_tokens: i64,
    output_tokens: i64,
    cost_usd: f32,
) -> StepExecutionEnvelope {
    StepExecutionEnvelope {
        status: if output.structured_output.is_some() {
            ExecutionStatus::Success
        } else {
            ExecutionStatus::Error
        },
        data: output.structured_output.clone(),
        metadata: ExecutionMetadata {
            tokens_in: Some(input_tokens as i32),
            tokens_out: Some(output_tokens as i32),
            cost_usd: Some(cost_usd as f64),
            model: Some(agent.model_id.clone()),
            agent_id: Some(agent.id),
            ..ExecutionMetadata::new(execution_id)
        },
        error: None,
    }
}

/// Extract a display name for a step (for logging and WebSocket events).
pub(crate) fn step_display_name(step: &WorkflowStepRow) -> String {
    step.name
        .clone()
        .or_else(|| step.output_variable_name.clone())
        .unwrap_or_else(|| step.id.to_string())
}

/// The error for a step that reached an agent-bearing executor with no agent.
///
/// For a `workforce` step this is not a missing column. Dispatch routes on
/// `child_workflow_id`, which only `system_node::sync::sync_to_db` ever sets,
/// so arriving here means no design was ever synced for the node — it has no
/// pipeline and nothing to run. The old message named the null field and the
/// step's UUID, which sends whoever reads it into the database; the cause is a
/// design run that did not produce a system, and the fix is to re-dispatch the
/// node. Both dispatchers raise it, so it is written once.
pub(crate) fn missing_agent_error(step: &WorkflowStepRow) -> HubError {
    if step.execution_mode == "workforce" {
        return HubError::Internal(anyhow::anyhow!(
            "step '{}' ({}) has no designed system, so there is nothing to run. \
             Its last design run did not produce one — check the node's dispatch \
             history and re-dispatch it.",
            step_display_name(step),
            step.id,
        ));
    }
    HubError::Internal(anyhow::anyhow!(
        "step '{}' ({}) has no agent_id for mode '{}'",
        step_display_name(step),
        step.id,
        step.execution_mode
    ))
}

/// Resolve port inputs for a step, returning None if no input ports are defined
/// or if resolution fails.
///
/// Uses the pre-built `incoming_edges` index in `PortMetadata` to avoid
/// scanning all edges per step (O(1) lookup instead of O(E) scan).
pub(crate) fn resolve_step_port_inputs(
    step: &WorkflowStepRow,
    port_meta: &PortMetadata,
    completed_envelopes: &HashMap<Uuid, StepExecutionEnvelope>,
) -> Option<HashMap<String, JsonValue>> {
    let inputs = port_meta.step_inputs.get(&step.id)?;
    let incoming = port_meta
        .incoming_edges
        .get(&step.id)
        .map(|v| v.as_slice())
        .unwrap_or(&[]);
    match resolve_port_inputs(
        step.id,
        incoming,
        inputs,
        &port_meta.step_outputs,
        completed_envelopes,
    ) {
        Ok(resolved) => {
            debug!(step_id = %step.id, ports = resolved.len(), "Resolved port inputs");
            Some(resolved)
        }
        Err(e) => {
            warn!("Port resolution failed for step {}: {}", step.id, e);
            None
        }
    }
}

/// Broadcast a StepFailed event, unless the error is an AwaitingUser pause.
pub(crate) fn broadcast_step_failure_if_real(
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    workflow_id: Uuid,
    step: &WorkflowStepRow,
    error: &HubError,
) {
    if !matches!(error, HubError::AwaitingUser { .. }) {
        broadcast_workflow_event(
            state,
            ctx,
            workflow_id,
            WorkflowEventKind::StepFailed {
                step_id: step.id,
                step_name: step_display_name(step),
                error: format!("{}", error),
            },
        );
    }
}

/// Wrap step output into an agent-less StepExecutionEnvelope (for context/pass-through steps).
pub(crate) fn wrap_in_agentless_envelope(
    step_id: Uuid,
    data: Option<JsonValue>,
    duration_ms: u64,
    input_tokens: i64,
    output_tokens: i64,
    cost_usd: f32,
) -> StepExecutionEnvelope {
    StepExecutionEnvelope {
        status: if data.is_some() {
            ExecutionStatus::Success
        } else {
            ExecutionStatus::Error
        },
        data,
        metadata: ExecutionMetadata {
            execution_time_ms: duration_ms,
            tokens_in: Some(input_tokens as i32),
            tokens_out: Some(output_tokens as i32),
            cost_usd: Some(cost_usd as f64),
            ..ExecutionMetadata::new(step_id)
        },
        error: None,
    }
}
