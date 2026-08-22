//! Shared per-node status resolution for a workflow.
//!
//! One collector, two consumers:
//! - `workflow_agent::state::build_current_state` renders `<current_state>` XML
//!   for the manager agent's system prompt.
//! - `api::workflows::live_state_handlers` serializes JSON for the frontend so a
//!   page refresh can re-derive the true state of every node.
//!
//! Keeping both on one collector is what stops the UI and the agent from
//! disagreeing about what a node is doing.
//!
//! Status is assembled from four sources:
//! - **TaskRegistry** (in-memory) — active dispatch tasks → `configuring`
//! - **workflow_executions** — an active run → `running`
//! - **agent_executions** — latest dispatch result → `error` on failure
//! - **workflow_steps** — `pinned` / `run_results_summary` / `child_workflow_id`
//!   / `description`, the persisted design-time truth

use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use crate::db::{AgentExecutionRow, WorkflowStepEdgeRow, WorkflowStepRow};
use crate::server::services::ServiceError;
use crate::server::state::task_registry::{TaskEntry, TaskStatus};
use crate::server::state::AppState;

mod tests;

// ── Types ──────────────────────────────────────────────────────────────────

/// Everything needed to resolve status for every node of a workflow.
///
/// Fetched once, in batch. Holds **all** steps — callers that only care about
/// a subset (the XML renderer wants workforce steps) filter at render time.
pub struct StepStatusInputs {
    pub steps: Vec<WorkflowStepRow>,
    pub edges: Vec<WorkflowStepEdgeRow>,
    pub active_run_id: Option<Uuid>,
    pub running_step_ids: HashSet<Uuid>,
    pub latest_dispatch_by_step: HashMap<Uuid, AgentExecutionRow>,
    /// Every dispatch task for the workflow, newest-first.
    pub registry_tasks: Vec<TaskEntry>,
    /// The same tasks grouped by step, each group still newest-first.
    pub registry_tasks_by_step: HashMap<Uuid, Vec<TaskEntry>>,
}

/// One dispatch per step, resolved from the live registry or the DB fallback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveDispatch {
    pub step_id: Uuid,
    pub execution_id: Uuid,
    pub status: String,
    pub instruction: String,
    pub created_at: String,
    pub result: Option<String>,
    pub trace_len: usize,
    /// Which store this came from — decides which endpoint can fetch its trace.
    ///
    /// `registry` → `GET /dispatch/:execution_id/trace`.
    /// `persisted` → `GET /workflows/:wid/steps/:sid/dispatch/history`, because
    /// `execution_id` is an *agent_execution* id that the dispatch route cannot
    /// resolve.
    pub source: DispatchSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchSource {
    Registry,
    Persisted,
}

impl DispatchSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            DispatchSource::Registry => "registry",
            DispatchSource::Persisted => "persisted",
        }
    }
}

// ── Collection ─────────────────────────────────────────────────────────────

/// Batch-fetch every input needed to resolve node status for `workflow_id`.
pub async fn collect(
    state: &AppState,
    workflow_id: Uuid,
) -> Result<StepStatusInputs, ServiceError> {
    let wf_repo = &*state.repos().workflows;
    let ae_repo = &*state.repos().agent_executions;

    let steps = wf_repo
        .list_steps(workflow_id)
        .await
        .map_err(ServiceError::Internal)?;
    let edges = wf_repo
        .list_edges(workflow_id)
        .await
        .map_err(ServiceError::Internal)?;

    let step_ids: Vec<Uuid> = steps.iter().map(|s| s.id).collect();

    let latest_dispatches = ae_repo
        .get_latest_dispatch_executions_for_steps(&step_ids)
        .await
        .map_err(ServiceError::Internal)?;
    let latest_dispatch_by_step: HashMap<Uuid, AgentExecutionRow> = latest_dispatches
        .into_iter()
        .filter_map(|ae| ae.workflow_step_id.map(|sid| (sid, ae)))
        .collect();

    // An active run means pending *or* running — a just-created run has no
    // started_at yet, and callers still need to treat it as current.
    let active_run_id = wf_repo
        .get_active_run_for_workflow(workflow_id)
        .await
        .map_err(ServiceError::Internal)?;

    let running_step_ids: HashSet<Uuid> = match active_run_id {
        Some(run_id) => ae_repo
            .get_running_step_ids_for_run(run_id)
            .await
            .map_err(ServiceError::Internal)?
            .into_iter()
            .collect(),
        None => HashSet::new(),
    };

    // One registry scan for the whole workflow instead of one per step.
    let registry_tasks = state.task_registry().list_tasks_for_workflow(workflow_id);
    let mut registry_tasks_by_step: HashMap<Uuid, Vec<TaskEntry>> = HashMap::new();
    for task in &registry_tasks {
        registry_tasks_by_step
            .entry(task.step_id)
            .or_default()
            .push(task.clone());
    }

    Ok(StepStatusInputs {
        steps,
        edges,
        active_run_id,
        running_step_ids,
        latest_dispatch_by_step,
        registry_tasks,
        registry_tasks_by_step,
    })
}

// ── Status resolution ──────────────────────────────────────────────────────

/// Resolve the persisted, design-time status of a node.
///
/// Never encodes the live run — that is layered on top by
/// [`resolve_node_status`]. Splitting it this way lets the frontend keep a
/// baseline that survives a run starting, so a pinned node stays "completed"
/// while a new run is in flight.
///
/// Priority (highest wins):
/// 1. `error` — latest dispatch execution failed
/// 2. `completed` — pinned or has run_results_summary
/// 3. `configured` — has child_workflow_id (system node agent completed)
/// 4. `described` — has description
/// 5. `idle` — default
pub fn resolve_baseline_status(
    step: &WorkflowStepRow,
    latest_dispatch: Option<&AgentExecutionRow>,
) -> &'static str {
    if let Some(dispatch) = latest_dispatch {
        if dispatch.status == "failed" {
            return "error";
        }
    }

    if step.pinned || !step.run_results_summary.is_empty() {
        return "completed";
    }

    if step.child_workflow_id.is_some() {
        return "configured";
    }

    if !step.description.is_empty() {
        return "described";
    }

    "idle"
}

/// Resolve real-time status for a workflow node.
///
/// Priority (highest wins):
/// 1. `configuring` — TaskRegistry has a Running task for this step
/// 2. `running` — the step is executing in the active run
/// 3. everything else — see [`resolve_baseline_status`]
///
/// Pure function — takes pre-fetched data, no DB access.
pub fn resolve_node_status(
    step: &WorkflowStepRow,
    active_tasks: &[TaskEntry],
    latest_dispatch: Option<&AgentExecutionRow>,
    is_running: bool,
) -> &'static str {
    // 1. Active dispatch task → configuring
    if active_tasks.iter().any(|t| t.status == TaskStatus::Running) {
        return "configuring";
    }

    // 2. Active runtime execution → running
    if is_running {
        return "running";
    }

    resolve_baseline_status(step, latest_dispatch)
}

// ── Dispatch merge ─────────────────────────────────────────────────────────

/// Reduce the live registry and the DB fallback to at most one dispatch per step.
///
/// The in-memory registry is authoritative while the process lives; after a
/// restart (or a `cleanup_before` GC) it is empty and the persisted
/// `agent_executions` row is all that remains. Pure and DB-free so it can be
/// unit-tested without a database.
///
/// `registry` must be newest-first — the first entry seen for a step wins.
pub fn merge_dispatches(
    registry: &[TaskEntry],
    persisted: &[AgentExecutionRow],
) -> Vec<LiveDispatch> {
    let mut out: Vec<LiveDispatch> = Vec::new();
    let mut seen: HashSet<Uuid> = HashSet::new();

    for task in registry {
        if !seen.insert(task.step_id) {
            continue;
        }
        out.push(LiveDispatch {
            step_id: task.step_id,
            execution_id: task.execution_id,
            status: task.status.as_str().to_string(),
            instruction: task.instruction.clone(),
            created_at: task.created_at.to_rfc3339(),
            result: task.result.clone(),
            trace_len: task.trace.len(),
            source: DispatchSource::Registry,
        });
    }

    for ae in persisted {
        let step_id = match ae.workflow_step_id {
            Some(id) => id,
            None => continue,
        };
        if !seen.insert(step_id) {
            continue;
        }
        let trace_len = ae
            .trace
            .as_ref()
            .and_then(|t| t.as_array())
            .map(|a| a.len())
            .unwrap_or(0);
        out.push(LiveDispatch {
            step_id,
            execution_id: ae.id,
            status: ae.status.clone(),
            instruction: ae.input.clone(),
            created_at: ae.started_at.to_rfc3339(),
            result: ae.output.clone(),
            trace_len,
            source: DispatchSource::Persisted,
        });
    }

    out
}

/// True when any dispatch is still running — drives the Generate button.
pub fn is_generating(dispatches: &[LiveDispatch]) -> bool {
    dispatches.iter().any(|d| d.status == "running")
}
