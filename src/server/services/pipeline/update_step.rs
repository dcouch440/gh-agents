//! Update an existing pipeline step.

use uuid::Uuid;

use crate::db::traits::WorkflowRepo;
use crate::server::hub::dag::dag_state::to_snake_case;
use crate::server::services::ServiceError;

use super::types::{PipelineContext, StepAdded, UpdateStepInput};

/// Update an existing pipeline step's name, description, execution mode,
/// or prompt template. Only provided fields are updated.
pub async fn update_step(
    repo: &dyn WorkflowRepo,
    _ctx: &PipelineContext,
    step_id: Uuid,
    input: UpdateStepInput,
) -> Result<StepAdded, ServiceError> {
    let mut step = repo
        .get_step(step_id)
        .await
        .map_err(ServiceError::Internal)?
        .ok_or_else(|| ServiceError::not_found("Pipeline step"))?;

    if let Some(ref name) = input.name {
        step.name = Some(name.clone());
        step.output_variable_name = Some(to_snake_case(name));
    }
    if let Some(ref description) = input.description {
        step.description = description.clone();
    }
    if let Some(ref execution_mode) = input.execution_mode {
        step.execution_mode = execution_mode.clone();
    }
    if let Some(ref prompt_template) = input.prompt_template {
        step.prompt_template = prompt_template.clone();
    }

    let name = step.name.clone().unwrap_or_default();
    let display_order = step.display_order;

    repo.update_step(step)
        .await
        .map_err(ServiceError::Internal)?;

    Ok(StepAdded {
        step_id,
        name,
        display_order,
    })
}
