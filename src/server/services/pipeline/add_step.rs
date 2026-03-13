//! Add a step to a pipeline.

use uuid::Uuid;

use crate::db::traits::WorkflowRepo;
use crate::db::WorkflowStepRow;
use crate::server::hub::dag::dag_state::to_snake_case;
use crate::server::services::steps::generate_ref_id;
use crate::server::services::ServiceError;

use super::recompute::recompute_execution_order;
use super::types::{AddStepInput, ExecutionOrderEntry, PipelineContext, StepAdded};

/// Add a step to the pipeline. Auto-creates the pipeline if it doesn't exist.
///
/// Returns the created step and the recomputed execution sequence.
pub async fn add_step(
    repo: &dyn WorkflowRepo,
    ctx: &PipelineContext,
    user_id: Uuid,
    input: AddStepInput,
) -> Result<(StepAdded, Vec<ExecutionOrderEntry>), ServiceError> {
    // Ensure pipeline exists
    let pipeline = super::create::create_pipeline(repo, ctx, user_id).await?;
    let pipeline_id = pipeline.pipeline_id;

    // Fetch existing steps (used for display_order + ref_id generation)
    let existing_steps = repo
        .list_steps(pipeline_id)
        .await
        .map_err(ServiceError::Internal)?;

    let display_order = match input.display_order {
        Some(order) => order,
        None => {
            let max_order = existing_steps
                .iter()
                .map(|s| s.display_order)
                .max()
                .unwrap_or(0);
            max_order + 1
        }
    };

    let ref_id = generate_ref_id(&existing_steps, &input.execution_mode);

    // Build the child step
    let output_variable_name = input
        .output_variable_name
        .unwrap_or_else(|| to_snake_case(&input.name));

    let child_step = WorkflowStepRow {
        id: Uuid::new_v4(),
        workflow_id: pipeline_id,
        agent_id: input.agent_id,
        execution_mode: input.execution_mode,
        agent_execution_mode: None,
        for_each_ref: None,
        prompt_template_id: None,
        prompt_template: input.prompt_template.unwrap_or_default(),
        output_schema_id: None,
        output_variable_name: Some(output_variable_name),
        interactive_agent_id: None,
        for_each_label_field: None,
        room_id: None,
        routing_mode: None,
        routing_field: None,
        display_order,
        version: 1,
        reasoning_trace: false,
        verification_agent_ids: None,
        position_x: Some(display_order as f64 * 200.0),
        position_y: Some(0.0),
        width: None,
        height: None,
        name: Some(input.name.clone()),
        system_prompt_suffix: None,
        visible: true,
        description: input.description,
        board_context_cache: String::new(),
        board_context_updated_at: None,
        goal_summary: String::new(),
        goal_summary_updated_at: None,
        sub_workflow_template_id: None,
        child_workflow_id: None,
        ref_id: Some(ref_id),
        pinned: false,
        run_results_summary: String::new(),
    };

    let created = repo
        .create_step(child_step)
        .await
        .map_err(ServiceError::Internal)?;

    // Recompute execution order
    let sequence = recompute_execution_order(repo, pipeline_id).await?;

    Ok((
        StepAdded {
            step_id: created.id,
            name: input.name,
            display_order,
        },
        sequence,
    ))
}
