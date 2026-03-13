use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

use crate::db::traits::{CreateDesignerOutputGenericInput, CreateDesignerOutputInput};
use crate::db::traits::{
    CreateStepInputPort, CreateWorkflowInput, UpdateWorkflowInput, WorkflowRepo,
};
use crate::db::{
    AgentDesignerOutputRow, AgentDesignerRunRow, BeliefExtractionPlanRow, BeliefRow,
    CanvasElementMapRow, CanvasSnapshotRow, ProtocolDocumentDefRow, RoomStepConfigRow,
    RoomStepMemberRow, RunTemplateRow, StepDocumentRow, StepInputRow, StepOutputRow,
    StepQuestionStateRow, StepRoutingRuleRow, TaskAgentRosterRow, TaskMissionBriefRow, WorkflowRow,
    WorkflowStepEdgeRow, WorkflowStepRow,
};

use super::PgRepo;

#[async_trait]
impl WorkflowRepo for PgRepo {
    // --- Workflows ---

    async fn create_workflow(&self, input: CreateWorkflowInput) -> Result<WorkflowRow> {
        let row: WorkflowRow = sqlx::query_as(
            "INSERT INTO workflows (user_id, name, description, container_enabled, target_repo_url, target_branch, vpn_enabled) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) \
             RETURNING id, user_id, name, description, execution_mode, created_at, version, container_enabled, target_repo_url, target_branch, vpn_enabled, board_overview_summary",
        )
        .bind(input.user_id)
        .bind(&input.name)
        .bind(&input.description)
        .bind(input.container_enabled)
        .bind(&input.target_repo_url)
        .bind(&input.target_branch)
        .bind(input.vpn_enabled)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn get_workflow(&self, id: Uuid) -> Result<Option<WorkflowRow>> {
        let row: Option<WorkflowRow> = sqlx::query_as(
            "SELECT id, user_id, name, description, execution_mode, created_at, version, container_enabled, target_repo_url, target_branch, vpn_enabled, board_overview_summary \
             FROM workflows WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn list_workflows(&self, user_id: Uuid) -> Result<Vec<WorkflowRow>> {
        let rows: Vec<WorkflowRow> = sqlx::query_as(
            "SELECT id, user_id, name, description, execution_mode, created_at, version, container_enabled, target_repo_url, target_branch, vpn_enabled, board_overview_summary \
             FROM workflows WHERE user_id = $1 ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn update_workflow(&self, input: UpdateWorkflowInput) -> Result<WorkflowRow> {
        // Build dynamic SET clauses for optional container fields
        let row: WorkflowRow = sqlx::query_as(
            "UPDATE workflows SET \
             name = COALESCE($1, name), \
             description = COALESCE($2, description), \
             container_enabled = COALESCE($3, container_enabled), \
             target_repo_url = CASE WHEN $4 THEN $5 ELSE target_repo_url END, \
             target_branch = CASE WHEN $6 THEN $7 ELSE target_branch END, \
             vpn_enabled = COALESCE($8, vpn_enabled), \
             version = version + 1 \
             WHERE id = $9 \
             RETURNING id, user_id, name, description, execution_mode, created_at, version, container_enabled, target_repo_url, target_branch, vpn_enabled, board_overview_summary",
        )
        .bind(input.name)
        .bind(input.description)
        .bind(input.container_enabled)
        .bind(input.target_repo_url.is_some()) // $4: whether to update target_repo_url
        .bind(input.target_repo_url.unwrap_or(None)) // $5: the value (may be None to clear)
        .bind(input.target_branch.is_some()) // $6: whether to update target_branch
        .bind(input.target_branch.unwrap_or(None)) // $7: the value (may be None to clear)
        .bind(input.vpn_enabled)
        .bind(input.id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn delete_workflow(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM workflows WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // --- Steps ---

    async fn create_step(&self, step: WorkflowStepRow) -> Result<WorkflowStepRow> {
        let row: WorkflowStepRow = sqlx::query_as(
            r#"
            INSERT INTO workflow_steps (id, workflow_id, agent_id, execution_mode, for_each_ref, prompt_template_id, prompt_template, output_schema_id, output_variable_name, interactive_agent_id, for_each_label_field, display_order, reasoning_trace, verification_agent_ids, position_x, position_y, width, height, name, system_prompt_suffix, visible, description, child_workflow_id, ref_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24)
            RETURNING *
            "#,
        )
        .bind(step.id)
        .bind(step.workflow_id)
        .bind(step.agent_id)
        .bind(&step.execution_mode)
        .bind(&step.for_each_ref)
        .bind(step.prompt_template_id)
        .bind(&step.prompt_template)
        .bind(step.output_schema_id)
        .bind(&step.output_variable_name)
        .bind(step.interactive_agent_id)
        .bind(&step.for_each_label_field)
        .bind(step.display_order)
        .bind(step.reasoning_trace)
        .bind(&step.verification_agent_ids)
        .bind(step.position_x)
        .bind(step.position_y)
        .bind(step.width)
        .bind(step.height)
        .bind(&step.name)
        .bind(&step.system_prompt_suffix)
        .bind(step.visible)
        .bind(&step.description)
        .bind(step.child_workflow_id)
        .bind(&step.ref_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn get_step(&self, id: Uuid) -> Result<Option<WorkflowStepRow>> {
        let row: Option<WorkflowStepRow> =
            sqlx::query_as("SELECT * FROM workflow_steps WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row)
    }

    async fn find_step_by_ref_id(
        &self,
        workflow_id: Uuid,
        ref_id: &str,
    ) -> Result<Option<WorkflowStepRow>> {
        let row: Option<WorkflowStepRow> =
            sqlx::query_as("SELECT * FROM workflow_steps WHERE workflow_id = $1 AND ref_id = $2")
                .bind(workflow_id)
                .bind(ref_id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row)
    }

    async fn list_steps(&self, workflow_id: Uuid) -> Result<Vec<WorkflowStepRow>> {
        let rows: Vec<WorkflowStepRow> = sqlx::query_as(
            "SELECT * FROM workflow_steps WHERE workflow_id = $1 ORDER BY display_order",
        )
        .bind(workflow_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn update_step(&self, step: WorkflowStepRow) -> Result<WorkflowStepRow> {
        let row: WorkflowStepRow = sqlx::query_as(
            r#"
            UPDATE workflow_steps
            SET agent_id = $1, execution_mode = $2, for_each_ref = $3, prompt_template_id = $4, prompt_template = $5,
                output_schema_id = $6, output_variable_name = $7, interactive_agent_id = $8, for_each_label_field = $9, display_order = $10,
                reasoning_trace = $11, verification_agent_ids = $12, position_x = $13, position_y = $14, width = $15, height = $16,
                name = $17, system_prompt_suffix = $18, visible = $19, description = $20, child_workflow_id = $21,
                board_context_cache = $22, board_context_updated_at = $23,
                version = version + 1
            WHERE id = $24
            RETURNING *
            "#,
        )
        .bind(step.agent_id)
        .bind(&step.execution_mode)
        .bind(&step.for_each_ref)
        .bind(step.prompt_template_id)
        .bind(&step.prompt_template)
        .bind(step.output_schema_id)
        .bind(&step.output_variable_name)
        .bind(step.interactive_agent_id)
        .bind(&step.for_each_label_field)
        .bind(step.display_order)
        .bind(step.reasoning_trace)
        .bind(&step.verification_agent_ids)
        .bind(step.position_x)
        .bind(step.position_y)
        .bind(step.width)
        .bind(step.height)
        .bind(&step.name)
        .bind(&step.system_prompt_suffix)
        .bind(step.visible)
        .bind(&step.description)
        .bind(step.child_workflow_id)
        .bind(&step.board_context_cache)
        .bind(step.board_context_updated_at)
        .bind(step.id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn delete_step(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM workflow_steps WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn set_step_pinned(&self, step_id: Uuid, pinned: bool) -> Result<()> {
        sqlx::query("UPDATE workflow_steps SET pinned = $1 WHERE id = $2")
            .bind(pinned)
            .bind(step_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn update_run_results_summary(&self, step_id: Uuid, summary: &str) -> Result<()> {
        sqlx::query("UPDATE workflow_steps SET run_results_summary = $1 WHERE id = $2")
            .bind(summary)
            .bind(step_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_run_context_for_step(
        &self,
        workflow_id: Uuid,
        step_id: Uuid,
    ) -> Result<Vec<(String, String, bool)>> {
        let rows: Vec<(String, String, bool)> = sqlx::query_as(
            "SELECT COALESCE(ws.name, ws.execution_mode) AS step_name, \
                    ws.run_results_summary, \
                    ws.pinned \
             FROM workflow_steps ws \
             WHERE ws.id = $1 AND ws.run_results_summary != '' \
             UNION \
             SELECT COALESCE(ws.name, ws.execution_mode) AS step_name, \
                    ws.run_results_summary, \
                    ws.pinned \
             FROM workflow_steps ws \
             JOIN workflow_step_edges e ON (ws.id = e.from_step_id AND e.to_step_id = $1) \
                                        OR (ws.id = e.to_step_id AND e.from_step_id = $1) \
             WHERE ws.workflow_id = $2 AND ws.run_results_summary != ''",
        )
        .bind(step_id)
        .bind(workflow_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    // --- Edges ---

    async fn set_edges(&self, workflow_id: Uuid, edges: Vec<WorkflowStepEdgeRow>) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM workflow_step_edges WHERE from_step_id IN (SELECT id FROM workflow_steps WHERE workflow_id = $1)")
            .bind(workflow_id)
            .execute(&mut *tx)
            .await?;
        for edge in &edges {
            sqlx::query(
                "INSERT INTO workflow_step_edges (workflow_id, from_step_id, to_step_id) VALUES ($1, $2, $3)",
            )
            .bind(workflow_id)
            .bind(edge.from_step_id)
            .bind(edge.to_step_id)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn list_edges(&self, workflow_id: Uuid) -> Result<Vec<WorkflowStepEdgeRow>> {
        let rows: Vec<WorkflowStepEdgeRow> = sqlx::query_as(
            "SELECT e.id, e.from_step_id, e.to_step_id, e.from_output_port, e.to_input_port, \
             e.transform_jsonpath, e.condition_type, e.condition_value, e.edge_label, e.workflow_id \
             FROM workflow_step_edges e \
             WHERE e.workflow_id = $1"
        )
            .bind(workflow_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    async fn add_edge(
        &self,
        workflow_id: Uuid,
        from_step_id: Uuid,
        to_step_id: Uuid,
    ) -> Result<WorkflowStepEdgeRow> {
        let row: WorkflowStepEdgeRow = sqlx::query_as(
            "INSERT INTO workflow_step_edges (workflow_id, from_step_id, to_step_id) VALUES ($1, $2, $3) ON CONFLICT (workflow_id, from_step_id, to_step_id) DO UPDATE SET from_step_id = EXCLUDED.from_step_id RETURNING *",
        )
        .bind(workflow_id)
        .bind(from_step_id)
        .bind(to_step_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn remove_edge(
        &self,
        from_step_id: Uuid,
        to_step_id: Uuid,
    ) -> Result<WorkflowStepEdgeRow> {
        let row: WorkflowStepEdgeRow = sqlx::query_as(
            "DELETE FROM workflow_step_edges WHERE from_step_id = $1 AND to_step_id = $2 RETURNING *",
        )
        .bind(from_step_id)
        .bind(to_step_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn delete_edge_by_id(&self, edge_id: Uuid) -> Result<WorkflowStepEdgeRow> {
        let row: WorkflowStepEdgeRow =
            sqlx::query_as("DELETE FROM workflow_step_edges WHERE id = $1 RETURNING *")
                .bind(edge_id)
                .fetch_one(&self.pool)
                .await?;
        Ok(row)
    }

    // --- Step documents ---

    async fn list_step_documents(&self, step_id: Uuid) -> Result<Vec<StepDocumentRow>> {
        let rows: Vec<StepDocumentRow> =
            sqlx::query_as("SELECT step_id, document_id FROM step_documents WHERE step_id = $1")
                .bind(step_id)
                .fetch_all(&self.pool)
                .await?;
        Ok(rows)
    }

    async fn add_step_document(&self, step_id: Uuid, document_id: Uuid) -> Result<()> {
        sqlx::query("INSERT INTO step_documents (step_id, document_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(step_id)
            .bind(document_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn remove_step_document(&self, step_id: Uuid, document_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM step_documents WHERE step_id = $1 AND document_id = $2")
            .bind(step_id)
            .bind(document_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // --- Protocol Document Definitions ---

    async fn get_document_def(&self, id: Uuid) -> Result<Option<ProtocolDocumentDefRow>> {
        let row = sqlx::query_as::<_, ProtocolDocumentDefRow>(
            "SELECT id, step_id, name, description, target_length, display_order, created_at, protocol_id, document_id, agent_roster_entry_id FROM protocol_document_defs WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn list_document_defs(&self, step_id: Uuid) -> Result<Vec<ProtocolDocumentDefRow>> {
        let rows = sqlx::query_as::<_, ProtocolDocumentDefRow>(
            "SELECT id, step_id, name, description, target_length, display_order, created_at, protocol_id, document_id, agent_roster_entry_id FROM protocol_document_defs WHERE step_id = $1 ORDER BY display_order, created_at",
        )
        .bind(step_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn create_document_def(
        &self,
        def: ProtocolDocumentDefRow,
    ) -> Result<ProtocolDocumentDefRow> {
        let row = sqlx::query_as::<_, ProtocolDocumentDefRow>(
            "INSERT INTO protocol_document_defs (id, step_id, name, description, target_length, display_order, protocol_id, document_id, agent_roster_entry_id) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING id, step_id, name, description, target_length, display_order, created_at, protocol_id, document_id, agent_roster_entry_id",
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

    async fn update_document_def(
        &self,
        id: Uuid,
        name: String,
        description: String,
        target_length: i32,
    ) -> Result<ProtocolDocumentDefRow> {
        let row = sqlx::query_as::<_, ProtocolDocumentDefRow>(
            "UPDATE protocol_document_defs SET name = $2, description = $3, target_length = $4 WHERE id = $1 RETURNING id, step_id, name, description, target_length, display_order, created_at, protocol_id, document_id, agent_roster_entry_id",
        )
        .bind(id)
        .bind(&name)
        .bind(&description)
        .bind(target_length)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn link_document_to_def(&self, def_id: Uuid, document_id: Uuid) -> Result<()> {
        sqlx::query("UPDATE protocol_document_defs SET document_id = $1 WHERE id = $2")
            .bind(document_id)
            .bind(def_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn delete_document_def(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM protocol_document_defs WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // --- Port Management (Phase 3) ---

    async fn get_step_inputs(&self, workflow_step_id: Uuid) -> Result<Vec<StepInputRow>> {
        let rows = sqlx::query_as::<_, StepInputRow>(
            "SELECT id, workflow_step_id, port_name, port_type, required, default_value, description, json_schema, created_at
             FROM step_inputs
             WHERE workflow_step_id = $1
             ORDER BY port_name"
        )
        .bind(workflow_step_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn get_step_outputs(&self, workflow_step_id: Uuid) -> Result<Vec<StepOutputRow>> {
        let rows = sqlx::query_as::<_, StepOutputRow>(
            "SELECT id, workflow_step_id, port_name, port_type, json_path, description, json_schema, created_at
             FROM step_outputs
             WHERE workflow_step_id = $1
             ORDER BY port_name"
        )
        .bind(workflow_step_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn create_step_input(&self, input: CreateStepInputPort) -> Result<StepInputRow> {
        let row = sqlx::query_as::<_, StepInputRow>(
            "INSERT INTO step_inputs (workflow_step_id, port_name, port_type, required, default_value, description, json_schema)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             RETURNING id, workflow_step_id, port_name, port_type, required, default_value, description, json_schema, created_at"
        )
        .bind(input.workflow_step_id)
        .bind(&input.port_name)
        .bind(&input.port_type)
        .bind(input.required)
        .bind(input.default_value)
        .bind(input.description)
        .bind(input.json_schema)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn create_step_output(
        &self,
        workflow_step_id: Uuid,
        port_name: &str,
        port_type: &str,
        json_path: &str,
        description: Option<String>,
        json_schema: Option<serde_json::Value>,
    ) -> Result<StepOutputRow> {
        let row = sqlx::query_as::<_, StepOutputRow>(
            "INSERT INTO step_outputs (workflow_step_id, port_name, port_type, json_path, description, json_schema)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING id, workflow_step_id, port_name, port_type, json_path, description, json_schema, created_at"
        )
        .bind(workflow_step_id)
        .bind(port_name)
        .bind(port_type)
        .bind(json_path)
        .bind(description)
        .bind(json_schema)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn delete_step_input(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM step_inputs WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn delete_step_output(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM step_outputs WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // --- Routing Rules (Phase 3) ---

    async fn get_step_routing_rules(
        &self,
        workflow_step_id: Uuid,
    ) -> Result<Vec<StepRoutingRuleRow>> {
        let rows = sqlx::query_as::<_, StepRoutingRuleRow>(
            "SELECT id, workflow_step_id, label_value, description, agent_id, display_order, created_at
             FROM step_routing_rules
             WHERE workflow_step_id = $1
             ORDER BY display_order, label_value"
        )
        .bind(workflow_step_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn create_routing_rule(
        &self,
        workflow_step_id: Uuid,
        label_value: &str,
        agent_id: Uuid,
        description: Option<String>,
        display_order: i32,
    ) -> Result<StepRoutingRuleRow> {
        let row = sqlx::query_as::<_, StepRoutingRuleRow>(
            "INSERT INTO step_routing_rules (workflow_step_id, label_value, agent_id, description, display_order)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING id, workflow_step_id, label_value, description, agent_id, display_order, created_at"
        )
        .bind(workflow_step_id)
        .bind(label_value)
        .bind(agent_id)
        .bind(description)
        .bind(display_order)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn update_routing_rule(
        &self,
        id: Uuid,
        agent_id: Option<Uuid>,
        description: Option<String>,
        display_order: Option<i32>,
    ) -> Result<StepRoutingRuleRow> {
        let row = sqlx::query_as::<_, StepRoutingRuleRow>(
            "UPDATE step_routing_rules SET
                agent_id = COALESCE($2, agent_id),
                description = COALESCE($3, description),
                display_order = COALESCE($4, display_order)
             WHERE id = $1
             RETURNING id, workflow_step_id, label_value, description, agent_id, display_order, created_at"
        )
        .bind(id)
        .bind(agent_id)
        .bind(description)
        .bind(display_order)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn delete_routing_rule(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM step_routing_rules WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn find_step_by_room_id(&self, room_id: Uuid) -> Result<Option<WorkflowStepRow>> {
        let row = sqlx::query_as::<_, WorkflowStepRow>(
            "SELECT * FROM workflow_steps WHERE room_id = $1 LIMIT 1",
        )
        .bind(room_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    // --- Workforce (Mission Briefs + Agent Roster) ---

    async fn get_mission_brief(&self, step_id: Uuid) -> Result<Option<TaskMissionBriefRow>> {
        let row = sqlx::query_as::<_, TaskMissionBriefRow>(
            "SELECT * FROM task_mission_briefs WHERE step_id = $1",
        )
        .bind(step_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn upsert_mission_brief(
        &self,
        step_id: Uuid,
        task_description: &str,
        available_capabilities: &[String],
        failure_mode: &str,
        downstream_context: Option<String>,
    ) -> Result<TaskMissionBriefRow> {
        let row = sqlx::query_as::<_, TaskMissionBriefRow>(
            "INSERT INTO task_mission_briefs (step_id, task_description, available_capabilities, failure_mode, downstream_context)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (step_id) DO UPDATE SET
                task_description = EXCLUDED.task_description,
                available_capabilities = EXCLUDED.available_capabilities,
                failure_mode = EXCLUDED.failure_mode,
                downstream_context = EXCLUDED.downstream_context,
                updated_at = now()
             RETURNING *",
        )
        .bind(step_id)
        .bind(task_description)
        .bind(available_capabilities)
        .bind(failure_mode)
        .bind(downstream_context)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn list_agent_roster(&self, mission_brief_id: Uuid) -> Result<Vec<TaskAgentRosterRow>> {
        let rows = sqlx::query_as::<_, TaskAgentRosterRow>(
            "SELECT * FROM task_agent_roster WHERE mission_brief_id = $1 ORDER BY execution_order",
        )
        .bind(mission_brief_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn add_roster_agent(
        &self,
        mission_brief_id: Uuid,
        name: &str,
        role_description: &str,
        capabilities: &[String],
        execution_order: i32,
    ) -> Result<TaskAgentRosterRow> {
        let row = sqlx::query_as::<_, TaskAgentRosterRow>(
            "INSERT INTO task_agent_roster (mission_brief_id, name, role_description, capabilities, execution_order)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING *",
        )
        .bind(mission_brief_id)
        .bind(name)
        .bind(role_description)
        .bind(capabilities)
        .bind(execution_order)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn update_roster_agent(
        &self,
        agent_id: Uuid,
        name: Option<String>,
        role_description: Option<String>,
        capabilities: Option<Vec<String>>,
    ) -> Result<TaskAgentRosterRow> {
        let row = sqlx::query_as::<_, TaskAgentRosterRow>(
            "UPDATE task_agent_roster SET
                name = COALESCE($2, name),
                role_description = COALESCE($3, role_description),
                capabilities = COALESCE($4, capabilities)
             WHERE id = $1
             RETURNING *",
        )
        .bind(agent_id)
        .bind(name)
        .bind(role_description)
        .bind(capabilities)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn remove_roster_agent(&self, agent_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM task_agent_roster WHERE id = $1")
            .bind(agent_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn update_roster_agent_order(&self, agent_id: Uuid, execution_order: i32) -> Result<()> {
        sqlx::query("UPDATE task_agent_roster SET execution_order = $2 WHERE id = $1")
            .bind(agent_id)
            .bind(execution_order)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn link_roster_agent_to_child_step(
        &self,
        agent_id: Uuid,
        child_step_id: Option<Uuid>,
    ) -> Result<()> {
        sqlx::query("UPDATE task_agent_roster SET child_step_id = $1 WHERE id = $2")
            .bind(child_step_id)
            .bind(agent_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // --- Belief Capture (Extraction Plans) ---

    async fn get_extraction_plan(&self, step_id: Uuid) -> Result<Option<BeliefExtractionPlanRow>> {
        let row = sqlx::query_as::<_, BeliefExtractionPlanRow>(
            "SELECT * FROM belief_extraction_plans WHERE step_id = $1",
        )
        .bind(step_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn upsert_extraction_plan(
        &self,
        step_id: Uuid,
        extraction_focus: &str,
        tag_vocabulary: &[String],
        contradiction_handling: &str,
        confidence_threshold: &str,
    ) -> Result<BeliefExtractionPlanRow> {
        let row = sqlx::query_as::<_, BeliefExtractionPlanRow>(
            "INSERT INTO belief_extraction_plans (step_id, extraction_focus, tag_vocabulary, contradiction_handling, confidence_threshold)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (step_id) DO UPDATE SET
                extraction_focus = EXCLUDED.extraction_focus,
                tag_vocabulary = EXCLUDED.tag_vocabulary,
                contradiction_handling = EXCLUDED.contradiction_handling,
                confidence_threshold = EXCLUDED.confidence_threshold,
                updated_at = now()
             RETURNING *",
        )
        .bind(step_id)
        .bind(extraction_focus)
        .bind(tag_vocabulary)
        .bind(contradiction_handling)
        .bind(confidence_threshold)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    // --- Belief Capture (Runtime Beliefs) ---

    async fn insert_belief(&self, belief: &BeliefRow) -> Result<BeliefRow> {
        let row = sqlx::query_as::<_, BeliefRow>(
            "INSERT INTO beliefs (
                id, workflow_id, workflow_execution_id, source_step_id,
                source_document_title, source_document_def_id, source_phase,
                content, reasoning, belief_type, confidence,
                confidence_justification, semantic_tags, emotional_tone,
                cross_source_tension, source_step_name, extraction_model,
                extraction_tokens_in, extraction_tokens_out
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                $11, $12, $13, $14, $15, $16, $17, $18, $19
            ) RETURNING *",
        )
        .bind(belief.id)
        .bind(belief.workflow_id)
        .bind(belief.workflow_execution_id)
        .bind(belief.source_step_id)
        .bind(&belief.source_document_title)
        .bind(belief.source_document_def_id)
        .bind(&belief.source_phase)
        .bind(&belief.content)
        .bind(&belief.reasoning)
        .bind(&belief.belief_type)
        .bind(&belief.confidence)
        .bind(&belief.confidence_justification)
        .bind(&belief.semantic_tags)
        .bind(&belief.emotional_tone)
        .bind(&belief.cross_source_tension)
        .bind(&belief.source_step_name)
        .bind(&belief.extraction_model)
        .bind(belief.extraction_tokens_in)
        .bind(belief.extraction_tokens_out)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn list_beliefs_for_execution(
        &self,
        workflow_execution_id: Uuid,
    ) -> Result<Vec<BeliefRow>> {
        let rows = sqlx::query_as::<_, BeliefRow>(
            "SELECT * FROM beliefs WHERE workflow_execution_id = $1 ORDER BY created_at",
        )
        .bind(workflow_execution_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    // --- Chat Beliefs ---

    async fn replace_chat_beliefs(
        &self,
        step_id: Uuid,
        beliefs: &[BeliefRow],
    ) -> Result<Vec<BeliefRow>> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("DELETE FROM beliefs WHERE source_step_id = $1 AND source_phase = 'chat'")
            .bind(step_id)
            .execute(&mut *tx)
            .await?;

        let mut inserted = Vec::with_capacity(beliefs.len());
        for belief in beliefs {
            let row = sqlx::query_as::<_, BeliefRow>(
                "INSERT INTO beliefs (
                    id, workflow_id, workflow_execution_id, source_step_id,
                    source_document_title, source_document_def_id, source_phase,
                    content, reasoning, belief_type, confidence,
                    confidence_justification, semantic_tags, emotional_tone,
                    cross_source_tension, source_step_name, extraction_model,
                    extraction_tokens_in, extraction_tokens_out
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                    $11, $12, $13, $14, $15, $16, $17, $18, $19
                ) RETURNING *",
            )
            .bind(belief.id)
            .bind(belief.workflow_id)
            .bind(belief.workflow_execution_id)
            .bind(belief.source_step_id)
            .bind(&belief.source_document_title)
            .bind(belief.source_document_def_id)
            .bind(&belief.source_phase)
            .bind(&belief.content)
            .bind(&belief.reasoning)
            .bind(&belief.belief_type)
            .bind(&belief.confidence)
            .bind(&belief.confidence_justification)
            .bind(&belief.semantic_tags)
            .bind(&belief.emotional_tone)
            .bind(&belief.cross_source_tension)
            .bind(&belief.source_step_name)
            .bind(&belief.extraction_model)
            .bind(belief.extraction_tokens_in)
            .bind(belief.extraction_tokens_out)
            .fetch_one(&mut *tx)
            .await?;
            inserted.push(row);
        }

        tx.commit().await?;
        Ok(inserted)
    }

    async fn get_beliefs_for_connected_steps(
        &self,
        workflow_id: Uuid,
        step_id: Uuid,
    ) -> Result<Vec<BeliefRow>> {
        let rows = sqlx::query_as::<_, BeliefRow>(
            "SELECT b.* FROM beliefs b
             WHERE b.source_phase = 'chat'
             AND b.source_step_id IN (
                 SELECT e.from_step_id FROM workflow_step_edges e
                 WHERE e.to_step_id = $1 AND e.workflow_id = $2
                 UNION
                 SELECT e.to_step_id FROM workflow_step_edges e
                 WHERE e.from_step_id = $1 AND e.workflow_id = $2
             )
             ORDER BY b.source_step_name, b.belief_type, b.created_at",
        )
        .bind(step_id)
        .bind(workflow_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    // --- Room Step Config (Design-Time) ---

    async fn get_room_step_config(&self, step_id: Uuid) -> Result<Option<RoomStepConfigRow>> {
        let row = sqlx::query_as::<_, RoomStepConfigRow>(
            "SELECT * FROM room_step_configs WHERE step_id = $1",
        )
        .bind(step_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn upsert_room_step_config(
        &self,
        step_id: Uuid,
        meeting_purpose: &str,
        max_turns: i32,
        interaction_mode: &str,
        gatekeeper_enabled: bool,
    ) -> Result<RoomStepConfigRow> {
        let row = sqlx::query_as::<_, RoomStepConfigRow>(
            "INSERT INTO room_step_configs (step_id, meeting_purpose, max_turns, interaction_mode, gatekeeper_enabled)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (step_id) DO UPDATE SET
                meeting_purpose = EXCLUDED.meeting_purpose,
                max_turns = EXCLUDED.max_turns,
                interaction_mode = EXCLUDED.interaction_mode,
                gatekeeper_enabled = EXCLUDED.gatekeeper_enabled,
                updated_at = now()
             RETURNING *",
        )
        .bind(step_id)
        .bind(meeting_purpose)
        .bind(max_turns)
        .bind(interaction_mode)
        .bind(gatekeeper_enabled)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn list_room_step_members(&self, step_id: Uuid) -> Result<Vec<RoomStepMemberRow>> {
        let rows = sqlx::query_as::<_, RoomStepMemberRow>(
            "SELECT * FROM room_step_members WHERE step_id = $1 ORDER BY display_order",
        )
        .bind(step_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn add_room_step_member(
        &self,
        step_id: Uuid,
        name: &str,
        role: &str,
        perspective: &str,
        display_order: i32,
    ) -> Result<RoomStepMemberRow> {
        let row = sqlx::query_as::<_, RoomStepMemberRow>(
            "INSERT INTO room_step_members (step_id, name, role, perspective, display_order)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING *",
        )
        .bind(step_id)
        .bind(name)
        .bind(role)
        .bind(perspective)
        .bind(display_order)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn update_room_step_member(
        &self,
        member_id: Uuid,
        name: Option<String>,
        role: Option<String>,
        perspective: Option<String>,
    ) -> Result<RoomStepMemberRow> {
        let row = sqlx::query_as::<_, RoomStepMemberRow>(
            "UPDATE room_step_members SET
                name = COALESCE($2, name),
                role = COALESCE($3, role),
                perspective = COALESCE($4, perspective),
                updated_at = now()
             WHERE id = $1
             RETURNING *",
        )
        .bind(member_id)
        .bind(name)
        .bind(role)
        .bind(perspective)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn remove_room_step_member(&self, member_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM room_step_members WHERE id = $1")
            .bind(member_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // --- Agent Designer ---

    async fn create_designer_run(
        &self,
        workflow_execution_id: Uuid,
        stage_execution_id: Uuid,
        step_id: Uuid,
        mission_brief_id: Uuid,
        model_id: &str,
    ) -> Result<AgentDesignerRunRow> {
        let row = sqlx::query_as::<_, AgentDesignerRunRow>(
            "INSERT INTO agent_designer_runs \
             (workflow_execution_id, stage_execution_id, step_id, mission_brief_id, archetype, phase, model_id) \
             VALUES ($1, $2, $3, $4, 'task_force', '', $5) \
             RETURNING *",
        )
        .bind(workflow_execution_id)
        .bind(stage_execution_id)
        .bind(step_id)
        .bind(mission_brief_id)
        .bind(model_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn create_designer_run_generic(
        &self,
        workflow_execution_id: Uuid,
        stage_execution_id: Uuid,
        step_id: Uuid,
        archetype: &str,
        phase: &str,
        model_id: &str,
    ) -> Result<AgentDesignerRunRow> {
        let row = sqlx::query_as::<_, AgentDesignerRunRow>(
            "INSERT INTO agent_designer_runs \
             (workflow_execution_id, stage_execution_id, step_id, archetype, phase, model_id) \
             VALUES ($1, $2, $3, $4, $5, $6) \
             RETURNING *",
        )
        .bind(workflow_execution_id)
        .bind(stage_execution_id)
        .bind(step_id)
        .bind(archetype)
        .bind(phase)
        .bind(model_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn update_designer_run_tokens(
        &self,
        run_id: Uuid,
        input_tokens: i64,
        output_tokens: i64,
        cost_usd: f32,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE agent_designer_runs SET input_tokens = $2, output_tokens = $3, cost_usd = $4 WHERE id = $1",
        )
        .bind(run_id)
        .bind(input_tokens)
        .bind(output_tokens)
        .bind(cost_usd)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn create_designer_output(
        &self,
        input: CreateDesignerOutputInput,
    ) -> Result<AgentDesignerOutputRow> {
        let row = sqlx::query_as::<_, AgentDesignerOutputRow>(
            "INSERT INTO agent_designer_outputs \
             (designer_run_id, agent_roster_entry_id, agent_name, assigned_tools, \
              generated_system_prompt, generated_task_prompt, design_reasoning, execution_order, \
              source_entity_id, source_archetype) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'task_force') \
             RETURNING *",
        )
        .bind(input.designer_run_id)
        .bind(input.agent_roster_entry_id)
        .bind(&input.agent_name)
        .bind(&input.assigned_tools)
        .bind(&input.generated_system_prompt)
        .bind(&input.generated_task_prompt)
        .bind(&input.design_reasoning)
        .bind(input.execution_order)
        .bind(input.agent_roster_entry_id.to_string())
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn create_designer_output_generic(
        &self,
        input: CreateDesignerOutputGenericInput,
    ) -> Result<AgentDesignerOutputRow> {
        let row = sqlx::query_as::<_, AgentDesignerOutputRow>(
            "INSERT INTO agent_designer_outputs \
             (designer_run_id, source_entity_id, source_archetype, agent_name, assigned_tools, \
              generated_system_prompt, generated_task_prompt, design_reasoning, execution_order, \
              protocol_execution_id) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) \
             RETURNING *",
        )
        .bind(input.designer_run_id)
        .bind(&input.source_entity_id)
        .bind(&input.source_archetype)
        .bind(&input.agent_name)
        .bind(&input.assigned_tools)
        .bind(&input.generated_system_prompt)
        .bind(&input.generated_task_prompt)
        .bind(&input.design_reasoning)
        .bind(input.execution_order)
        .bind(input.protocol_execution_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn list_designer_outputs(
        &self,
        designer_run_id: Uuid,
    ) -> Result<Vec<AgentDesignerOutputRow>> {
        let rows = sqlx::query_as::<_, AgentDesignerOutputRow>(
            "SELECT * FROM agent_designer_outputs WHERE designer_run_id = $1 ORDER BY execution_order",
        )
        .bind(designer_run_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn list_designer_outputs_by_protocol_execution(
        &self,
        protocol_execution_id: Uuid,
    ) -> Result<Vec<AgentDesignerOutputRow>> {
        let rows = sqlx::query_as::<_, AgentDesignerOutputRow>(
            "SELECT * FROM agent_designer_outputs \
             WHERE protocol_execution_id = $1 \
             ORDER BY execution_order",
        )
        .bind(protocol_execution_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn list_designer_runs_for_step(
        &self,
        step_id: Uuid,
        workflow_execution_id: Uuid,
    ) -> Result<Vec<AgentDesignerRunRow>> {
        let rows = sqlx::query_as::<_, AgentDesignerRunRow>(
            "SELECT * FROM agent_designer_runs \
             WHERE step_id = $1 AND workflow_execution_id = $2 \
             ORDER BY created_at",
        )
        .bind(step_id)
        .bind(workflow_execution_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn get_latest_designer_run_for_step(
        &self,
        step_id: Uuid,
    ) -> Result<Option<AgentDesignerRunRow>> {
        let row = sqlx::query_as::<_, AgentDesignerRunRow>(
            "SELECT * FROM agent_designer_runs \
             WHERE step_id = $1 \
             ORDER BY created_at DESC \
             LIMIT 1",
        )
        .bind(step_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    // --- Step Plan ---

    async fn get_plan(&self, step_id: Uuid) -> Result<Option<String>> {
        let row =
            sqlx::query_scalar::<_, String>("SELECT content FROM step_plan WHERE step_id = $1")
                .bind(step_id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row)
    }

    async fn upsert_plan(&self, step_id: Uuid, content: &str) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO step_plan (step_id, content, updated_at)
            VALUES ($1, $2, now())
            ON CONFLICT (step_id) DO UPDATE
            SET content = $2, updated_at = now()
            "#,
        )
        .bind(step_id)
        .bind(content)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_all_plans_for_workflow(
        &self,
        workflow_id: Uuid,
    ) -> Result<Vec<(Uuid, Option<String>, String, String)>> {
        let rows = sqlx::query_as::<_, (Uuid, Option<String>, String, String)>(
            r#"
            SELECT ws.id, ws.name, ws.execution_mode, sp.content
            FROM workflow_steps ws
            JOIN step_plan sp ON sp.step_id = ws.id
            WHERE ws.workflow_id = $1 AND sp.content != ''
            ORDER BY ws.name
            "#,
        )
        .bind(workflow_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    // --- Board Overview Summary ---

    async fn get_board_overview_summary(&self, workflow_id: Uuid) -> Result<String> {
        let summary = sqlx::query_scalar::<_, String>(
            "SELECT board_overview_summary FROM workflows WHERE id = $1",
        )
        .bind(workflow_id)
        .fetch_optional(&self.pool)
        .await?
        .unwrap_or_default();
        Ok(summary)
    }

    async fn update_board_overview_summary(&self, workflow_id: Uuid, summary: &str) -> Result<()> {
        sqlx::query("UPDATE workflows SET board_overview_summary = $1 WHERE id = $2")
            .bind(summary)
            .bind(workflow_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // --- Step Question State ---

    async fn get_step_question_state(&self, step_id: Uuid) -> Result<Option<StepQuestionStateRow>> {
        let row = sqlx::query_as::<_, StepQuestionStateRow>(
            "SELECT step_id, status_text, question_text, updated_at \
             FROM step_question_state WHERE step_id = $1",
        )
        .bind(step_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn get_step_question_states(
        &self,
        step_ids: &[Uuid],
    ) -> Result<Vec<StepQuestionStateRow>> {
        let rows = sqlx::query_as::<_, StepQuestionStateRow>(
            "SELECT step_id, status_text, question_text, updated_at \
             FROM step_question_state WHERE step_id = ANY($1)",
        )
        .bind(step_ids)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn upsert_step_question_state(
        &self,
        step_id: Uuid,
        status_text: &str,
        question_text: Option<String>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO step_question_state (step_id, status_text, question_text, updated_at)
            VALUES ($1, $2, $3, now())
            ON CONFLICT (step_id) DO UPDATE
            SET status_text = $2, question_text = $3, updated_at = now()
            "#,
        )
        .bind(step_id)
        .bind(status_text)
        .bind(question_text)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // --- Run Templates ---

    async fn create_template(
        &self,
        workflow_id: Uuid,
        user_id: Uuid,
        name: &str,
        description: Option<String>,
        snapshot: serde_json::Value,
    ) -> Result<RunTemplateRow> {
        let row = sqlx::query_as::<_, RunTemplateRow>(
            "INSERT INTO run_templates (workflow_id, user_id, name, description, snapshot) \
             VALUES ($1, $2, $3, $4, $5) RETURNING *",
        )
        .bind(workflow_id)
        .bind(user_id)
        .bind(name)
        .bind(description)
        .bind(&snapshot)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn get_template(&self, template_id: Uuid) -> Result<Option<RunTemplateRow>> {
        let row = sqlx::query_as::<_, RunTemplateRow>("SELECT * FROM run_templates WHERE id = $1")
            .bind(template_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row)
    }

    async fn list_templates(&self, workflow_id: Uuid) -> Result<Vec<RunTemplateRow>> {
        let rows = sqlx::query_as::<_, RunTemplateRow>(
            "SELECT id, workflow_id, user_id, name, description, \
             '{}'::jsonb AS snapshot, created_at \
             FROM run_templates WHERE workflow_id = $1 ORDER BY created_at DESC",
        )
        .bind(workflow_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn delete_template(&self, template_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM run_templates WHERE id = $1")
            .bind(template_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_canvas_snapshot(&self, workflow_id: Uuid) -> Result<Option<CanvasSnapshotRow>> {
        let row = sqlx::query_as::<_, CanvasSnapshotRow>(
            "SELECT * FROM canvas_snapshots WHERE workflow_id = $1",
        )
        .bind(workflow_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn upsert_canvas_snapshot(&self, row: CanvasSnapshotRow) -> Result<CanvasSnapshotRow> {
        let result = sqlx::query_as::<_, CanvasSnapshotRow>(
            r#"INSERT INTO canvas_snapshots (workflow_id, snapshot_json, elements_json, last_response_json, created_at, updated_at)
               VALUES ($1, $2, $3, $4, NOW(), NOW())
               ON CONFLICT (workflow_id) DO UPDATE
               SET snapshot_json = $2, elements_json = $3, last_response_json = $4, updated_at = NOW()
               RETURNING *"#,
        )
        .bind(row.workflow_id)
        .bind(&row.snapshot_json)
        .bind(&row.elements_json)
        .bind(&row.last_response_json)
        .fetch_one(&self.pool)
        .await?;
        Ok(result)
    }

    async fn update_canvas_snapshot_response(
        &self,
        workflow_id: Uuid,
        response_json: String,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE canvas_snapshots SET last_response_json = $1, updated_at = NOW() WHERE workflow_id = $2",
        )
        .bind(&response_json)
        .bind(workflow_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn list_element_maps(&self, workflow_id: Uuid) -> Result<Vec<CanvasElementMapRow>> {
        let rows = sqlx::query_as::<_, CanvasElementMapRow>(
            "SELECT * FROM canvas_element_maps WHERE workflow_id = $1",
        )
        .bind(workflow_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn upsert_element_map(&self, row: CanvasElementMapRow) -> Result<CanvasElementMapRow> {
        let result = sqlx::query_as::<_, CanvasElementMapRow>(
            r#"INSERT INTO canvas_element_maps (workflow_id, element_id, step_id, edge_id, created_at)
               VALUES ($1, $2, $3, $4, NOW())
               ON CONFLICT (workflow_id, element_id) DO UPDATE
               SET step_id = $3, edge_id = $4
               RETURNING *"#,
        )
        .bind(row.workflow_id)
        .bind(&row.element_id)
        .bind(row.step_id)
        .bind(row.edge_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(result)
    }

    async fn delete_element_map(&self, workflow_id: Uuid, element_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM canvas_element_maps WHERE workflow_id = $1 AND element_id = $2")
            .bind(workflow_id)
            .bind(element_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn upsert_step_image(&self, step_id: Uuid, stroke_image_base64: &str) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO step_images (step_id, stroke_image_base64, updated_at)
               VALUES ($1, $2, NOW())
               ON CONFLICT (step_id) DO UPDATE
               SET stroke_image_base64 = $2, updated_at = NOW()"#,
        )
        .bind(step_id)
        .bind(stroke_image_base64)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_step_stroke_image(&self, step_id: Uuid) -> Result<Option<String>> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT stroke_image_base64 FROM step_images WHERE step_id = $1")
                .bind(step_id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row.map(|r| r.0).filter(|s| !s.is_empty()))
    }
}
