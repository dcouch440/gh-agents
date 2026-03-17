//! Document service: create, read, update, delete, search documents.

use uuid::Uuid;

use crate::db::traits::DocumentRepo;
use crate::db::{DocumentRow, DocumentSearchResult};

use super::error::ServiceError;
use super::validation;

/// Input for creating a new document.
pub struct CreateDocumentInput {
    pub user_id: Uuid,
    pub title: String,
    pub content: String,
    pub doc_type: Option<String>,
    pub session_id: Option<Uuid>,
    pub tags: Option<Vec<String>>,
}

/// Input for updating an existing document.
pub struct UpdateDocumentInput {
    pub content: Option<String>,
    pub title: Option<String>,
    pub tags: Option<Vec<String>>,
}

/// Verify the caller owns this document.
async fn verify_ownership(
    repo: &dyn DocumentRepo,
    user_id: Uuid,
    doc_id: Uuid,
) -> Result<DocumentRow, ServiceError> {
    super::ownership::fetch_and_check_owner(
        || repo.get_document(doc_id),
        user_id,
        |d| d.user_id,
        "Document",
    )
    .await
}

/// Create a new document.
pub async fn create_document(
    repo: &dyn DocumentRepo,
    input: CreateDocumentInput,
) -> Result<DocumentRow, ServiceError> {
    validation::validate_name(&input.title, "title")?;
    if input.content.len() > crate::constants::MAX_DESCRIPTION_LENGTH {
        return Err(ServiceError::validation("Content exceeds maximum length"));
    }

    let row = repo
        .create_document(crate::db::traits::CreateDocumentInput {
            user_id: input.user_id,
            session_id: input.session_id,
            title: input.title,
            content: input.content,
            doc_type: input.doc_type.unwrap_or_else(|| "architecture".to_string()),
            ref_tag: String::new(),
            tags: input.tags.unwrap_or_default(),
        })
        .await?;

    Ok(row)
}

/// Get a single document by ID, verifying ownership.
pub async fn get_document(
    repo: &dyn DocumentRepo,
    user_id: Uuid,
    doc_id: Uuid,
) -> Result<DocumentRow, ServiceError> {
    verify_ownership(repo, user_id, doc_id).await
}

/// List documents for a user.
pub async fn list_documents(
    repo: &dyn DocumentRepo,
    user_id: Uuid,
) -> Result<Vec<DocumentRow>, ServiceError> {
    let rows = repo.list_documents(user_id).await?;
    Ok(rows)
}

/// Search documents for a user.
pub async fn search_documents(
    repo: &dyn DocumentRepo,
    user_id: Uuid,
    query: &str,
) -> Result<Vec<DocumentSearchResult>, ServiceError> {
    let results = repo.search_documents(user_id, query).await?;
    Ok(results)
}

/// Update an existing document (partial update).
pub async fn update_document(
    repo: &dyn DocumentRepo,
    user_id: Uuid,
    doc_id: Uuid,
    input: UpdateDocumentInput,
) -> Result<DocumentRow, ServiceError> {
    verify_ownership(repo, user_id, doc_id).await?;

    let updated = repo
        .update_document(doc_id, input.content, input.title, input.tags)
        .await?;

    Ok(updated)
}

/// Delete a document by ID, verifying ownership.
pub async fn delete_document(
    repo: &dyn DocumentRepo,
    user_id: Uuid,
    doc_id: Uuid,
) -> Result<(), ServiceError> {
    verify_ownership(repo, user_id, doc_id).await?;

    repo.delete_document(doc_id).await?;
    Ok(())
}

mod tests;
