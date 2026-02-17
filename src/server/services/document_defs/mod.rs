//! Document definition service: CRUD for protocol document definitions on workflow steps.

use uuid::Uuid;

use crate::db::traits::WorkflowRepo;
use crate::db::ProtocolDocumentDefRow;

use super::error::ServiceError;
use super::steps::verify_step_access;

pub struct CreateDocumentDefInput {
    pub user_id: Uuid,
    pub workflow_id: Uuid,
    pub step_id: Uuid,
    pub name: String,
    pub description: String,
    pub target_length: i32,
    pub display_order: i32,
}

pub struct UpdateDocumentDefInput {
    pub user_id: Uuid,
    pub workflow_id: Uuid,
    pub step_id: Uuid,
    pub def_id: Uuid,
    pub name: Option<String>,
    pub description: Option<String>,
    pub target_length: Option<i32>,
}

/// Info about a deleted document def, for the consistency scanner.
pub struct DeletedDocumentDefInfo {
    pub def_name: String,
    pub step_name: String,
}

/// List document definitions for a step, verifying ownership.
pub async fn list_document_defs(
    repo: &dyn WorkflowRepo,
    user_id: Uuid,
    workflow_id: Uuid,
    step_id: Uuid,
) -> Result<Vec<ProtocolDocumentDefRow>, ServiceError> {
    verify_step_access(repo, user_id, workflow_id, step_id).await?;
    let rows = repo.list_document_defs(step_id).await?;
    Ok(rows)
}

/// Create a document definition on a step, verifying ownership.
pub async fn create_document_def(
    repo: &dyn WorkflowRepo,
    input: CreateDocumentDefInput,
) -> Result<ProtocolDocumentDefRow, ServiceError> {
    verify_step_access(repo, input.user_id, input.workflow_id, input.step_id).await?;

    let def = ProtocolDocumentDefRow {
        id: Uuid::new_v4(),
        step_id: Some(input.step_id),
        name: input.name,
        description: input.description,
        target_length: input.target_length,
        display_order: input.display_order,
        created_at: chrono::Utc::now(),
        protocol_id: None,
        document_id: None,
        agent_roster_entry_id: None,
    };

    let row = repo.create_document_def(def).await?;
    Ok(row)
}

/// Update a document definition (partial), verifying ownership.
pub async fn update_document_def(
    repo: &dyn WorkflowRepo,
    input: UpdateDocumentDefInput,
) -> Result<ProtocolDocumentDefRow, ServiceError> {
    verify_step_access(repo, input.user_id, input.workflow_id, input.step_id).await?;

    // Fetch existing to merge partial update
    let defs = repo.list_document_defs(input.step_id).await?;
    let existing = defs
        .into_iter()
        .find(|d| d.id == input.def_id)
        .ok_or_else(|| ServiceError::not_found("Document Definition"))?;

    let name = input.name.unwrap_or(existing.name);
    let description = input.description.unwrap_or(existing.description);
    let target_length = input.target_length.unwrap_or(existing.target_length);

    let row = repo
        .update_document_def(input.def_id, name, description, target_length)
        .await?;
    Ok(row)
}

/// Delete a document definition, verifying ownership.
/// Returns info about the deleted def for the consistency scanner.
pub async fn delete_document_def(
    repo: &dyn WorkflowRepo,
    user_id: Uuid,
    workflow_id: Uuid,
    step_id: Uuid,
    def_id: Uuid,
) -> Result<DeletedDocumentDefInfo, ServiceError> {
    verify_step_access(repo, user_id, workflow_id, step_id).await?;

    // Load name + step name before deleting (for consistency scanner)
    let def_name = repo
        .list_document_defs(step_id)
        .await
        .ok()
        .and_then(|defs| defs.into_iter().find(|d| d.id == def_id))
        .map(|d| d.name)
        .unwrap_or_default();

    let step_name = repo
        .get_step(step_id)
        .await
        .ok()
        .flatten()
        .and_then(|s| s.name)
        .unwrap_or_default();

    repo.delete_document_def(def_id).await?;

    Ok(DeletedDocumentDefInfo {
        def_name,
        step_name,
    })
}

mod tests;
