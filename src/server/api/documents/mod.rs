//! Document management endpoints

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::constants::{MAX_DESCRIPTION_LENGTH, MAX_TITLE_LENGTH};
use crate::server::auth as auth_utils;
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
pub async fn list_documents(State(state): State<AppState>, auth: auth_utils::AuthUser) -> Result<Json<Vec<DocumentListItem>>, StatusCode> {
    let doc_repo = state.doc_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let docs = doc_repo.list_documents(auth.user_id.0).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
pub async fn search_documents(State(state): State<AppState>, auth: auth_utils::AuthUser, Query(query): Query<DocumentSearchQuery>) -> Result<Json<Vec<crate::db::DocumentSearchResult>>, StatusCode> {
    let doc_repo = state.doc_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let results = doc_repo.search_documents(auth.user_id.0, &query.q).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
pub async fn get_document(State(state): State<AppState>, auth: auth_utils::AuthUser, Path(doc_id): Path<Uuid>) -> Result<Json<DocumentResponse>, StatusCode> {
    let doc_repo = state.doc_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let doc = doc_repo.get_document(doc_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;

    // Verify ownership
    if doc.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(Json(DocumentResponse {
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
    }))
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
pub async fn create_document(State(state): State<AppState>, auth: auth_utils::AuthUser, Json(request): Json<CreateDocumentRequest>) -> Result<(StatusCode, Json<DocumentResponse>), StatusCode> {
    if request.title.trim().is_empty() || request.title.len() > MAX_TITLE_LENGTH {
        return Err(StatusCode::BAD_REQUEST);
    }
    if request.content.len() > MAX_DESCRIPTION_LENGTH {
        return Err(StatusCode::BAD_REQUEST);
    }

    let doc_repo = state.doc_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let doc = doc_repo
        .create_document(
            auth.user_id.0,
            request.session_id,
            request.title,
            request.content,
            request.doc_type.unwrap_or_else(|| "architecture".to_string()),
            String::new(),
            request.tags.unwrap_or_default(),
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((
        StatusCode::CREATED,
        Json(DocumentResponse {
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
        }),
    ))
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
) -> Result<Json<DocumentResponse>, StatusCode> {
    let doc_repo = state.doc_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    // Verify ownership
    let existing = doc_repo.get_document(doc_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;

    if existing.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }

    let doc = doc_repo
        .update_document(doc_id, request.content, request.title, request.tags)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(DocumentResponse {
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
    }))
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
pub async fn delete_document(State(state): State<AppState>, auth: auth_utils::AuthUser, Path(doc_id): Path<Uuid>) -> Result<StatusCode, StatusCode> {
    let doc_repo = state.doc_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    // Verify ownership
    let existing = doc_repo.get_document(doc_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;

    if existing.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }

    doc_repo.delete_document(doc_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}
#[cfg(test)]
mod tests;
