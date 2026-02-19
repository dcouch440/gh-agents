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

    /// For-each step with iteration config.
    pub fn for_each_step_with(
        id: Uuid,
        for_each_ref: &str,
        var_name: Option<&str>,
        display_order: i32,
    ) -> WorkflowStepRow {
        WorkflowStepRow {
            id,
            workflow_id: Uuid::new_v4(),
            agent_id: Some(Uuid::new_v4()),
            execution_mode: "for_each".into(),
            agent_execution_mode: Some("parallel".into()),
            for_each_ref: Some(for_each_ref.into()),
            output_variable_name: var_name.map(String::from),
            display_order,
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
}
