//! Document management endpoints

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::AppError;
use crate::server::auth as auth_utils;
use crate::server::services::documents;
use crate::server::state::AppState;

/// List item for documents (excludes content).
#[derive(Serialize, utoipa::ToSchema)]
pub struct DocumentListItem {
    pub id: Uuid,
    pub title: String,
    pub summary: Option<String>,
    pub ref_tag: Option<String>,
    pub tags: Option<Vec<String>>,
    pub doc_type: Option<String>,
    pub updated_at: DateTime<Utc>,
}

/// Response for a full document (includes content).
#[derive(Serialize, utoipa::ToSchema)]
pub struct DocumentResponse {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub summary: Option<String>,
    pub ref_tag: Option<String>,
    pub tags: Option<Vec<String>>,
    pub doc_type: Option<String>,
    pub session_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request body for creating a document.
#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateDocumentRequest {
    pub title: String,
    pub content: String,
    pub doc_type: Option<String>,
    pub session_id: Option<Uuid>,
    pub tags: Option<Vec<String>>,
}

/// Request body for updating a document.
#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateDocumentRequest {
    pub content: Option<String>,
    pub title: Option<String>,
    pub tags: Option<Vec<String>>,
}

/// Query parameters for document search.
#[derive(Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct DocumentSearchQuery {
    pub q: String,
}

fn document_response(doc: crate::db::DocumentRow) -> DocumentResponse {
    DocumentResponse {
        id: doc.id,
        title: doc.title,
        content: doc.content,
        summary: doc.summary,
        ref_tag: doc.ref_tag,
        tags: doc.tags,
        doc_type: doc.doc_type,
        session_id: doc.session_id,
        created_at: doc.created_at,
        updated_at: doc.updated_at,
    }
}

/// GET /api/documents - List all documents for the authenticated user.
#[utoipa::path(
    get,
    path = "/api/documents",
    tag = "Documents",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of documents", body = Vec<DocumentListItem>)
    )
)]
pub async fn list_documents(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
) -> Result<Json<Vec<DocumentListItem>>, AppError> {
    let docs = documents::list_documents(state.repos().documents.as_ref(), auth.user_id.0).await?;
    let items: Vec<DocumentListItem> = docs
        .into_iter()
        .map(|d| DocumentListItem {
            id: d.id,
            title: d.title,
            summary: d.summary,
            ref_tag: d.ref_tag,
            tags: d.tags,
            doc_type: d.doc_type,
            updated_at: d.updated_at,
        })
        .collect();
    Ok(Json(items))
}

/// GET /api/documents/search?q=query - Search documents.
#[utoipa::path(
    get,
    path = "/api/documents/search",
    tag = "Documents",
    security(("bearer_auth" = [])),
    params(DocumentSearchQuery),
    responses(
        (status = 200, description = "Search results")
    )
)]
pub async fn search_documents(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Query(query): Query<DocumentSearchQuery>,
) -> Result<Json<Vec<crate::db::DocumentSearchResult>>, AppError> {
    let results =
        documents::search_documents(state.repos().documents.as_ref(), auth.user_id.0, &query.q)
            .await?;
    Ok(Json(results))
}

/// GET /api/documents/:id - Get a full document by ID.
#[utoipa::path(
    get,
    path = "/api/documents/{id}",
    tag = "Documents",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Document ID")),
    responses(
        (status = 200, description = "Document found", body = DocumentResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn get_document(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(doc_id): Path<Uuid>,
) -> Result<Json<DocumentResponse>, AppError> {
    let doc =
        documents::get_document(state.repos().documents.as_ref(), auth.user_id.0, doc_id).await?;
    Ok(Json(document_response(doc)))
}

/// POST /api/documents - Create a new document.
#[utoipa::path(
    post,
    path = "/api/documents",
    tag = "Documents",
    security(("bearer_auth" = [])),
    request_body = CreateDocumentRequest,
    responses(
        (status = 201, description = "Document created", body = DocumentResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn create_document(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Json(request): Json<CreateDocumentRequest>,
) -> Result<(StatusCode, Json<DocumentResponse>), AppError> {
    let doc = documents::create_document(
        state.repos().documents.as_ref(),
        documents::CreateDocumentInput {
            user_id: auth.user_id.0,
            title: request.title,
            content: request.content,
            doc_type: request.doc_type,
            session_id: request.session_id,
            tags: request.tags,
        },
    )
    .await?;
    Ok((StatusCode::CREATED, Json(document_response(doc))))
}

/// PATCH /api/documents/:id - Update a document.
#[utoipa::path(
    patch,
    path = "/api/documents/{id}",
    tag = "Documents",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Document ID")),
    request_body = UpdateDocumentRequest,
    responses(
        (status = 200, description = "Updated document", body = DocumentResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn update_document(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(doc_id): Path<Uuid>,
    Json(request): Json<UpdateDocumentRequest>,
) -> Result<Json<DocumentResponse>, AppError> {
    let doc = documents::update_document(
        state.repos().documents.as_ref(),
        auth.user_id.0,
        doc_id,
        documents::UpdateDocumentInput {
            content: request.content,
            title: request.title,
            tags: request.tags,
        },
    )
    .await?;
    Ok(Json(document_response(doc)))
}

/// DELETE /api/documents/:id - Delete a document.
#[utoipa::path(
    delete,
    path = "/api/documents/{id}",
    tag = "Documents",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Document ID")),
    responses(
        (status = 204, description = "Deleted successfully"),
        (status = 404, description = "Not found")
    )
)]
pub async fn delete_document(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(doc_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    documents::delete_document(state.repos().documents.as_ref(), auth.user_id.0, doc_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
#[cfg(test)]
mod tests;
