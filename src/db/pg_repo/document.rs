use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

use crate::db::traits::{CreateDocumentInput, DocumentRepo, OutputSchemaRepo, PromptTemplateRepo};
use crate::db::{DocumentRow, DocumentSearchResult, OutputSchemaRow, PromptTemplateRow};

use super::PgRepo;

#[async_trait]
impl DocumentRepo for PgRepo {
    async fn create_document(&self, input: CreateDocumentInput) -> Result<DocumentRow> {
        let id = Uuid::new_v4();
        let row: DocumentRow = sqlx::query_as(
            r#"
            INSERT INTO documents (id, user_id, session_id, title, content, doc_type, ref_tag, tags)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, user_id, session_id, title, content, summary, doc_type, ref_tag, tags, created_at, updated_at, workflow_id, target_length, is_static, source_protocol_step_id
            "#,
        )
        .bind(id)
        .bind(input.user_id)
        .bind(input.session_id)
        .bind(&input.title)
        .bind(&input.content)
        .bind(&input.doc_type)
        .bind(&input.ref_tag)
        .bind(&input.tags)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    async fn create_workflow_document(
        &self,
        user_id: Uuid,
        title: String,
        workflow_id: Uuid,
        target_length: Option<i32>,
        source_protocol_step_id: Option<Uuid>,
    ) -> Result<DocumentRow> {
        let id = Uuid::new_v4();
        let row: DocumentRow = sqlx::query_as(
            r#"
            INSERT INTO documents (id, user_id, title, content, doc_type, workflow_id, target_length, is_static, source_protocol_step_id)
            VALUES ($1, $2, $3, '', 'protocol', $4, $5, false, $6)
            RETURNING id, user_id, session_id, title, content, summary, doc_type, ref_tag, tags, created_at, updated_at, workflow_id, target_length, is_static, source_protocol_step_id
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(&title)
        .bind(workflow_id)
        .bind(target_length)
        .bind(source_protocol_step_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    async fn update_document(
        &self,
        doc_id: Uuid,
        content: Option<String>,
        title: Option<String>,
        tags: Option<Vec<String>>,
    ) -> Result<DocumentRow> {
        let row: DocumentRow = sqlx::query_as(
            r#"
            UPDATE documents
            SET
                content = COALESCE($1, content),
                title = COALESCE($2, title),
                tags = COALESCE($3, tags),
                updated_at = NOW()
            WHERE id = $4
            RETURNING id, user_id, session_id, title, content, summary, doc_type, ref_tag, tags, created_at, updated_at, workflow_id, target_length, is_static, source_protocol_step_id
            "#,
        )
        .bind(content)
        .bind(title)
        .bind(tags)
        .bind(doc_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    async fn update_document_summary(&self, doc_id: Uuid, summary: String) -> Result<()> {
        sqlx::query("UPDATE documents SET summary = $1, updated_at = NOW() WHERE id = $2")
            .bind(&summary)
            .bind(doc_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_document(&self, doc_id: Uuid) -> Result<Option<DocumentRow>> {
        let row: Option<DocumentRow> = sqlx::query_as("SELECT id, user_id, session_id, title, content, summary, doc_type, ref_tag, tags, created_at, updated_at, workflow_id, target_length, is_static, source_protocol_step_id FROM documents WHERE id = $1")
            .bind(doc_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row)
    }

    async fn get_documents_by_ids(&self, doc_ids: &[Uuid]) -> Result<Vec<DocumentRow>> {
        let rows: Vec<DocumentRow> = sqlx::query_as(
            "SELECT id, user_id, session_id, title, content, summary, doc_type, ref_tag, tags, created_at, updated_at, workflow_id, target_length, is_static, source_protocol_step_id FROM documents WHERE id = ANY($1)",
        )
        .bind(doc_ids)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn get_document_by_ref_tag(&self, ref_tag: &str) -> Result<Option<DocumentRow>> {
        let row: Option<DocumentRow> = sqlx::query_as("SELECT id, user_id, session_id, title, content, summary, doc_type, ref_tag, tags, created_at, updated_at, workflow_id, target_length, is_static, source_protocol_step_id FROM documents WHERE ref_tag = $1")
            .bind(ref_tag)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row)
    }

    async fn list_documents(&self, user_id: Uuid) -> Result<Vec<DocumentRow>> {
        let rows: Vec<DocumentRow> =
            sqlx::query_as("SELECT id, user_id, session_id, title, content, summary, doc_type, ref_tag, tags, created_at, updated_at, workflow_id, target_length, is_static, source_protocol_step_id FROM documents WHERE user_id = $1 ORDER BY updated_at DESC")
                .bind(user_id)
                .fetch_all(&self.pool)
                .await?;
        Ok(rows)
    }

    async fn list_session_documents(&self, session_id: Uuid) -> Result<Vec<DocumentRow>> {
        let rows: Vec<DocumentRow> =
            sqlx::query_as("SELECT id, user_id, session_id, title, content, summary, doc_type, ref_tag, tags, created_at, updated_at FROM documents WHERE session_id = $1 ORDER BY updated_at DESC")
                .bind(session_id)
                .fetch_all(&self.pool)
                .await?;
        Ok(rows)
    }

    async fn search_documents(
        &self,
        user_id: Uuid,
        query: &str,
    ) -> Result<Vec<DocumentSearchResult>> {
        let rows: Vec<DocumentSearchResult> = sqlx::query_as(
            r#"
            SELECT id, title, summary, ref_tag,
                   ts_headline('english', content, plainto_tsquery('english', $2),
                       'StartSel=**, StopSel=**, MaxWords=35, MinWords=15') AS snippet
            FROM documents
            WHERE user_id = $1
              AND to_tsvector('english', title || ' ' || content) @@ plainto_tsquery('english', $2)
            ORDER BY ts_rank(to_tsvector('english', title || ' ' || content), plainto_tsquery('english', $2)) DESC
            LIMIT 50
            "#,
        )
        .bind(user_id)
        .bind(query)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn delete_document(&self, doc_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM documents WHERE id = $1")
            .bind(doc_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[async_trait]
impl OutputSchemaRepo for PgRepo {
    async fn create_output_schema(
        &self,
        user_id: Option<Uuid>,
        name: String,
        schema: serde_json::Value,
    ) -> Result<OutputSchemaRow> {
        let row: OutputSchemaRow = sqlx::query_as(
            r#"
            INSERT INTO output_schemas (user_id, name, schema)
            VALUES ($1, $2, $3)
            RETURNING id, user_id, name, schema, created_at, version
            "#,
        )
        .bind(user_id)
        .bind(&name)
        .bind(&schema)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn get_output_schema(&self, id: Uuid) -> Result<Option<OutputSchemaRow>> {
        let row: Option<OutputSchemaRow> = sqlx::query_as("SELECT id, user_id, name, schema, created_at, version FROM output_schemas WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row)
    }

    async fn list_output_schemas(&self, user_id: Uuid) -> Result<Vec<OutputSchemaRow>> {
        let rows: Vec<OutputSchemaRow> = sqlx::query_as("SELECT id, user_id, name, schema, created_at, version FROM output_schemas WHERE user_id = $1 OR user_id IS NULL ORDER BY created_at DESC")
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    async fn update_output_schema(
        &self,
        id: Uuid,
        name: Option<String>,
        schema: Option<serde_json::Value>,
    ) -> Result<OutputSchemaRow> {
        let row: OutputSchemaRow = sqlx::query_as(
            r#"
            UPDATE output_schemas
            SET name = COALESCE($1, name),
                schema = COALESCE($2, schema),
                version = version + 1
            WHERE id = $3
            RETURNING id, user_id, name, schema, created_at, version
            "#,
        )
        .bind(name)
        .bind(schema)
        .bind(id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn delete_output_schema(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM output_schemas WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[async_trait]
impl PromptTemplateRepo for PgRepo {
    async fn create_prompt_template(
        &self,
        user_id: Option<Uuid>,
        name: String,
        content: String,
    ) -> Result<PromptTemplateRow> {
        let row: PromptTemplateRow = sqlx::query_as(
            r#"
            INSERT INTO prompt_templates (user_id, name, content)
            VALUES ($1, $2, $3)
            RETURNING id, user_id, name, content, created_at, version
            "#,
        )
        .bind(user_id)
        .bind(&name)
        .bind(&content)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn get_prompt_template(&self, id: Uuid) -> Result<Option<PromptTemplateRow>> {
        let row: Option<PromptTemplateRow> = sqlx::query_as("SELECT id, user_id, name, content, created_at, version FROM prompt_templates WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row)
    }

    async fn list_prompt_templates(&self, user_id: Uuid) -> Result<Vec<PromptTemplateRow>> {
        let rows: Vec<PromptTemplateRow> = sqlx::query_as("SELECT id, user_id, name, content, created_at, version FROM prompt_templates WHERE user_id = $1 OR user_id IS NULL ORDER BY created_at DESC")
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    async fn update_prompt_template(
        &self,
        id: Uuid,
        name: Option<String>,
        content: Option<String>,
    ) -> Result<PromptTemplateRow> {
        let row: PromptTemplateRow = sqlx::query_as(
            r#"
            UPDATE prompt_templates
            SET name = COALESCE($1, name),
                content = COALESCE($2, content),
                version = version + 1
            WHERE id = $3
            RETURNING id, user_id, name, content, created_at, version
            "#,
        )
        .bind(name)
        .bind(content)
        .bind(id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn delete_prompt_template(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM prompt_templates WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
