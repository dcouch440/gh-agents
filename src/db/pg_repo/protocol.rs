use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

use crate::db::traits::{
    CreateProtocolInput, ProtocolRepo, UpdateProtocolExecutionStatusInput, UpdateProtocolInput,
};
use crate::db::{
    ProtocolDocumentDefRow, ProtocolExecutionRow, ProtocolPortRow, ProtocolRow,
    WorkflowStepProtocolRow,
};

use super::PgRepo;

#[async_trait]
impl ProtocolRepo for PgRepo {
    async fn create_protocol(&self, input: CreateProtocolInput) -> Result<ProtocolRow> {
        let row = sqlx::query_as::<_, ProtocolRow>(
            "INSERT INTO protocols (name, description, protocol_type, config, agent_id, output_schema_id, prompt_template_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             RETURNING id, name, description, protocol_type, config, version, created_at, updated_at, agent_id, output_schema_id, prompt_template_id",
        )
        .bind(&input.name)
        .bind(&input.description)
        .bind(&input.protocol_type)
        .bind(&input.config)
        .bind(input.agent_id)
        .bind(input.output_schema_id)
        .bind(input.prompt_template_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn get_protocol(&self, id: Uuid) -> Result<Option<ProtocolRow>> {
        let row = sqlx::query_as::<_, ProtocolRow>(
            "SELECT id, name, description, protocol_type, config, version, created_at, updated_at, agent_id, output_schema_id, prompt_template_id
             FROM protocols WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn get_protocol_by_type(&self, protocol_type: &str) -> Result<Option<ProtocolRow>> {
        let row = sqlx::query_as::<_, ProtocolRow>(
            "SELECT id, name, description, protocol_type, config, version, created_at, updated_at, agent_id, output_schema_id, prompt_template_id
             FROM protocols WHERE protocol_type = $1 LIMIT 1",
        )
        .bind(protocol_type)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn list_protocols(&self) -> Result<Vec<ProtocolRow>> {
        let rows = sqlx::query_as::<_, ProtocolRow>(
            "SELECT id, name, description, protocol_type, config, version, created_at, updated_at, agent_id, output_schema_id, prompt_template_id
             FROM protocols ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn update_protocol(&self, input: UpdateProtocolInput) -> Result<ProtocolRow> {
        let row = sqlx::query_as::<_, ProtocolRow>(
            "UPDATE protocols SET
                name = COALESCE($2, name),
                description = COALESCE($3, description),
                config = COALESCE($4, config),
                agent_id = COALESCE($5, agent_id),
                output_schema_id = COALESCE($6, output_schema_id),
                prompt_template_id = COALESCE($7, prompt_template_id),
                version = version + 1,
                updated_at = now()
             WHERE id = $1
             RETURNING id, name, description, protocol_type, config, version, created_at, updated_at, agent_id, output_schema_id, prompt_template_id",
        )
        .bind(input.id)
        .bind(input.name)
        .bind(input.description)
        .bind(input.config)
        .bind(input.agent_id)
        .bind(input.output_schema_id)
        .bind(input.prompt_template_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn delete_protocol(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM protocols WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // --- Protocol Ports ---

    async fn list_protocol_ports(&self, protocol_id: Uuid) -> Result<Vec<ProtocolPortRow>> {
        let rows = sqlx::query_as::<_, ProtocolPortRow>(
            "SELECT id, protocol_id, port_name, description, agent_id, display_order
             FROM protocol_ports WHERE protocol_id = $1 ORDER BY display_order",
        )
        .bind(protocol_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn create_protocol_port(
        &self,
        protocol_id: Uuid,
        port_name: String,
        description: String,
        agent_id: Uuid,
        display_order: i32,
    ) -> Result<ProtocolPortRow> {
        let row = sqlx::query_as::<_, ProtocolPortRow>(
            "INSERT INTO protocol_ports (protocol_id, port_name, description, agent_id, display_order)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING id, protocol_id, port_name, description, agent_id, display_order",
        )
        .bind(protocol_id)
        .bind(&port_name)
        .bind(&description)
        .bind(agent_id)
        .bind(display_order)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn update_protocol_port(
        &self,
        id: Uuid,
        port_name: Option<String>,
        description: Option<String>,
        agent_id: Option<Uuid>,
        display_order: Option<i32>,
    ) -> Result<ProtocolPortRow> {
        let row = sqlx::query_as::<_, ProtocolPortRow>(
            "UPDATE protocol_ports SET
                port_name = COALESCE($2, port_name),
                description = COALESCE($3, description),
                agent_id = COALESCE($4, agent_id),
                display_order = COALESCE($5, display_order)
             WHERE id = $1
             RETURNING id, protocol_id, port_name, description, agent_id, display_order",
        )
        .bind(id)
        .bind(port_name)
        .bind(description)
        .bind(agent_id)
        .bind(display_order)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn delete_protocol_port(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM protocol_ports WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // --- Workflow Step Protocol Linkage ---

    async fn get_step_protocol(
        &self,
        workflow_step_id: Uuid,
    ) -> Result<Option<WorkflowStepProtocolRow>> {
        let row = sqlx::query_as::<_, WorkflowStepProtocolRow>(
            "SELECT id, workflow_step_id, protocol_id, applied_expansion, created_at
             FROM workflow_step_protocols WHERE workflow_step_id = $1",
        )
        .bind(workflow_step_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn create_step_protocol(
        &self,
        workflow_step_id: Uuid,
        protocol_id: Uuid,
        applied_expansion: serde_json::Value,
    ) -> Result<WorkflowStepProtocolRow> {
        let row = sqlx::query_as::<_, WorkflowStepProtocolRow>(
            "INSERT INTO workflow_step_protocols (workflow_step_id, protocol_id, applied_expansion)
             VALUES ($1, $2, $3)
             RETURNING id, workflow_step_id, protocol_id, applied_expansion, created_at",
        )
        .bind(workflow_step_id)
        .bind(protocol_id)
        .bind(&applied_expansion)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn delete_step_protocol(&self, workflow_step_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM workflow_step_protocols WHERE workflow_step_id = $1")
            .bind(workflow_step_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // --- Protocol-scoped Document Definitions ---

    async fn list_protocol_document_defs(
        &self,
        protocol_id: Uuid,
    ) -> Result<Vec<ProtocolDocumentDefRow>> {
        let rows = sqlx::query_as::<_, ProtocolDocumentDefRow>(
            "SELECT id, step_id, name, description, target_length, display_order, created_at, protocol_id, document_id, agent_roster_entry_id \
             FROM protocol_document_defs WHERE protocol_id = $1 ORDER BY display_order, created_at",
        )
        .bind(protocol_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn create_protocol_document_def(
        &self,
        def: ProtocolDocumentDefRow,
    ) -> Result<ProtocolDocumentDefRow> {
        let row = sqlx::query_as::<_, ProtocolDocumentDefRow>(
            "INSERT INTO protocol_document_defs (id, step_id, name, description, target_length, display_order, protocol_id, document_id, agent_roster_entry_id) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
             RETURNING id, step_id, name, description, target_length, display_order, created_at, protocol_id, document_id, agent_roster_entry_id",
        )
        .bind(def.id)
        .bind(def.step_id)
        .bind(&def.name)
        .bind(&def.description)
        .bind(def.target_length)
        .bind(def.display_order)
        .bind(def.protocol_id)
        .bind(def.document_id)
        .bind(def.agent_roster_entry_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn update_protocol_document_def(
        &self,
        id: Uuid,
        name: String,
        description: String,
        target_length: i32,
    ) -> Result<ProtocolDocumentDefRow> {
        let row = sqlx::query_as::<_, ProtocolDocumentDefRow>(
            "UPDATE protocol_document_defs SET name = $2, description = $3, target_length = $4 \
             WHERE id = $1 \
             RETURNING id, step_id, name, description, target_length, display_order, created_at, protocol_id, document_id, agent_roster_entry_id",
        )
        .bind(id)
        .bind(&name)
        .bind(&description)
        .bind(target_length)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn delete_protocol_document_def(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM protocol_document_defs WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // --- Protocol Executions ---

    async fn create_protocol_execution(
        &self,
        row: ProtocolExecutionRow,
    ) -> Result<ProtocolExecutionRow> {
        let result = sqlx::query_as::<_, ProtocolExecutionRow>(
            "INSERT INTO protocol_executions \
             (id, protocol_step_id, workflow_run_id, phase, document_def_id, agent_id, \
              input_prompt, output_content, status, error_message, \
              tokens_in, tokens_out, cost_usd, model, capabilities_used, created_at, completed_at, \
              agent_name, archetype, designer_run_id) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, \
                     $18, $19, $20) \
             RETURNING *",
        )
        .bind(row.id)
        .bind(row.protocol_step_id)
        .bind(row.workflow_run_id)
        .bind(&row.phase)
        .bind(row.document_def_id)
        .bind(row.agent_id)
        .bind(&row.input_prompt)
        .bind(&row.output_content)
        .bind(&row.status)
        .bind(&row.error_message)
        .bind(row.tokens_in)
        .bind(row.tokens_out)
        .bind(row.cost_usd)
        .bind(&row.model)
        .bind(&row.capabilities_used)
        .bind(row.created_at)
        .bind(row.completed_at)
        .bind(&row.agent_name)
        .bind(&row.archetype)
        .bind(row.designer_run_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(result)
    }

    async fn update_protocol_execution_status(
        &self,
        input: UpdateProtocolExecutionStatusInput,
    ) -> Result<ProtocolExecutionRow> {
        let row = sqlx::query_as::<_, ProtocolExecutionRow>(
            "UPDATE protocol_executions \
             SET status = $2, output_content = $3, error_message = $4, \
                 tokens_in = $5, tokens_out = $6, cost_usd = $7, model = $8, \
                 completed_at = CASE WHEN $2 IN ('complete', 'failed') THEN now() ELSE completed_at END \
             WHERE id = $1 \
             RETURNING *",
        )
        .bind(input.id)
        .bind(&input.status)
        .bind(&input.output_content)
        .bind(&input.error_message)
        .bind(input.tokens_in)
        .bind(input.tokens_out)
        .bind(input.cost_usd)
        .bind(&input.model)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn list_protocol_executions_by_step(
        &self,
        step_id: Uuid,
    ) -> Result<Vec<ProtocolExecutionRow>> {
        let rows = sqlx::query_as::<_, ProtocolExecutionRow>(
            "SELECT * FROM protocol_executions WHERE protocol_step_id = $1 ORDER BY created_at",
        )
        .bind(step_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn list_protocol_executions_by_run(
        &self,
        run_id: Uuid,
    ) -> Result<Vec<ProtocolExecutionRow>> {
        let rows = sqlx::query_as::<_, ProtocolExecutionRow>(
            "SELECT * FROM protocol_executions WHERE workflow_run_id = $1 ORDER BY created_at",
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}
