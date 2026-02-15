//! Document persistence logic for the documenter pipeline.
//!
//! Handles determining how generated documents should be persisted (update,
//! create-and-link, or skip) and executing the actual database operations.

use tracing::warn;
use uuid::Uuid;

use crate::server::state::AppState;

/// What the write phase should do with the generated document content.
#[derive(Debug, PartialEq)]
pub(crate) enum DocumentPersistAction {
    /// A document already exists — update it in place.
    Update(Uuid),
    /// No document exists but a def is available — create a new document and link it.
    CreateAndLink(Uuid),
    /// Neither document nor def available — content cannot be persisted.
    Skip,
}

/// Determine how to persist a write phase result based on the current state
/// of the document definition.
///
/// - `document_id` — the existing linked document (from `protocol_document_defs.document_id`)
/// - `def_id` — the document definition row id (for linking a newly created document)
pub(crate) fn determine_persist_action(
    document_id: Option<Uuid>,
    def_id: Option<Uuid>,
) -> DocumentPersistAction {
    if let Some(did) = document_id {
        DocumentPersistAction::Update(did)
    } else if let Some(did) = def_id {
        DocumentPersistAction::CreateAndLink(did)
    } else {
        DocumentPersistAction::Skip
    }
}

/// Context needed to persist a generated document to the database.
pub(super) struct DocumentPersistContext {
    pub document_id: Option<Uuid>,
    pub def_id: Option<Uuid>,
    pub doc_name: String,
    pub user_id: Uuid,
    pub workflow_id: Uuid,
    pub step_id: Uuid,
    pub run_id: Uuid,
}

/// Persist the generated document content to the database.
///
/// Handles three cases based on the current state of the document definition:
/// update an existing document, create a new one and link it, or skip.
pub(super) async fn persist_document_content(
    state: &AppState,
    ctx: &DocumentPersistContext,
    content: &str,
) {
    let doc_repo = match state.doc_repo() {
        Some(repo) => repo,
        None => return,
    };

    let persisted_doc_id = match determine_persist_action(ctx.document_id, ctx.def_id) {
        DocumentPersistAction::Update(did) => {
            let _ = doc_repo
                .update_document(did, Some(content.to_string()), None, None)
                .await;
            Some(did)
        }
        DocumentPersistAction::CreateAndLink(did) => {
            match doc_repo
                .create_workflow_document(
                    ctx.user_id,
                    ctx.doc_name.clone(),
                    ctx.workflow_id,
                    None,
                    Some(ctx.step_id),
                )
                .await
            {
                Ok(doc) => {
                    let _ = state
                        .repos()
                        .workflows
                        .link_document_to_def(did, doc.id)
                        .await;
                    let _ = doc_repo
                        .update_document(doc.id, Some(content.to_string()), None, None)
                        .await;
                    Some(doc.id)
                }
                Err(e) => {
                    warn!(doc = %ctx.doc_name, "Failed to create document: {}", e);
                    None
                }
            }
        }
        DocumentPersistAction::Skip => None,
    };

    // Create content version snapshot for this run
    if let Some(doc_id) = persisted_doc_id {
        let cv_repo = &*state.repos().content_versions;
        if let Err(e) = super::super::versioning::snapshot_content(
            cv_repo,
            ctx.run_id,
            ctx.step_id,
            doc_id,
            super::super::versioning::content_types::DOCUMENT,
            "output",
            content,
        )
        .await
        {
            warn!(doc = %ctx.doc_name, "Failed to snapshot document version: {e}");
        }
    }
}
