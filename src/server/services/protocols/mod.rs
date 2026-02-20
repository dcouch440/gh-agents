//! Protocol service: create, read, update, delete protocols, manage ports,
//! preview expansions, and apply protocols to workflow steps.

use std::collections::HashMap;

use once_cell::sync::Lazy;
use regex::Regex;
use uuid::Uuid;

use crate::db::traits::{
    AgentRepo, CreateProtocolInput, OutputSchemaRepo, PromptTemplateRepo, ProtocolRepo, ToolRepo,
    UpdateProtocolInput, WorkflowRepo,
};
use crate::db::{
    AgentRow, OutputSchemaRow, PromptTemplateRow, ProtocolPortRow, ProtocolRow,
    WorkflowStepEdgeRow, WorkflowStepRow,
};
use crate::server::hub::protocols::types::ProtocolExpansion;
use crate::server::hub::protocols::ProtocolEngine;

use super::error::ServiceError;

/// Valid port name pattern: lowercase alphanumeric + underscores, starting with a letter.
static PORT_NAME_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[a-z][a-z0-9_]*$").unwrap());

/// Maximum allowed port name length.
const MAX_PORT_NAME_LEN: usize = 50;

mod tests;

// ============================================================================
// Service input types
// ============================================================================

/// Input for creating a new protocol via the service layer.
pub struct CreateProtocolServiceInput {
    pub name: String,
    pub description: Option<String>,
    pub protocol_type: String,
    pub config: Option<serde_json::Value>,
    pub agent_id: Option<Uuid>,
    pub output_schema_id: Option<Uuid>,
    pub prompt_template_id: Option<Uuid>,
}

/// Result of applying a protocol to a workflow step.
pub struct ApplyResult {
    pub output_schema_id: Uuid,
    pub created_steps: Vec<CreatedStep>,
}

/// A step created during protocol application.
pub struct CreatedStep {
    pub port_name: String,
    pub step_id: Uuid,
    pub agent_id: Option<Uuid>,
}

// ============================================================================
// CRUD
// ============================================================================

/// Create a new protocol after validating the protocol type against the engine.
pub async fn create_protocol(
    proto_repo: &dyn ProtocolRepo,
    protocol_engine: &ProtocolEngine,
    input: CreateProtocolServiceInput,
) -> Result<ProtocolRow, ServiceError> {
    let known_types: Vec<String> = protocol_engine
        .list_types()
        .into_iter()
        .map(|(t, _)| t.to_string())
        .collect();
    if !known_types.contains(&input.protocol_type) {
        return Err(ServiceError::validation(format!(
            "Unknown protocol type: {}. Valid types: {}",
            input.protocol_type,
            known_types.join(", ")
        )));
    }

    let row = proto_repo
        .create_protocol(CreateProtocolInput {
            name: input.name,
            description: input.description.unwrap_or_default(),
            protocol_type: input.protocol_type,
            config: input.config.unwrap_or(serde_json::json!({})),
            agent_id: input.agent_id,
            output_schema_id: input.output_schema_id,
            prompt_template_id: input.prompt_template_id,
        })
        .await?;

    Ok(row)
}

/// Get a single protocol by ID, returning the row and its ports.
pub async fn get_protocol(
    proto_repo: &dyn ProtocolRepo,
    protocol_id: Uuid,
) -> Result<(ProtocolRow, Vec<ProtocolPortRow>), ServiceError> {
    let row = proto_repo
        .get_protocol(protocol_id)
        .await?
        .ok_or_else(|| ServiceError::not_found("Protocol"))?;

    let ports = proto_repo.list_protocol_ports(protocol_id).await?;

    Ok((row, ports))
}

/// List all protocols.
pub async fn list_protocols(
    proto_repo: &dyn ProtocolRepo,
) -> Result<Vec<ProtocolRow>, ServiceError> {
    let rows = proto_repo.list_protocols().await?;
    Ok(rows)
}

/// Update an existing protocol (partial update). Verifies the protocol exists first.
pub async fn update_protocol(
    proto_repo: &dyn ProtocolRepo,
    protocol_id: Uuid,
    input: UpdateProtocolInput,
) -> Result<ProtocolRow, ServiceError> {
    proto_repo
        .get_protocol(protocol_id)
        .await?
        .ok_or_else(|| ServiceError::not_found("Protocol"))?;

    let row = proto_repo.update_protocol(input).await?;
    Ok(row)
}

/// Delete a protocol by ID. Verifies it exists first.
pub async fn delete_protocol(
    proto_repo: &dyn ProtocolRepo,
    protocol_id: Uuid,
) -> Result<(), ServiceError> {
    proto_repo
        .get_protocol(protocol_id)
        .await?
        .ok_or_else(|| ServiceError::not_found("Protocol"))?;

    proto_repo.delete_protocol(protocol_id).await?;
    Ok(())
}

/// List all ports for a protocol.
pub async fn list_protocol_ports(
    proto_repo: &dyn ProtocolRepo,
    protocol_id: Uuid,
) -> Result<Vec<ProtocolPortRow>, ServiceError> {
    let ports = proto_repo.list_protocol_ports(protocol_id).await?;
    Ok(ports)
}

// ============================================================================
// Port management
// ============================================================================

/// Validate a port name against the allowed pattern and length.
pub(crate) fn validate_port_name(name: &str) -> Result<(), ServiceError> {
    if name.is_empty() || name.len() > MAX_PORT_NAME_LEN || !PORT_NAME_REGEX.is_match(name) {
        return Err(ServiceError::validation(format!(
            "Invalid port name \"{}\": must match [a-z][a-z0-9_]* and be at most {} characters",
            name, MAX_PORT_NAME_LEN
        )));
    }
    Ok(())
}

/// Create a new port on a protocol. Validates the port name and verifies the protocol exists.
pub async fn create_port(
    proto_repo: &dyn ProtocolRepo,
    protocol_id: Uuid,
    port_name: String,
    description: Option<String>,
    agent_id: Uuid,
    display_order: Option<i32>,
) -> Result<ProtocolPortRow, ServiceError> {
    validate_port_name(&port_name)?;

    proto_repo
        .get_protocol(protocol_id)
        .await?
        .ok_or_else(|| ServiceError::not_found("Protocol"))?;

    let port = proto_repo
        .create_protocol_port(
            protocol_id,
            port_name,
            description.unwrap_or_default(),
            agent_id,
            display_order.unwrap_or(0),
        )
        .await?;

    Ok(port)
}

/// Update an existing port. Validates the port name if provided.
pub async fn update_port(
    proto_repo: &dyn ProtocolRepo,
    port_id: Uuid,
    port_name: Option<String>,
    description: Option<String>,
    agent_id: Option<Uuid>,
    display_order: Option<i32>,
) -> Result<ProtocolPortRow, ServiceError> {
    if let Some(ref name) = port_name {
        validate_port_name(name)?;
    }

    let port = proto_repo
        .update_protocol_port(port_id, port_name, description, agent_id, display_order)
        .await?;

    Ok(port)
}

/// Delete a port by ID.
pub async fn delete_port(proto_repo: &dyn ProtocolRepo, port_id: Uuid) -> Result<(), ServiceError> {
    proto_repo.delete_protocol_port(port_id).await?;
    Ok(())
}

// ============================================================================
// Resolution helpers
// ============================================================================

/// Resolve the associated agent, output schema, and prompt template for a protocol row.
/// Returns domain types (not response types) — the handler maps these to response types.
pub(crate) async fn resolve_protocol_associations(
    agent_repo: &dyn AgentRepo,
    schema_repo: &dyn OutputSchemaRepo,
    template_repo: &dyn PromptTemplateRepo,
    protocol_row: &ProtocolRow,
) -> Result<
    (
        Option<AgentRow>,
        Option<OutputSchemaRow>,
        Option<PromptTemplateRow>,
    ),
    ServiceError,
> {
    let agent = if let Some(agent_id) = protocol_row.agent_id {
        agent_repo.get_persisted_agent(agent_id).await?
    } else {
        None
    };

    let output_schema = if let Some(schema_id) = protocol_row.output_schema_id {
        schema_repo.get_output_schema(schema_id).await?
    } else {
        None
    };

    let prompt_template = if let Some(template_id) = protocol_row.prompt_template_id {
        template_repo.get_prompt_template(template_id).await?
    } else {
        None
    };

    Ok((agent, output_schema, prompt_template))
}

/// Resolve agent names from agent IDs found in the port rows.
pub(crate) async fn resolve_agent_names(
    agent_repo: &dyn AgentRepo,
    ports: &[ProtocolPortRow],
) -> Result<HashMap<Uuid, String>, ServiceError> {
    let mut names = HashMap::new();
    for port in ports {
        if names.contains_key(&port.agent_id) {
            continue;
        }
        let agent = agent_repo
            .get_persisted_agent(port.agent_id)
            .await?
            .ok_or_else(|| {
                ServiceError::validation(format!(
                    "Agent {} not found for port {}",
                    port.agent_id, port.port_name
                ))
            })?;
        names.insert(port.agent_id, agent.name);
    }
    Ok(names)
}

/// Resolve agent output schemas from agent IDs in the port rows.
/// Only includes agents that have an `output_schema_id` set.
pub(crate) async fn resolve_agent_schemas(
    agent_repo: &dyn AgentRepo,
    schema_repo: &dyn OutputSchemaRepo,
    ports: &[ProtocolPortRow],
) -> Result<HashMap<Uuid, serde_json::Value>, ServiceError> {
    let mut schemas = HashMap::new();
    for port in ports {
        if schemas.contains_key(&port.agent_id) {
            continue;
        }
        let agent = agent_repo.get_persisted_agent(port.agent_id).await?;
        if let Some(agent) = agent {
            if let Some(schema_id) = agent.output_schema_id {
                let schema_row = schema_repo.get_output_schema(schema_id).await?;
                if let Some(row) = schema_row {
                    schemas.insert(port.agent_id, row.schema);
                }
            }
        }
    }
    Ok(schemas)
}

/// Resolve agent tool names from agent IDs in the port rows.
pub(crate) async fn resolve_agent_tools(
    tool_repo: &dyn ToolRepo,
    ports: &[ProtocolPortRow],
) -> Result<HashMap<Uuid, Vec<String>>, ServiceError> {
    let mut tools_map = HashMap::new();
    for port in ports {
        if tools_map.contains_key(&port.agent_id) {
            continue;
        }
        let tools = tool_repo.get_agent_tools(port.agent_id).await?;
        let tool_names: Vec<String> = tools.into_iter().map(|t| t.name).collect();
        tools_map.insert(port.agent_id, tool_names);
    }
    Ok(tools_map)
}

// ============================================================================
// Preview + Apply
// ============================================================================

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
            sub_workflow_template_id: None,
            child_workflow_id: None,
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
