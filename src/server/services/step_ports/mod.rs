//! Step port service: CRUD for input and output ports on workflow steps.

use uuid::Uuid;

use crate::db::traits::{CreateStepInputPort, WorkflowRepo};
use crate::db::{StepInputRow, StepOutputRow};

use super::error::ServiceError;
use super::steps::verify_step_access;
use super::validation;

pub struct CreateStepInputInput {
    pub user_id: Uuid,
    pub workflow_id: Uuid,
    pub step_id: Uuid,
    pub port_name: String,
    pub port_type: String,
    pub required: bool,
    pub default_value: Option<serde_json::Value>,
    pub description: Option<String>,
    pub json_schema: Option<serde_json::Value>,
}

pub struct CreateStepOutputInput {
    pub user_id: Uuid,
    pub workflow_id: Uuid,
    pub step_id: Uuid,
    pub port_name: String,
    pub port_type: String,
    pub json_path: String,
    pub description: Option<String>,
    pub json_schema: Option<serde_json::Value>,
}

/// List input ports for a step, verifying ownership.
pub async fn list_step_inputs(
    repo: &dyn WorkflowRepo,
    user_id: Uuid,
    workflow_id: Uuid,
    step_id: Uuid,
) -> Result<Vec<StepInputRow>, ServiceError> {
    verify_step_access(repo, user_id, workflow_id, step_id).await?;
    let rows = repo.get_step_inputs(step_id).await?;
    Ok(rows)
}

/// List output ports for a step, verifying ownership.
pub async fn list_step_outputs(
    repo: &dyn WorkflowRepo,
    user_id: Uuid,
    workflow_id: Uuid,
    step_id: Uuid,
) -> Result<Vec<StepOutputRow>, ServiceError> {
    verify_step_access(repo, user_id, workflow_id, step_id).await?;
    let rows = repo.get_step_outputs(step_id).await?;
    Ok(rows)
}

/// Create an input port on a step, verifying ownership.
pub async fn create_step_input(
    repo: &dyn WorkflowRepo,
    input: CreateStepInputInput,
) -> Result<StepInputRow, ServiceError> {
    validation::validate_required(&input.port_name, "Port name")?;
    verify_step_access(repo, input.user_id, input.workflow_id, input.step_id).await?;
    let row = repo
        .create_step_input(CreateStepInputPort {
            workflow_step_id: input.step_id,
            port_name: input.port_name,
            port_type: input.port_type,
            required: input.required,
            default_value: input.default_value,
            description: input.description,
            json_schema: input.json_schema,
        })
        .await?;
    Ok(row)
}

/// Create an output port on a step, verifying ownership.
pub async fn create_step_output(
    repo: &dyn WorkflowRepo,
    input: CreateStepOutputInput,
) -> Result<StepOutputRow, ServiceError> {
    validation::validate_required(&input.port_name, "Port name")?;
    validation::validate_required(&input.json_path, "json_path")?;
    verify_step_access(repo, input.user_id, input.workflow_id, input.step_id).await?;
    let row = repo
        .create_step_output(
            input.step_id,
            &input.port_name,
            &input.port_type,
            &input.json_path,
            input.description,
            input.json_schema,
        )
        .await?;
    Ok(row)
}

/// Delete an input port, verifying ownership.
pub async fn delete_step_input(
    repo: &dyn WorkflowRepo,
    user_id: Uuid,
    workflow_id: Uuid,
    step_id: Uuid,
    port_id: Uuid,
) -> Result<(), ServiceError> {
    verify_step_access(repo, user_id, workflow_id, step_id).await?;
    repo.delete_step_input(port_id).await?;
    Ok(())
}

/// Delete an output port, verifying ownership.
pub async fn delete_step_output(
    repo: &dyn WorkflowRepo,
    user_id: Uuid,
    workflow_id: Uuid,
    step_id: Uuid,
    port_id: Uuid,
) -> Result<(), ServiceError> {
    verify_step_access(repo, user_id, workflow_id, step_id).await?;
    repo.delete_step_output(port_id).await?;
    Ok(())
}

mod tests;
