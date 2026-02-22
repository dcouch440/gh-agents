//! Shared test fixtures for database row types.
//!
//! These helpers use `..Default::default()` so that adding new fields
//! to row structs requires ZERO changes in test files.

#[cfg(test)]
pub mod fixtures {
    use crate::db::*;
    use crate::types::{ExecutionMetadata, ExecutionStatus, StepExecutionEnvelope};
    use uuid::Uuid;

    // ── WorkflowStepRow ────────────────────────────────────────────

    /// Minimal step with auto-generated IDs.
    pub fn step() -> WorkflowStepRow {
        WorkflowStepRow {
            id: Uuid::new_v4(),
            workflow_id: Uuid::new_v4(),
            agent_id: Some(Uuid::new_v4()),
            ..Default::default()
        }
    }

    /// Step with specific id, prompt, variable name, and display order.
    pub fn step_with(
        id: Uuid,
        prompt: &str,
        var_name: Option<&str>,
        display_order: i32,
    ) -> WorkflowStepRow {
        WorkflowStepRow {
            id,
            workflow_id: Uuid::new_v4(),
            agent_id: Some(Uuid::new_v4()),
            prompt_template: prompt.into(),
            output_variable_name: var_name.map(String::from),
            display_order,
            ..Default::default()
        }
    }

    /// Workforce step with iteration config.
    pub fn workforce_step_with(
        id: Uuid,
        var_name: Option<&str>,
        display_order: i32,
    ) -> WorkflowStepRow {
        WorkflowStepRow {
            id,
            workflow_id: Uuid::new_v4(),
            agent_id: Some(Uuid::new_v4()),
            execution_mode: "workforce".into(),
            agent_execution_mode: Some("parallel".into()),
            output_variable_name: var_name.map(String::from),
            display_order,
            ..Default::default()
        }
    }

    /// Step scoped to a specific workflow.
    pub fn step_in(workflow_id: Uuid) -> WorkflowStepRow {
        WorkflowStepRow {
            id: Uuid::new_v4(),
            workflow_id,
            ..Default::default()
        }
    }

    // ── WorkflowStepEdgeRow ────────────────────────────────────────

    /// Edge between two steps (auto-generated IDs).
    pub fn edge(from: Uuid, to: Uuid) -> WorkflowStepEdgeRow {
        WorkflowStepEdgeRow {
            id: Uuid::new_v4(),
            from_step_id: from,
            to_step_id: to,
            workflow_id: Uuid::new_v4(),
            ..Default::default()
        }
    }

    /// Edge scoped to a specific workflow.
    pub fn edge_in(workflow_id: Uuid, from: Uuid, to: Uuid) -> WorkflowStepEdgeRow {
        WorkflowStepEdgeRow {
            id: Uuid::new_v4(),
            workflow_id,
            from_step_id: from,
            to_step_id: to,
            ..Default::default()
        }
    }

    /// Edge with port mapping.
    pub fn port_edge(from: Uuid, to: Uuid, from_port: &str, to_port: &str) -> WorkflowStepEdgeRow {
        WorkflowStepEdgeRow {
            from_output_port: Some(from_port.into()),
            to_input_port: Some(to_port.into()),
            ..edge(from, to)
        }
    }

    /// Edge with condition fields.
    pub fn conditional_edge(
        from: Uuid,
        to: Uuid,
        condition_type: &str,
        condition_field: &str,
        condition_value: &str,
    ) -> WorkflowStepEdgeRow {
        WorkflowStepEdgeRow {
            condition_type: Some(condition_type.into()),
            condition_value: Some(
                serde_json::json!({ "field": condition_field, "value": condition_value }),
            ),
            ..edge(from, to)
        }
    }

    /// Edge with both port mapping and conditions.
    pub fn conditional_port_edge(
        from: Uuid,
        to: Uuid,
        from_port: &str,
        to_port: &str,
        condition_type: &str,
        condition_field: &str,
        condition_value: &str,
    ) -> WorkflowStepEdgeRow {
        WorkflowStepEdgeRow {
            from_output_port: Some(from_port.into()),
            to_input_port: Some(to_port.into()),
            condition_type: Some(condition_type.into()),
            condition_value: Some(
                serde_json::json!({ "field": condition_field, "value": condition_value }),
            ),
            ..edge(from, to)
        }
    }

    // ── StepExecutionEnvelope ──────────────────────────────────────

    /// Successful envelope with JSON data.
    pub fn envelope(data: serde_json::Value) -> StepExecutionEnvelope {
        StepExecutionEnvelope {
            status: ExecutionStatus::Success,
            data: Some(data),
            metadata: ExecutionMetadata::new(Uuid::new_v4()),
            error: None,
        }
    }

    /// Empty successful envelope (no data).
    pub fn empty_envelope() -> StepExecutionEnvelope {
        StepExecutionEnvelope {
            status: ExecutionStatus::Success,
            data: None,
            metadata: ExecutionMetadata::new(Uuid::new_v4()),
            error: None,
        }
    }

    // ── AgentRow ───────────────────────────────────────────────────

    /// Agent with a given ID.
    pub fn agent(id: Uuid) -> AgentRow {
        AgentRow {
            id,
            name: "Test Agent".into(),
            system_prompt: "You are a test agent.".into(),
            ..Default::default()
        }
    }

    /// Agent owned by a specific user.
    pub fn agent_owned(id: Uuid, user_id: Uuid) -> AgentRow {
        AgentRow {
            user_id: Some(user_id),
            status: Some("idle".into()),
            ..agent(id)
        }
    }

    /// System agent (no owner, is_system = true).
    pub fn system_agent(id: Uuid) -> AgentRow {
        AgentRow {
            user_id: None,
            is_system: true,
            ..agent(id)
        }
    }

    // ── WorkflowRow ────────────────────────────────────────────────

    /// Workflow owned by a given user.
    pub fn workflow(user_id: Uuid) -> WorkflowRow {
        WorkflowRow {
            id: Uuid::new_v4(),
            user_id,
            name: "Test Workflow".into(),
            ..Default::default()
        }
    }

    // ── TaskMissionBriefRow ────────────────────────────────────────

    /// Mission brief for a given step.
    pub fn brief(step_id: Uuid) -> TaskMissionBriefRow {
        TaskMissionBriefRow {
            id: Uuid::new_v4(),
            step_id,
            ..Default::default()
        }
    }

    // ── TaskAgentRosterRow ─────────────────────────────────────────

    /// Roster agent entry.
    pub fn roster_agent(brief_id: Uuid, name: &str, order: i32) -> TaskAgentRosterRow {
        TaskAgentRosterRow {
            id: Uuid::new_v4(),
            mission_brief_id: brief_id,
            name: name.into(),
            role_description: format!("{name} role"),
            execution_order: order,
            ..Default::default()
        }
    }

    // ── AgentExecutionRow ──────────────────────────────────────────

    /// Agent execution with auto-generated IDs.
    pub fn agent_execution() -> AgentExecutionRow {
        AgentExecutionRow {
            id: Uuid::new_v4(),
            agent_id: Some(Uuid::new_v4()),
            ..Default::default()
        }
    }

    // ── ExecutionMessageRow ────────────────────────────────────────

    /// Execution message with auto-generated IDs.
    pub fn execution_message(agent_execution_id: Uuid) -> ExecutionMessageRow {
        ExecutionMessageRow {
            id: Uuid::new_v4(),
            agent_execution_id,
            ..Default::default()
        }
    }

    // ── TokenLedgerRow ─────────────────────────────────────────────

    /// Token ledger entry with auto-generated IDs.
    pub fn token_ledger(user_id: Uuid) -> TokenLedgerRow {
        TokenLedgerRow {
            id: Uuid::new_v4(),
            user_id,
            ..Default::default()
        }
    }

    // ── StepInputRow / StepOutputRow ───────────────────────────────

    /// Input port for a step.
    pub fn step_input(step_id: Uuid, port_name: &str, required: bool) -> StepInputRow {
        StepInputRow {
            id: Uuid::new_v4(),
            workflow_step_id: step_id,
            port_name: port_name.into(),
            required,
            ..Default::default()
        }
    }

    /// Output port for a step.
    pub fn step_output(step_id: Uuid, port_name: &str, json_path: &str) -> StepOutputRow {
        StepOutputRow {
            id: Uuid::new_v4(),
            workflow_step_id: step_id,
            port_name: port_name.into(),
            json_path: json_path.into(),
            ..Default::default()
        }
    }

    // ── BeliefRow ──────────────────────────────────────────────────

    /// Belief with minimal required fields.
    pub fn belief(workflow_id: Uuid, source_step_id: Uuid) -> BeliefRow {
        BeliefRow {
            id: Uuid::new_v4(),
            workflow_id,
            source_step_id,
            ..Default::default()
        }
    }

    // ── ToolRow ─────────────────────────────────────────────────────

    /// Tool with a given name.
    pub fn tool_row(name: &str) -> ToolRow {
        ToolRow {
            id: Uuid::new_v4(),
            name: name.into(),
            display_name: name.into(),
            description: String::new(),
            parameters: serde_json::json!({}),
            created_at: chrono::Utc::now(),
            version: 1,
        }
    }

    // ── StepRoutingRuleRow ─────────────────────────────────────────

    /// Routing rule for a step.
    pub fn routing_rule(step_id: Uuid) -> StepRoutingRuleRow {
        StepRoutingRuleRow {
            id: Uuid::new_v4(),
            workflow_step_id: step_id,
            label_value: "default".into(),
            description: None,
            agent_id: Uuid::new_v4(),
            display_order: 0,
            created_at: chrono::Utc::now(),
        }
    }

    // ── PromptTemplateRow ────────────────────────────────────────

    /// Prompt template owned by a given user.
    pub fn prompt_template(user_id: Uuid) -> PromptTemplateRow {
        PromptTemplateRow {
            id: Uuid::new_v4(),
            user_id: Some(user_id),
            name: "Test Template".into(),
            content: "Some prompt content".into(),
            created_at: chrono::Utc::now(),
            version: 1,
        }
    }

    // ── OutputSchemaRow ──────────────────────────────────────────

    /// Output schema owned by a given user.
    pub fn output_schema(user_id: Uuid) -> OutputSchemaRow {
        OutputSchemaRow {
            id: Uuid::new_v4(),
            user_id: Some(user_id),
            name: "Test Schema".into(),
            schema: serde_json::json!({"type": "object"}),
            created_at: chrono::Utc::now(),
            version: 1,
        }
    }

    // ── ProtocolRow ──────────────────────────────────────────────

    /// Protocol with sensible defaults.
    pub fn protocol_row() -> ProtocolRow {
        ProtocolRow {
            id: Uuid::new_v4(),
            name: "Test Protocol".into(),
            description: "A test protocol".into(),
            protocol_type: "workforce".into(),
            config: serde_json::json!({}),
            version: 1,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            agent_id: None,
            output_schema_id: None,
            prompt_template_id: None,
        }
    }

    // ── ResultRow ────────────────────────────────────────────────

    /// Result owned by a given user.
    pub fn result_row(user_id: Uuid) -> ResultRow {
        ResultRow {
            id: Uuid::new_v4(),
            user_id,
            agent_execution_id: Uuid::new_v4(),
            output_schema_id: None,
            name: "test result".into(),
            data: serde_json::json!({"key": "value"}),
            created_at: chrono::Utc::now(),
        }
    }

    // ── SessionRow ───────────────────────────────────────────────

    /// Chat session for a given user.
    pub fn session(user_id: Uuid) -> SessionRow {
        SessionRow {
            id: Uuid::new_v4(),
            user_id,
            mode_id: "home".into(),
            title: "Test session".into(),
            summary: String::new(),
            agent_id: None,
            draft_config: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    // ── DocumentRow ──────────────────────────────────────────────

    /// Document owned by a given user.
    pub fn document(user_id: Uuid) -> DocumentRow {
        DocumentRow {
            id: Uuid::new_v4(),
            user_id,
            session_id: None,
            title: "Test Document".into(),
            content: "Some content".into(),
            summary: None,
            doc_type: Some("architecture".into()),
            ref_tag: None,
            tags: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            workflow_id: None,
            target_length: None,
            is_static: None,
            source_protocol_step_id: None,
        }
    }

    // ── WorkflowCollectionRow ────────────────────────────────────

    /// Collection owned by a given user.
    pub fn collection(user_id: Uuid) -> WorkflowCollectionRow {
        WorkflowCollectionRow {
            id: Uuid::new_v4(),
            user_id,
            name: "Test Collection".into(),
            description: Some("A test collection".into()),
            execution_mode: "sequential".into(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }
}
