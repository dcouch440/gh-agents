//! Step readiness computation for workshop execution.

use uuid::Uuid;

use crate::db::{WorkflowStepEdgeRow, WorkflowStepRow};

use super::super::dag_state::{build_incoming_edge_index, DagExecutionState};
use super::super::utils::{check_step_readiness, StepReadiness};

/// Compute which steps are ready to execute given current completion state.
///
/// Iterates all steps not yet completed and returns those whose upstream
/// dependencies are fully satisfied.
pub(crate) fn next_executable_steps(
    steps: &[WorkflowStepRow],
    edges: &[WorkflowStepEdgeRow],
    dag_state: &DagExecutionState,
) -> Vec<Uuid> {
    let incoming_index = build_incoming_edge_index(edges);
    steps
        .iter()
        .filter(|s| !dag_state.completed.contains_key(&s.id))
        .filter(|s| {
            let incoming = incoming_index
                .get(&s.id)
                .map(|v| v.as_slice())
                .unwrap_or(&[]);
            check_step_readiness(
                s.id,
                incoming,
                &dag_state.completed,
                &dag_state.completed_envelopes,
            ) == StepReadiness::Ready
        })
        .map(|s| s.id)
        .collect()
}
