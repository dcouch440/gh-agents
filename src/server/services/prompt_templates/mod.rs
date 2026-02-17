//! Prompt template service: create, read, update, delete prompt templates.

use uuid::Uuid;

use crate::db::traits::PromptTemplateRepo;
use crate::db::PromptTemplateRow;

use super::error::ServiceError;
use super::validation;

/// Verify strict ownership: the template must exist AND have a `user_id` that
/// matches the caller. System templates (`user_id = None`) are NOT editable, so
/// they fail this check.
async fn verify_ownership(
    repo: &dyn PromptTemplateRepo,
    user_id: Uuid,
    template_id: Uuid,
) -> Result<PromptTemplateRow, ServiceError> {
    let template = repo
        .get_prompt_template(template_id)
        .await?
        .ok_or_else(|| ServiceError::not_found("Prompt template"))?;
    match template.user_id {
        Some(owner) if owner != user_id => return Err(ServiceError::not_found("Prompt template")),
        None => return Err(ServiceError::not_found("Prompt template")),
        _ => {}
    }
    Ok(template)
}

/// Create a new prompt template owned by the given user.
pub async fn create_prompt_template(
    repo: &dyn PromptTemplateRepo,
    user_id: Uuid,
    name: String,
    content: String,
) -> Result<PromptTemplateRow, ServiceError> {
    validation::validate_name(&name, "name")?;
    validation::validate_prompt(&content)?;
    let row = repo
        .create_prompt_template(Some(user_id), name, content)
        .await?;
    Ok(row)
}

/// Get a single prompt template by ID.
///
/// System templates (`user_id = None`) are visible to all users.
/// User-owned templates are only visible to their owner.
pub async fn get_prompt_template(
    repo: &dyn PromptTemplateRepo,
    user_id: Uuid,
    template_id: Uuid,
) -> Result<PromptTemplateRow, ServiceError> {
    let template = repo
        .get_prompt_template(template_id)
        .await?
        .ok_or_else(|| ServiceError::not_found("Prompt template"))?;
    if let Some(owner) = template.user_id {
        if owner != user_id {
            return Err(ServiceError::not_found("Prompt template"));
        }
    }
    Ok(template)
}

/// List prompt templates for a user.
pub async fn list_prompt_templates(
    repo: &dyn PromptTemplateRepo,
    user_id: Uuid,
) -> Result<Vec<PromptTemplateRow>, ServiceError> {
    let rows = repo.list_prompt_templates(user_id).await?;
    Ok(rows)
}

/// Update an existing prompt template (partial update).
///
/// Only user-owned templates can be updated; system templates are read-only.
pub async fn update_prompt_template(
    repo: &dyn PromptTemplateRepo,
    user_id: Uuid,
    template_id: Uuid,
    name: Option<String>,
    content: Option<String>,
) -> Result<PromptTemplateRow, ServiceError> {
    verify_ownership(repo, user_id, template_id).await?;

    if let Some(ref n) = name {
        validation::validate_name(n, "name")?;
    }
    if let Some(ref c) = content {
        validation::validate_prompt(c)?;
    }

    let row = repo
        .update_prompt_template(template_id, name, content)
        .await?;
    Ok(row)
}

/// Delete a prompt template by ID.
///
/// Only user-owned templates can be deleted; system templates are read-only.
pub async fn delete_prompt_template(
    repo: &dyn PromptTemplateRepo,
    user_id: Uuid,
    template_id: Uuid,
) -> Result<(), ServiceError> {
    verify_ownership(repo, user_id, template_id).await?;
    repo.delete_prompt_template(template_id).await?;
    Ok(())
}

mod tests;
