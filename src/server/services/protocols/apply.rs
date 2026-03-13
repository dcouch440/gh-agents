//! Apply and preview protocol expansions on workflow steps.

use uuid::Uuid;

use crate::db::traits::{AgentRepo, OutputSchemaRepo, ProtocolRepo, ToolRepo, WorkflowRepo};
use crate::db::{WorkflowStepEdgeRow, WorkflowStepRow};
use crate::server::hub::protocols::types::ProtocolExpansion;
use crate::server::hub::protocols::ProtocolEngine;
use crate::server::services::error::ServiceError;

use super::resolve::{resolve_agent_names, resolve_agent_schemas, resolve_agent_tools};
use super::{ApplyResult, CreatedStep};

/// Preview the expansion of a protocol (dry run, no DB writes).
/// Loads the protocol, ports, resolves agents/tools/schemas, builds the config,
/// and returns the resulting expansion.
pub async fn preview_expansion(
    proto_repo: &dyn ProtocolRepo,
    agent_repo: &dyn AgentRepo,
    tool_repo: &dyn ToolRepo,
    schema_repo: &dyn OutputSchemaRepo,
    protocol_engine: &ProtocolEngine,
    protocol_id: Uuid,
) -> Result<ProtocolExpansion, ServiceError> {
    let protocol = proto_repo
        .get_protocol(protocol_id)
        .await?
        .ok_or_else(|| ServiceError::not_found("Protocol"))?;

    let ports = proto_repo.list_protocol_ports(protocol_id).await?;

    let agent_names = resolve_agent_names(agent_repo, &ports).await?;
    let agent_tools = resolve_agent_tools(tool_repo, &ports).await?;
    let agent_schemas = resolve_agent_schemas(agent_repo, schema_repo, &ports).await?;

    let config = protocol_engine.build_config(
        &protocol.protocol_type,
        protocol.config,
        &ports,
        &agent_names,
        &agent_tools,
        &agent_schemas,
    );

    let expansion = protocol_engine
        .preview(&config)
        .map_err(|e| ServiceError::validation(e.to_string()))?;

    Ok(expansion)
}

/// Apply a protocol to a workflow step. This is the full orchestration:
/// 1. Load protocol + ports
/// 2. Verify the target step exists
/// 3. Resolve agents, tools, schemas
/// 4. Expand the protocol
/// 5. Create output schema
/// 6. Update anchor step with schema + prompt injection
/// 7. Create downstream steps, routing rules, and edges
/// 8. Store protocol linkage snapshot
#[allow(clippy::too_many_arguments)]
pub async fn apply_protocol(
    proto_repo: &dyn ProtocolRepo,
    wf_repo: &dyn WorkflowRepo,
    os_repo: &dyn OutputSchemaRepo,
    agent_repo: &dyn AgentRepo,
    tool_repo: &dyn ToolRepo,
    protocol_engine: &ProtocolEngine,
    user_id: Uuid,
    protocol_id: Uuid,
    step_id: Uuid,
) -> Result<ApplyResult, ServiceError> {
    // Load protocol + ports
    let protocol = proto_repo
        .get_protocol(protocol_id)
        .await?
        .ok_or_else(|| ServiceError::not_found("Protocol"))?;

    let ports = proto_repo.list_protocol_ports(protocol_id).await?;

    // Verify the target step exists
    let anchor_step = wf_repo
        .get_step(step_id)
        .await?
        .ok_or_else(|| ServiceError::not_found("Workflow step"))?;

    // Resolve agent names, tools, and content schemas
    let agent_names = resolve_agent_names(agent_repo, &ports).await?;
    let agent_tools = resolve_agent_tools(tool_repo, &ports).await?;
    let agent_schemas = resolve_agent_schemas(agent_repo, os_repo, &ports).await?;

    let protocol_config_json = protocol.config.clone();

    // Expand
    let config = protocol_engine.build_config(
        &protocol.protocol_type,
        protocol_config_json,
        &ports,
        &agent_names,
        &agent_tools,
        &agent_schemas,
    );
    let expansion = protocol_engine
        .expand(&config)
        .map_err(|e| ServiceError::validation(e.to_string()))?;

    // 1. Create output schema
    let schema_name = format!("{} — auto-generated", protocol.name);
    let schema_row = os_repo
        .create_output_schema(Some(user_id), schema_name, expansion.output_schema.clone())
        .await?;

    // 2. Update anchor step with output schema and prompt injection
    let anchor_output_var = anchor_step
        .output_variable_name
        .clone()
        .unwrap_or_else(|| format!("protocol_{}", protocol_id));
    let mut updated_step = anchor_step.clone();
    updated_step.output_schema_id = Some(schema_row.id);
    updated_step.output_variable_name = Some(anchor_output_var.clone());
    if !expansion.prompt_injection.is_empty() {
        updated_step.prompt_template = format!(
            "{}\n\n{}",
            anchor_step.prompt_template, expansion.prompt_injection
        );
    }
    wf_repo.update_step(updated_step).await?;

    // 3. Create downstream steps, routing rules, and edges
    let mut created_steps = Vec::new();
    for step_def in &expansion.steps {
        let resolved_for_each_ref = step_def.for_each_ref.as_ref().map(|r| {
            if r == "{anchor_output}" {
                anchor_output_var.clone()
            } else {
                r.clone()
            }
        });

        let new_step = WorkflowStepRow {
            id: Uuid::new_v4(),
            workflow_id: anchor_step.workflow_id,
            agent_id: step_def.agent_id,
            execution_mode: step_def.execution_mode.clone(),
            agent_execution_mode: None,
            for_each_ref: resolved_for_each_ref,
            prompt_template_id: None,
            prompt_template: step_def
                .prompt_template
                .clone()
                .unwrap_or_else(|| "{task_input}".to_string()),
            output_schema_id: None,
            output_variable_name: Some(step_def.port_name.clone()),
            interactive_agent_id: None,
            for_each_label_field: step_def.for_each_label_field.clone(),
            room_id: None,
            routing_mode: step_def.routing_mode.clone(),
            routing_field: step_def.routing_field.clone(),
            display_order: created_steps.len() as i32 + anchor_step.display_order + 1,
            version: 1,
            reasoning_trace: false,
            verification_agent_ids: None,
            position_x: None,
            position_y: None,
            width: None,
            height: None,
            name: None,
            system_prompt_suffix: None,
            visible: true,
            description: String::new(),
            board_context_cache: String::new(),
            board_context_updated_at: None,
            goal_summary: String::new(),
            goal_summary_updated_at: None,

            child_workflow_id: None,
            ref_id: None,
            pinned: false,
            run_results_summary: String::new(),
        };

        let created = wf_repo.create_step(new_step).await?;

        // Create routing rules for label-routed steps
        for rule in &step_def.routing_rules {
            wf_repo
                .create_routing_rule(
                    created.id,
                    &rule.label_value,
                    rule.agent_id,
                    rule.description.clone(),
                    rule.display_order,
                )
                .await?;
        }

        created_steps.push(CreatedStep {
            port_name: step_def.port_name.clone(),
            step_id: created.id,
            agent_id: step_def.agent_id,
        });
    }

    // 4. Create edges from anchor to downstream steps
    let mut all_edges = wf_repo.list_edges(anchor_step.workflow_id).await?;

    for (edge_def, created) in expansion.edges.iter().zip(created_steps.iter()) {
        all_edges.push(WorkflowStepEdgeRow {
            id: Uuid::new_v4(),
            from_step_id: step_id,
            to_step_id: created.step_id,
            from_output_port: Some(edge_def.from_output_port.clone()),
            to_input_port: Some(edge_def.to_input_port.clone()),
            transform_jsonpath: None,
            condition_type: edge_def.condition_type.clone(),
            condition_value: edge_def.condition_value.clone(),
            edge_label: Some(edge_def.target_port_name.clone()),
            workflow_id: anchor_step.workflow_id,
        });
    }

    wf_repo
        .set_edges(anchor_step.workflow_id, all_edges)
        .await?;

    // 5. Store protocol linkage snapshot
    let snapshot =
        serde_json::to_value(&expansion).map_err(|e| ServiceError::Internal(e.into()))?;
    proto_repo
        .create_step_protocol(step_id, protocol_id, snapshot)
        .await?;

    Ok(ApplyResult {
        output_schema_id: schema_row.id,
        created_steps,
    })
}
