//! Routing rule service: CRUD for label-based agent routing on workflow steps.

use uuid::Uuid;

use crate::db::traits::WorkflowRepo;
use crate::db::StepRoutingRuleRow;

use super::error::ServiceError;
use super::steps::verify_step_access;
use super::validation;

pub struct CreateRoutingRuleInput {
    pub user_id: Uuid,
    pub workflow_id: Uuid,
    pub step_id: Uuid,
    pub label_value: String,
    pub agent_id: Uuid,
    pub description: Option<String>,
    pub display_order: i32,
}

pub struct UpdateRoutingRuleInput {
    pub user_id: Uuid,
    pub workflow_id: Uuid,
    pub step_id: Uuid,
    pub rule_id: Uuid,
    pub agent_id: Option<Uuid>,
    pub description: Option<String>,
    pub display_order: Option<i32>,
}

/// List routing rules for a step, verifying ownership.
pub async fn list_routing_rules(
    repo: &dyn WorkflowRepo,
    user_id: Uuid,
    workflow_id: Uuid,
    step_id: Uuid,
) -> Result<Vec<StepRoutingRuleRow>, ServiceError> {
    verify_step_access(repo, user_id, workflow_id, step_id).await?;
    let rows = repo.get_step_routing_rules(step_id).await?;
    Ok(rows)
}

/// Create a routing rule on a step, verifying ownership.
pub async fn create_routing_rule(
    repo: &dyn WorkflowRepo,
    input: CreateRoutingRuleInput,
) -> Result<StepRoutingRuleRow, ServiceError> {
    validation::validate_required(&input.label_value, "Label value")?;
    verify_step_access(repo, input.user_id, input.workflow_id, input.step_id).await?;
    let row = repo
        .create_routing_rule(
            input.step_id,
            &input.label_value,
            input.agent_id,
            input.description,
            input.display_order,
        )
        .await?;
    Ok(row)
}

/// Update a routing rule, verifying ownership.
pub async fn update_routing_rule(
    repo: &dyn WorkflowRepo,
    input: UpdateRoutingRuleInput,
) -> Result<StepRoutingRuleRow, ServiceError> {
    verify_step_access(repo, input.user_id, input.workflow_id, input.step_id).await?;
    let row = repo
        .update_routing_rule(
            input.rule_id,
            input.agent_id,
            input.description,
            input.display_order,
        )
        .await?;
    Ok(row)
}

/// Delete a routing rule, verifying ownership.
pub async fn delete_routing_rule(
    repo: &dyn WorkflowRepo,
    user_id: Uuid,
    workflow_id: Uuid,
    step_id: Uuid,
    rule_id: Uuid,
) -> Result<(), ServiceError> {
    verify_step_access(repo, user_id, workflow_id, step_id).await?;
    repo.delete_routing_rule(rule_id).await?;
    Ok(())
}

mod tests;
