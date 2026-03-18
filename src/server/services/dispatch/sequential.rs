//! Sequential design pipeline for board submit.
//!
//! Runs builder + designer for each step in topological order, threading
//! the previous step's handoff to the next step's builder and designer.
//! Spawned as a single background task from the board submit handler.

use uuid::Uuid;

use crate::db::{WorkflowStepEdgeRow, WorkflowStepRow};
use crate::server::services::board::instruction::NodeDispatchInstruction;
use crate::server::services::dispatch::PreviousStepHandoff;
use crate::server::state::AppState;
use crate::types::UserId;

/// Run the design pipeline sequentially in topological order.
///
/// For each step with a dispatch instruction:
/// 1. Look up the previous step's `designer_handoff` (written by its designer)
/// 2. Look up the next step's `description` (box text for forward awareness)
/// 3. Run the builder + designer with that context
/// 4. After completion, the designer's `complete_design` has written the handoff to DB
/// 5. Move to the next step
pub async fn run_sequential_design_pipeline(
    state: AppState,
    workflow_id: Uuid,
    user_id: UserId,
    instructions: Vec<NodeDispatchInstruction>,
    steps: Vec<WorkflowStepRow>,
    edges: Vec<WorkflowStepEdgeRow>,
) {
    // Build topo-sorted step order
    let topo_order = match crate::server::hub::dag::topological_sort(&steps, &edges)
    {
        Ok(order) => order,
        Err(e) => {
            tracing::warn!(
                workflow_id = %workflow_id,
                error = %e,
                "Failed to topological sort for sequential design — falling back to instruction order"
            );
            // Fall back to instruction order (already topo-sorted by the board serializer)
            instructions.iter().map(|i| i.step_id).collect()
        }
    };

    // Build step_id → topo position for sorting
    let topo_position: std::collections::HashMap<Uuid, usize> = topo_order
        .iter()
        .enumerate()
        .map(|(i, id)| (*id, i))
        .collect();

    // Sort instructions by topological position
    let mut sorted_instructions = instructions;
    sorted_instructions.sort_by_key(|i| topo_position.get(&i.step_id).copied().unwrap_or(usize::MAX));

    // Build step lookup (refreshed after each iteration)
    let mut step_map: std::collections::HashMap<Uuid, WorkflowStepRow> =
        steps.into_iter().map(|s| (s.id, s)).collect();

    for instruction in &sorted_instructions {
        // Find previous step's handoff via edges
        let parent_ids =
            crate::server::hub::dag::get_parent_steps(instruction.step_id, &edges);
        let previous_step_handoff = parent_ids.first().and_then(|pid| {
            let parent = step_map.get(pid)?;
            if parent.designer_handoff.is_empty() {
                return None;
            }
            Some(PreviousStepHandoff {
                step_name: parent.name.clone().unwrap_or_default(),
                handoff_description: parent.designer_handoff.clone(),
            })
        });

        // Find next step's box text via edges
        let child_ids =
            crate::server::hub::dag::get_child_steps(instruction.step_id, &edges);
        let next_step_text = child_ids.first().and_then(|cid| {
            let child = step_map.get(cid)?;
            if child.description.is_empty() {
                return None;
            }
            Some(child.description.clone())
        });

        // Append previous step context to the builder instruction
        let enriched_instruction = match &previous_step_handoff {
            Some(h) => format!(
                "{}\n\n<previous_step name=\"{}\">\n<handoff>\n{}\n</handoff>\n</previous_step>",
                instruction.instruction, h.step_name, h.handoff_description
            ),
            None => instruction.instruction.clone(),
        };

        // Find or create session + register task (reuse dispatch_to_builder logic)
        let session_id = super::find_or_create_builder_session(
            &state,
            instruction.step_id,
            workflow_id,
            user_id,
            &instruction.execution_mode,
        )
        .await;

        let (execution_id, _cancel_token) = state.task_registry().spawn_task(
            instruction.step_id,
            workflow_id,
            session_id,
            enriched_instruction.clone(),
        );

        // Broadcast started event
        state.broadcast_session(crate::server::ws::events::SessionEvent {
            session_id: Uuid::nil(),
            user_id: None,
            kind: crate::server::ws::events::SessionEventKind::DispatchStarted {
                execution_id,
                step_id: instruction.step_id,
                instruction: enriched_instruction.clone(),
            },
        });

        // Run synchronously (awaited, not spawned)
        crate::server::executors::dispatch::run_dispatch_task(
            state.clone(),
            execution_id,
            instruction.step_id,
            workflow_id,
            enriched_instruction,
            session_id,
            user_id,
            previous_step_handoff,
            next_step_text,
        )
        .await;

        // Re-read step from DB to pick up the designer_handoff written by complete_design
        if let Ok(Some(updated_step)) = state
            .repos()
            .workflows
            .get_step(instruction.step_id)
            .await
        {
            step_map.insert(instruction.step_id, updated_step);
        }
    }

    tracing::info!(
        workflow_id = %workflow_id,
        steps = sorted_instructions.len(),
        "Sequential design pipeline completed"
    );
}
