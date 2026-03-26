use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

use crate::db::{DocumentRow, DocumentSearchResult, OutputSchemaRow, PromptTemplateRow};

// ============================================================================
// Document Repository
// ============================================================================

/// Input for creating a new document.
#[derive(Debug, Clone)]
pub struct CreateDocumentInput {
    pub user_id: Uuid,
    pub session_id: Option<Uuid>,
    pub title: String,
    pub content: String,
    pub doc_type: String,
    pub ref_tag: String,
    pub tags: Vec<String>,
}

/// Database operations for document management.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait DocumentRepo: Send + Sync {
    /// Create a new document.
    async fn create_document(&self, input: CreateDocumentInput) -> Result<DocumentRow>;

    /// Create a blank document linked to a workflow for protocol-generated content.
    ///
    /// Sets `workflow_id`, `target_length`, `source_protocol_step_id`, and `is_static = false`.
    /// Content starts empty and is populated by the protocol executor at runtime.
    async fn create_workflow_document(
        &self,
        user_id: Uuid,
        title: String,
        workflow_id: Uuid,
        target_length: Option<i32>,
        source_protocol_step_id: Option<Uuid>,
    ) -> Result<DocumentRow>;

    /// Update a document's content, title, and tags.
    async fn update_document(
        &self,
        doc_id: Uuid,
        content: Option<String>,
        title: Option<String>,
        tags: Option<Vec<String>>,
    ) -> Result<DocumentRow>;

    /// Update a document's summary.
    async fn update_document_summary(&self, doc_id: Uuid, summary: String) -> Result<()>;

    /// Get a document by ID.
    async fn get_document(&self, doc_id: Uuid) -> Result<Option<DocumentRow>>;

    /// Get multiple documents by IDs in a single query.
    async fn get_documents_by_ids(&self, doc_ids: &[Uuid]) -> Result<Vec<DocumentRow>>;

    /// Get a document by ref_tag.
    async fn get_document_by_ref_tag(&self, ref_tag: &str) -> Result<Option<DocumentRow>>;

    /// List all documents for a user.
    async fn list_documents(&self, user_id: Uuid) -> Result<Vec<DocumentRow>>;

    /// List all documents for a session.
    async fn list_session_documents(&self, session_id: Uuid) -> Result<Vec<DocumentRow>>;

    /// Full-text search documents for a user.
    async fn search_documents(
        &self,
        user_id: Uuid,
        query: &str,
    ) -> Result<Vec<DocumentSearchResult>>;

    /// Delete a document by ID.
    async fn delete_document(&self, doc_id: Uuid) -> Result<()>;
}

// ============================================================================
// Output Schema Repository
// ============================================================================

/// Database operations for output schema management.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait OutputSchemaRepo: Send + Sync {
    /// Create a new output schema.
    async fn create_output_schema(
        &self,
        user_id: Option<Uuid>,
        name: String,
        schema: serde_json::Value,
    ) -> Result<OutputSchemaRow>;

    /// Get an output schema by ID.
    async fn get_output_schema(&self, id: Uuid) -> Result<Option<OutputSchemaRow>>;

    /// List all output schemas for a user.
    async fn list_output_schemas(&self, user_id: Uuid) -> Result<Vec<OutputSchemaRow>>;

    /// Update an output schema's name and/or schema.
    async fn update_output_schema(
        &self,
        id: Uuid,
        name: Option<String>,
        schema: Option<serde_json::Value>,
    ) -> Result<OutputSchemaRow>;

    /// Delete an output schema by ID.
    async fn delete_output_schema(&self, id: Uuid) -> Result<()>;
}

// ============================================================================
// Prompt Template Repository
// ============================================================================

/// Database operations for prompt template management.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait PromptTemplateRepo: Send + Sync {
    /// Create a new prompt template.
    async fn create_prompt_template(
        &self,
        user_id: Option<Uuid>,
        name: String,
        content: String,
    ) -> Result<PromptTemplateRow>;

    /// Get a prompt template by ID.
    async fn get_prompt_template(&self, id: Uuid) -> Result<Option<PromptTemplateRow>>;

    /// List all prompt templates for a user.
    async fn list_prompt_templates(&self, user_id: Uuid) -> Result<Vec<PromptTemplateRow>>;

    /// Update a prompt template's name and/or content.
    async fn update_prompt_template(
        &self,
        id: Uuid,
        name: Option<String>,
        content: Option<String>,
    ) -> Result<PromptTemplateRow>;

    /// Delete a prompt template by ID.
    async fn delete_prompt_template(&self, id: Uuid) -> Result<()>;
}
