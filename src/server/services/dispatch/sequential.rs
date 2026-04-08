//! Sequential design pipeline for board submit.
//!
//! Runs builder + designer for each step in topological order, threading
//! the previous step's handoff to the next step's builder and designer.
//! After each step, if the designer updated the handoff, downstream steps
//! are re-designed (designer-only, no builder) — propagating until a step
//! absorbs the change without updating its own handoff.
//!
//! Spawned as a single background task from the board submit handler.

use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use crate::db::{WorkflowStepEdgeRow, WorkflowStepRow};
use crate::server::services::board::instruction::NodeDispatchInstruction;
use crate::server::services::dispatch::PreviousStepHandoff;
use crate::server::state::AppState;
use crate::server::ws::events::{SessionEvent, SessionEventKind};
use crate::types::UserId;

/// Run the design pipeline sequentially in topological order.
///
/// For each step in the workflow (topo order):
/// - If it has a dispatch instruction → run builder + designer
/// - If a parent's handoff changed → run designer-only (propagation re-design)
/// - Otherwise → skip
///
/// After each step, compare old vs new handoff. If changed, downstream
/// steps will be re-designed when they're reached in the topo order.
pub async fn run_sequential_design_pipeline(
    state: AppState,
    workflow_id: Uuid,
    user_id: UserId,
    instructions: Vec<NodeDispatchInstruction>,
    steps: Vec<WorkflowStepRow>,
    edges: Vec<WorkflowStepEdgeRow>,
) {
    // Auto-checkpoint before destructive design pipeline
    if let Err(e) = crate::server::services::workflow_agent::versions::save_version(
        workflow_id,
        user_id.0,
        None,
        "auto_pre_generate",
        &state,
    )
    .await
    {
        tracing::warn!(
            workflow_id = %workflow_id,
            error = %e,
            "Failed to auto-checkpoint before Generate — continuing anyway"
        );
    }

    // Build topo-sorted levels — steps in the same level have no edges
    // between them and can run in parallel.
    let levels = match crate::server::hub::dag::topological_sort_levels(&steps, &edges) {
        Ok(lvls) => lvls,
        Err(e) => {
            tracing::warn!(
                workflow_id = %workflow_id,
                error = %e,
                "Failed to topological sort for design pipeline — falling back to instruction order"
            );
            vec![instructions.iter().map(|i| i.step_id).collect()]
        }
    };

    // Build instruction lookup by step_id
    let instruction_map: HashMap<Uuid, &NodeDispatchInstruction> =
        instructions.iter().map(|i| (i.step_id, i)).collect();

    // Build step lookup (refreshed after each level completes)
    let mut step_map: HashMap<Uuid, WorkflowStepRow> =
        steps.into_iter().map(|s| (s.id, s)).collect();

    // Track which steps had their handoff changed (for propagation)
    let mut handoff_changed: HashSet<Uuid> = HashSet::new();
    let mut dispatched_count: usize = 0;
    let mut propagated_count: usize = 0;

    for level in &levels {
        // Partition level steps into dispatch vs propagate vs skip.
        // Capture old handoffs before any tasks run.
        let mut level_tasks: Vec<(Uuid, Option<NodeDispatchInstruction>, String)> = Vec::new();

        for step_id in level {
            let has_instruction = instruction_map.contains_key(step_id);
            let parent_ids = crate::server::hub::dag::get_parent_steps(*step_id, &edges);
            let parent_changed = parent_ids.iter().any(|pid| handoff_changed.contains(pid));

            if !has_instruction && !parent_changed {
                continue;
            }

            let old_handoff = step_map
                .get(step_id)
                .map(|s| s.designer_handoff.clone())
                .unwrap_or_default();

            if has_instruction {
                let instruction = instruction_map[step_id].clone();
                level_tasks.push((*step_id, Some(instruction), old_handoff));
            } else {
                tracing::info!(
                    step_id = %step_id,
                    "Propagating re-design: upstream handoff changed"
                );
                level_tasks.push((*step_id, None, old_handoff));
            }
        }

        if level_tasks.is_empty() {
            continue;
        }

        // Run all tasks in this level in parallel
        if level_tasks.len() == 1 {
            // Single task — run directly (no spawn overhead)
            let (step_id, instruction, _) = &level_tasks[0];
            let parent_ids = crate::server::hub::dag::get_parent_steps(*step_id, &edges);
            let previous_step_handoff = lookup_previous_handoff(&parent_ids, &step_map);

            if let Some(instr) = instruction {
                run_system_node_dispatch(
                    &state,
                    *step_id,
                    workflow_id,
                    user_id,
                    instr,
                    previous_step_handoff,
                )
                .await;
                dispatched_count += 1;
            } else {
                run_system_node_propagation(
                    &state,
                    *step_id,
                    workflow_id,
                    user_id,
                    previous_step_handoff,
                )
                .await;
                propagated_count += 1;
            }
        } else {
            // Multiple tasks — run in parallel via JoinSet
            let mut join_set = tokio::task::JoinSet::new();

            for (step_id, instruction, _) in &level_tasks {
                let task_state = state.clone();
                let task_step_id = *step_id;
                let task_workflow_id = workflow_id;
                let task_user_id = user_id;
                let parent_ids = crate::server::hub::dag::get_parent_steps(*step_id, &edges);
                let task_handoff = lookup_previous_handoff(&parent_ids, &step_map);

                if let Some(instr) = instruction.clone() {
                    join_set.spawn(async move {
                        run_system_node_dispatch(
                            &task_state,
                            task_step_id,
                            task_workflow_id,
                            task_user_id,
                            &instr,
                            task_handoff,
                        )
                        .await;
                        (task_step_id, true)
                    });
                } else {
                    join_set.spawn(async move {
                        run_system_node_propagation(
                            &task_state,
                            task_step_id,
                            task_workflow_id,
                            task_user_id,
                            task_handoff,
                        )
                        .await;
                        (task_step_id, false)
                    });
                }
            }

            while let Some(join_result) = join_set.join_next().await {
                match join_result {
                    Ok((_, was_dispatch)) => {
                        if was_dispatch {
                            dispatched_count += 1;
                        } else {
                            propagated_count += 1;
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            error = %e,
                            "Design pipeline task panicked"
                        );
                    }
                }
            }
        }

        // After level completes: re-read all dispatched/propagated steps and
        // check handoff changes for downstream propagation.
        for (step_id, _, old_handoff) in &level_tasks {
            match state.repos().workflows.get_step(*step_id).await {
                Ok(Some(updated_step)) => {
                    let new_handoff = updated_step.designer_handoff.clone();
                    step_map.insert(*step_id, updated_step);

                    if *old_handoff != new_handoff {
                        tracing::info!(
                            step_id = %step_id,
                            old_len = old_handoff.len(),
                            new_len = new_handoff.len(),
                            "Handoff changed — downstream steps will be re-designed"
                        );
                        handoff_changed.insert(*step_id);
                    } else {
                        tracing::info!(
                            step_id = %step_id,
                            handoff_len = new_handoff.len(),
                            "Handoff unchanged after design — no downstream propagation"
                        );
                    }
                }
                Ok(None) => {
                    tracing::warn!(
                        step_id = %step_id,
                        "Step not found after design — cannot check handoff propagation"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        step_id = %step_id,
                        error = %e,
                        "Failed to re-read step after design — cannot check handoff propagation"
                    );
                }
            }
        }
    }

    tracing::info!(
        workflow_id = %workflow_id,
        dispatched = dispatched_count,
        propagated = propagated_count,
        "Sequential design pipeline completed"
    );
}

// ── Helpers ──────────────────────────────────────────────────────────────

/// Look up handoffs from ALL parent steps (fan-in support).
fn lookup_previous_handoff(
    parent_ids: &[Uuid],
    step_map: &HashMap<Uuid, WorkflowStepRow>,
) -> Vec<PreviousStepHandoff> {
    parent_ids
        .iter()
        .filter_map(|pid| {
            let parent = step_map.get(pid)?;
            if parent.designer_handoff.is_empty() {
                return None;
            }
            Some(PreviousStepHandoff {
                step_name: parent.name.clone().unwrap_or_default(),
                handoff_description: parent.designer_handoff.clone(),
            })
        })
        .collect()
}

/// Run the system node agent for a step with an instruction.
///
/// Replaces `run_builder_and_designer` — the system node agent handles both
/// configuration (builder) and prompt design (designer) in a single pass.
async fn run_system_node_dispatch(
    state: &AppState,
    step_id: Uuid,
    workflow_id: Uuid,
    user_id: UserId,
    instruction: &NodeDispatchInstruction,
    previous_step_handoff: Vec<PreviousStepHandoff>,
) {
    // Enrich instruction with previous step context
    let enriched_instruction =
        enrich_with_previous_step(&instruction.instruction, &previous_step_handoff);

    let session_id =
        super::find_or_create_builder_session(state, step_id, workflow_id, user_id, "system_agent")
            .await;

    let (execution_id, _cancel_token) = state.task_registry().spawn_task(
        step_id,
        workflow_id,
        session_id,
        enriched_instruction.clone(),
    );

    state.broadcast_session(SessionEvent {
        session_id: Uuid::nil(),
        user_id: None,
        kind: SessionEventKind::DispatchStarted {
            execution_id,
            step_id,
            instruction: enriched_instruction.clone(),
        },
    });

    crate::server::executors::dispatch::system_node::run_system_node_task(
        state.clone(),
        execution_id,
        step_id,
        workflow_id,
        enriched_instruction,
        session_id,
        user_id,
    )
    .await;
}

/// Re-run the system node agent when an upstream step's description changed.
///
/// Replaces `run_propagation_redesign` — formats an instruction with the
/// updated upstream context and re-runs the system node agent.
async fn run_system_node_propagation(
    state: &AppState,
    step_id: Uuid,
    workflow_id: Uuid,
    user_id: UserId,
    previous_step_handoff: Vec<PreviousStepHandoff>,
) {
    // Use the step's box text (prompt_template) as context for the re-run.
    // prompt_template is the user's raw canvas text; description may be empty.
    let step = state
        .repos()
        .workflows
        .get_step(step_id)
        .await
        .ok()
        .flatten();

    let task_text = step
        .as_ref()
        .map(|s| {
            if !s.prompt_template.is_empty() {
                s.prompt_template.clone()
            } else {
                s.description.clone()
            }
        })
        .unwrap_or_default();

    if task_text.is_empty() && previous_step_handoff.is_empty() {
        tracing::debug!(
            step_id = %step_id,
            "Skipping system node propagation — no task text or upstream context"
        );
        return;
    }

    let base_instruction = format!(
        "The upstream step changed what it produces.\n\n<task>\n{}\n</task>",
        task_text
    );
    let instruction_text = enrich_with_previous_step(&base_instruction, &previous_step_handoff);

    let session_id =
        super::find_or_create_builder_session(state, step_id, workflow_id, user_id, "system_agent")
            .await;

    let (execution_id, _cancel_token) = state.task_registry().spawn_task(
        step_id,
        workflow_id,
        session_id,
        instruction_text.clone(),
    );

    state.broadcast_session(SessionEvent {
        session_id: Uuid::nil(),
        user_id: None,
        kind: SessionEventKind::DispatchStarted {
            execution_id,
            step_id,
            instruction: instruction_text.clone(),
        },
    });

    crate::server::executors::dispatch::system_node::run_system_node_task(
        state.clone(),
        execution_id,
        step_id,
        workflow_id,
        instruction_text,
        session_id,
        user_id,
    )
    .await;
}

/// Enrich an instruction with `<previous_step>` context blocks.
fn enrich_with_previous_step(
    instruction: &str,
    previous_step_handoff: &[PreviousStepHandoff],
) -> String {
    if previous_step_handoff.is_empty() {
        return instruction.to_string();
    }

    let blocks: Vec<String> = previous_step_handoff
        .iter()
        .map(|h| {
            format!(
                "<previous_step name=\"{}\">\n{}\n</previous_step>",
                h.step_name, h.handoff_description
            )
        })
        .collect();

    format!("{}\n\n{}", instruction, blocks.join("\n\n"))
}
