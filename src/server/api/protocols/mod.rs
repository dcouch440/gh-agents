//! Protocol management endpoints — CRUD, port management, preview, and apply.

use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::AppError;
use crate::server::auth as auth_utils;
use crate::server::hub::protocols::types::ProtocolExpansion;
use crate::server::state::AppState;

/// Valid port name pattern: lowercase alphanumeric + underscores, starting with a letter.
static PORT_NAME_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[a-z][a-z0-9_]*$").unwrap());

/// Maximum allowed port name length.
const MAX_PORT_NAME_LEN: usize = 50;

pub mod documents;
pub mod executions;

#[cfg(test)]
mod tests;

// ============================================================================
// Response / Request types
// ============================================================================

#[derive(Serialize)]
pub struct ProtocolResponse {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub protocol_type: String,
    pub config: serde_json::Value,
    pub version: i32,
    pub ports: Vec<ProtocolPortResponse>,
    pub agent: Option<ProtocolAgentResponse>,
    pub output_schema: Option<ProtocolSchemaResponse>,
    pub prompt_template: Option<ProtocolTemplateResponse>,
}

#[derive(Serialize)]
pub struct ProtocolAgentResponse {
    pub id: Uuid,
    pub name: String,
    pub system_prompt: String,
    pub model_provider: String,
    pub model_id: String,
}

#[derive(Serialize)]
pub struct ProtocolSchemaResponse {
    pub id: Uuid,
    pub name: String,
    pub schema: serde_json::Value,
}

#[derive(Serialize)]
pub struct ProtocolTemplateResponse {
    pub id: Uuid,
    pub name: String,
    pub content: String,
}

#[derive(Serialize)]
pub struct ProtocolPortResponse {
    pub id: Uuid,
    pub port_name: String,
    pub description: String,
    pub agent_id: Uuid,
    pub display_order: i32,
}

#[derive(Deserialize)]
pub struct CreateProtocolRequest {
    pub name: String,
    pub description: Option<String>,
    pub protocol_type: String,
    pub config: Option<serde_json::Value>,
    pub agent_id: Option<Uuid>,
    pub output_schema_id: Option<Uuid>,
    pub prompt_template_id: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct UpdateProtocolRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub config: Option<serde_json::Value>,
    pub agent_id: Option<Uuid>,
    pub output_schema_id: Option<Uuid>,
    pub prompt_template_id: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct CreatePortRequest {
    pub port_name: String,
    pub description: Option<String>,
    pub agent_id: Uuid,
    pub display_order: Option<i32>,
}

#[derive(Deserialize)]
pub struct UpdatePortRequest {
    pub port_name: Option<String>,
    pub description: Option<String>,
    pub agent_id: Option<Uuid>,
    pub display_order: Option<i32>,
}

#[derive(Serialize)]
pub struct PreviewResponse {
    pub expansion: ProtocolExpansion,
}

#[derive(Deserialize, Default)]
pub struct ApplyProtocolRequest {}

#[derive(Serialize)]
pub struct ApplyResponse {
    pub output_schema_id: Uuid,
    pub created_steps: Vec<CreatedStepResponse>,
}

#[derive(Serialize)]
pub struct CreatedStepResponse {
    pub port_name: String,
    pub step_id: Uuid,
    pub agent_id: Option<Uuid>,
}

#[derive(Serialize)]
pub struct ProtocolTypesResponse {
    pub types: Vec<ProtocolTypeInfo>,
}

#[derive(Serialize)]
pub struct ProtocolTypeInfo {
    pub name: String,
    pub description: String,
}

// ============================================================================
// Handlers
// ============================================================================

/// GET /api/protocols/types — List all registered protocol types.
pub async fn list_protocol_types(
    State(state): State<AppState>,
) -> Result<Json<ProtocolTypesResponse>, AppError> {
    let engine = state.protocol_engine();
    let types = engine
        .list_types()
        .into_iter()
        .map(|(name, desc)| ProtocolTypeInfo {
            name: name.to_string(),
            description: desc.to_string(),
        })
        .collect();
    Ok(Json(ProtocolTypesResponse { types }))
}

/// GET /api/protocols — List all protocols.
pub async fn list_protocols(
    State(state): State<AppState>,
) -> Result<Json<Vec<ProtocolResponse>>, AppError> {
    let repo = &state.repos().protocols;
    let rows = repo
        .list_protocols()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let mut responses = Vec::with_capacity(rows.len());
    for row in rows {
        let ports = repo
            .list_protocol_ports(row.id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let (agent, output_schema, prompt_template) =
            resolve_protocol_associations(&state, &row).await?;
        responses.push(ProtocolResponse {
            id: row.id,
            name: row.name,
            description: row.description,
            protocol_type: row.protocol_type,
            config: row.config,
            version: row.version,
            ports: ports
                .into_iter()
                .map(|p| ProtocolPortResponse {
                    id: p.id,
                    port_name: p.port_name,
                    description: p.description,
                    agent_id: p.agent_id,
                    display_order: p.display_order,
                })
                .collect(),
            agent,
            output_schema,
            prompt_template,
        });
    }

    Ok(Json(responses))
}

/// POST /api/protocols — Create a new protocol.
pub async fn create_protocol(
    State(state): State<AppState>,
    Json(request): Json<CreateProtocolRequest>,
) -> Result<(StatusCode, Json<ProtocolResponse>), AppError> {
    // Validate protocol type is known
    let engine = state.protocol_engine();
    let known_types: Vec<String> = engine
        .list_types()
        .into_iter()
        .map(|(t, _)| t.to_string())
        .collect();
    if !known_types.contains(&request.protocol_type) {
        return Err(AppError::bad_request(format!(
            "Unknown protocol type: {}. Valid types: {}",
            request.protocol_type,
            known_types.join(", ")
        )));
    }

    let repo = &state.repos().protocols;
    let row = repo
        .create_protocol(
            request.name,
            request.description.unwrap_or_default(),
            request.protocol_type,
            request.config.unwrap_or(serde_json::json!({})),
            request.agent_id,
            request.output_schema_id,
            request.prompt_template_id,
        )
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let (agent, output_schema, prompt_template) =
        resolve_protocol_associations(&state, &row).await?;

    Ok((
        StatusCode::CREATED,
        Json(ProtocolResponse {
            id: row.id,
            name: row.name,
            description: row.description,
            protocol_type: row.protocol_type,
            config: row.config,
            version: row.version,
            ports: vec![],
            agent,
            output_schema,
            prompt_template,
        }),
    ))
}

/// GET /api/protocols/:id — Get a protocol by ID.
pub async fn get_protocol(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ProtocolResponse>, AppError> {
    let repo = &state.repos().protocols;
    let row = repo
        .get_protocol(id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::not_found("protocol"))?;

    let ports = repo
        .list_protocol_ports(id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let (agent, output_schema, prompt_template) =
        resolve_protocol_associations(&state, &row).await?;

    Ok(Json(ProtocolResponse {
        id: row.id,
        name: row.name,
        description: row.description,
        protocol_type: row.protocol_type,
        config: row.config,
        version: row.version,
        ports: ports
            .into_iter()
            .map(|p| ProtocolPortResponse {
                id: p.id,
                port_name: p.port_name,
                description: p.description,
                agent_id: p.agent_id,
                display_order: p.display_order,
            })
            .collect(),
        agent,
        output_schema,
        prompt_template,
    }))
}

/// PUT /api/protocols/:id — Update a protocol.
pub async fn update_protocol(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateProtocolRequest>,
) -> Result<Json<ProtocolResponse>, AppError> {
    let repo = &state.repos().protocols;

    // Verify exists
    repo.get_protocol(id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::not_found("protocol"))?;

    let row = repo
        .update_protocol(
            id,
            request.name,
            request.description,
            request.config,
            request.agent_id,
            request.output_schema_id,
            request.prompt_template_id,
        )
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let ports = repo
        .list_protocol_ports(id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let (agent, output_schema, prompt_template) =
        resolve_protocol_associations(&state, &row).await?;

    Ok(Json(ProtocolResponse {
        id: row.id,
        name: row.name,
        description: row.description,
        protocol_type: row.protocol_type,
        config: row.config,
        version: row.version,
        ports: ports
            .into_iter()
            .map(|p| ProtocolPortResponse {
                id: p.id,
                port_name: p.port_name,
                description: p.description,
                agent_id: p.agent_id,
                display_order: p.display_order,
            })
            .collect(),
        agent,
        output_schema,
        prompt_template,
    }))
}

/// DELETE /api/protocols/:id — Delete a protocol.
pub async fn delete_protocol(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let repo = &state.repos().protocols;
    repo.get_protocol(id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::not_found("protocol"))?;

    repo.delete_protocol(id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// Port handlers
// ============================================================================

/// POST /api/protocols/:id/ports — Add a port to a protocol.
pub async fn create_port(
    State(state): State<AppState>,
    Path(protocol_id): Path<Uuid>,
    Json(request): Json<CreatePortRequest>,
) -> Result<(StatusCode, Json<ProtocolPortResponse>), AppError> {
    // Validate port name format
    if request.port_name.is_empty()
        || request.port_name.len() > MAX_PORT_NAME_LEN
        || !PORT_NAME_REGEX.is_match(&request.port_name)
    {
        return Err(AppError::bad_request(format!(
            "Invalid port name \"{}\": must match [a-z][a-z0-9_]* and be at most {} characters",
            request.port_name, MAX_PORT_NAME_LEN
        )));
    }

    let repo = &state.repos().protocols;

    // Verify protocol exists
    repo.get_protocol(protocol_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::not_found("protocol"))?;

    let port = repo
        .create_protocol_port(
            protocol_id,
            request.port_name,
            request.description.unwrap_or_default(),
            request.agent_id,
            request.display_order.unwrap_or(0),
        )
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(ProtocolPortResponse {
            id: port.id,
            port_name: port.port_name,
            description: port.description,
            agent_id: port.agent_id,
            display_order: port.display_order,
        }),
    ))
}

/// PUT /api/protocols/:protocol_id/ports/:port_id — Update a port.
pub async fn update_port(
    State(state): State<AppState>,
    Path((_, port_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<UpdatePortRequest>,
) -> Result<Json<ProtocolPortResponse>, AppError> {
    // Validate port name if being updated
    if let Some(ref name) = request.port_name {
        if name.is_empty() || name.len() > MAX_PORT_NAME_LEN || !PORT_NAME_REGEX.is_match(name) {
            return Err(AppError::bad_request(format!(
                "Invalid port name \"{}\": must match [a-z][a-z0-9_]* and be at most {} characters",
                name, MAX_PORT_NAME_LEN
            )));
        }
    }

    let repo = &state.repos().protocols;
    let port = repo
        .update_protocol_port(
            port_id,
            request.port_name,
            request.description,
            request.agent_id,
            request.display_order,
        )
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(ProtocolPortResponse {
        id: port.id,
        port_name: port.port_name,
        description: port.description,
        agent_id: port.agent_id,
        display_order: port.display_order,
    }))
}

/// DELETE /api/protocols/:protocol_id/ports/:port_id — Delete a port.
pub async fn delete_port(
    State(state): State<AppState>,
    Path((_, port_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    let repo = &state.repos().protocols;
    repo.delete_protocol_port(port_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// Preview & Apply handlers
// ============================================================================

/// POST /api/protocols/:id/preview — Preview expansion (dry run, no DB writes).
pub async fn preview_expansion(
    State(state): State<AppState>,
    Path(protocol_id): Path<Uuid>,
) -> Result<Json<PreviewResponse>, AppError> {
    let repo = &state.repos().protocols;
    let protocol = repo
        .get_protocol(protocol_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::not_found("protocol"))?;

    let ports = repo
        .list_protocol_ports(protocol_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Resolve agent names, tools, and content schemas for prompt injection
    let agent_names = resolve_agent_names(&state, &ports).await?;
    let agent_tools = resolve_agent_tools(&state, &ports).await?;
    let agent_schemas = resolve_agent_schemas(&state, &ports).await?;

    let engine = state.protocol_engine();
    let config = engine.build_config(
        &protocol.protocol_type,
        protocol.config,
        &ports,
        &agent_names,
        &agent_tools,
        &agent_schemas,
    );

    let expansion = engine
        .preview(&config)
        .map_err(|e| AppError::bad_request(e.to_string()))?;

    Ok(Json(PreviewResponse { expansion }))
}

/// POST /api/protocols/:id/apply/:step_id — Apply protocol to a workflow step.
pub async fn apply_protocol(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path((protocol_id, step_id)): Path<(Uuid, Uuid)>,
    Json(_request): Json<ApplyProtocolRequest>,
) -> Result<(StatusCode, Json<ApplyResponse>), AppError> {
    let proto_repo = &state.repos().protocols;
    let wf_repo = &state.repos().workflows;
    let os_repo = &state.repos().output_schemas;

    // Load protocol + ports
    let protocol = proto_repo
        .get_protocol(protocol_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::not_found("protocol"))?;

    let ports = proto_repo
        .list_protocol_ports(protocol_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Verify the target step exists
    let anchor_step = wf_repo
        .get_step(step_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::not_found("workflow step"))?;

    // Resolve agent names, tools, and content schemas
    let agent_names = resolve_agent_names(&state, &ports).await?;
    let agent_tools = resolve_agent_tools(&state, &ports).await?;
    let agent_schemas = resolve_agent_schemas(&state, &ports).await?;

    // For documenter protocols, inject document definitions and capabilities into config
    let protocol_config_json = if protocol.protocol_type == "documenter" {
        let doc_defs = proto_repo
            .list_protocol_document_defs(protocol_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        if doc_defs.is_empty() {
            return Err(AppError::bad_request(
                "Documenter protocol requires at least one document definition".to_string(),
            ));
        }

        let defs_json: Vec<serde_json::Value> = doc_defs
            .iter()
            .map(|d| {
                serde_json::json!({
                    "name": d.name,
                    "description": d.description,
                    "target_length": d.target_length,
                })
            })
            .collect();

        let capabilities = state
            .repos()
            .tool_capabilities
            .get_tool_capabilities()
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let cap_keys: Vec<String> = capabilities
            .iter()
            .map(|c| c.capability_key.clone())
            .collect();

        let mut config_json = protocol.config.clone();
        config_json["document_defs"] = serde_json::json!(defs_json);
        config_json["available_capabilities"] = serde_json::json!(cap_keys);
        config_json
    } else {
        protocol.config.clone()
    };

    // Expand
    let engine = state.protocol_engine();
    let config = engine.build_config(
        &protocol.protocol_type,
        protocol_config_json,
        &ports,
        &agent_names,
        &agent_tools,
        &agent_schemas,
    );
    let expansion = engine
        .expand(&config)
        .map_err(|e| AppError::bad_request(e.to_string()))?;

    // 1. Create output schema
    let schema_name = format!("{} — auto-generated", protocol.name);
    let schema_row = os_repo
        .create_output_schema(
            Some(auth.user_id.0),
            schema_name,
            expansion.output_schema.clone(),
        )
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // 2. Update anchor step with output schema and prompt injection.
    // Resolve the anchor's output variable name for for_each_ref resolution.
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
    // For documenter protocols, also set execution_mode so the DAG executor
    // dispatches to DocumenterExecutor at runtime.
    if protocol.protocol_type == "documenter" {
        updated_step.execution_mode = "documenter".to_string();
    }
    wf_repo
        .update_step(updated_step)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // 2b. Documenter-specific: scaffold blank documents and step-scoped doc defs.
    if protocol.protocol_type == "documenter" {
        let doc_defs = proto_repo
            .list_protocol_document_defs(protocol_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let doc_repo = &state.repos().documents;

        for def in &doc_defs {
            // Create blank document linked to the workflow
            let doc = doc_repo
                .create_workflow_document(
                    auth.user_id.0,
                    def.name.clone(),
                    anchor_step.workflow_id,
                    Some(def.target_length),
                    Some(step_id),
                )
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;

            // Link document to the anchor step
            wf_repo
                .add_step_document(step_id, doc.id)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;

            // Create step-scoped copy of the doc def (step_id set, protocol_id null)
            let step_scoped_def = crate::db::ProtocolDocumentDefRow {
                id: Uuid::new_v4(),
                step_id: Some(step_id),
                name: def.name.clone(),
                description: def.description.clone(),
                target_length: def.target_length,
                display_order: def.display_order,
                created_at: chrono::Utc::now(),
                protocol_id: None,
                document_id: Some(doc.id),
            };
            wf_repo
                .create_document_def(step_scoped_def)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
        }
    }

    // 3. Create downstream steps, routing rules, and edges
    let mut created_steps = Vec::new();
    for step_def in &expansion.steps {
        // Resolve the {anchor_output} sentinel in for_each_ref
        let resolved_for_each_ref = step_def.for_each_ref.as_ref().map(|r| {
            if r == "{anchor_output}" {
                anchor_output_var.clone()
            } else {
                r.clone()
            }
        });

        let new_step = crate::db::WorkflowStepRow {
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
            name: None,
            system_prompt_suffix: None,
            visible: true,
            description: String::new(),
        };

        let created = wf_repo
            .create_step(new_step)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

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
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
        }

        created_steps.push(CreatedStepResponse {
            port_name: step_def.port_name.clone(),
            step_id: created.id,
            agent_id: step_def.agent_id,
        });
    }

    // 4. Create edges from anchor → downstream steps
    let mut all_edges = wf_repo
        .list_edges(anchor_step.workflow_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Wire edges from anchor → newly created downstream steps (if any).
    for (edge_def, created) in expansion.edges.iter().zip(created_steps.iter()) {
        all_edges.push(crate::db::WorkflowStepEdgeRow {
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
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // 5. Store protocol linkage snapshot
    let snapshot =
        serde_json::to_value(&expansion).map_err(|e| AppError::Internal(e.to_string()))?;
    proto_repo
        .create_step_protocol(step_id, protocol_id, snapshot)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(ApplyResponse {
            output_schema_id: schema_row.id,
            created_steps,
        }),
    ))
}

/// DELETE /api/protocols/:protocol_id/unapply/:step_id — Remove protocol from step.
pub async fn unapply_protocol(
    State(state): State<AppState>,
    Path((_, step_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    let repo = &state.repos().protocols;
    repo.delete_step_protocol(step_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// Helpers
// ============================================================================

/// Resolve the associated agent, output schema, and prompt template for a protocol row.
async fn resolve_protocol_associations(
    state: &AppState,
    row: &crate::db::ProtocolRow,
) -> Result<
    (
        Option<ProtocolAgentResponse>,
        Option<ProtocolSchemaResponse>,
        Option<ProtocolTemplateResponse>,
    ),
    AppError,
> {
    let agent = if let Some(agent_id) = row.agent_id {
        state
            .repo()
            .get_persisted_agent(agent_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .map(|a| ProtocolAgentResponse {
                id: a.id,
                name: a.name,
                system_prompt: a.system_prompt,
                model_provider: a.model_provider,
                model_id: a.model_id,
            })
    } else {
        None
    };

    let output_schema = if let Some(schema_id) = row.output_schema_id {
        state
            .repos()
            .output_schemas
            .get_output_schema(schema_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .map(|s| ProtocolSchemaResponse {
                id: s.id,
                name: s.name,
                schema: s.schema,
            })
    } else {
        None
    };

    let prompt_template = if let Some(template_id) = row.prompt_template_id {
        state
            .repos()
            .prompt_templates
            .get_prompt_template(template_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .map(|t| ProtocolTemplateResponse {
                id: t.id,
                name: t.name,
                content: t.content,
            })
    } else {
        None
    };

    Ok((agent, output_schema, prompt_template))
}

/// Resolve agent names from agent IDs in the port rows.
async fn resolve_agent_names(
    state: &AppState,
    ports: &[crate::db::ProtocolPortRow],
) -> Result<HashMap<Uuid, String>, AppError> {
    let mut names = HashMap::new();
    for port in ports {
        if names.contains_key(&port.agent_id) {
            continue;
        }
        let agent = state
            .repo()
            .get_persisted_agent(port.agent_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .ok_or_else(|| {
                AppError::bad_request(format!(
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
async fn resolve_agent_schemas(
    state: &AppState,
    ports: &[crate::db::ProtocolPortRow],
) -> Result<HashMap<Uuid, serde_json::Value>, AppError> {
    let mut schemas = HashMap::new();
    for port in ports {
        if schemas.contains_key(&port.agent_id) {
            continue;
        }
        let agent = state
            .repo()
            .get_persisted_agent(port.agent_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        if let Some(agent) = agent {
            if let Some(schema_id) = agent.output_schema_id {
                let schema_row = state
                    .repos()
                    .output_schemas
                    .get_output_schema(schema_id)
                    .await
                    .map_err(|e| AppError::Internal(e.to_string()))?;
                if let Some(row) = schema_row {
                    schemas.insert(port.agent_id, row.schema);
                }
            }
        }
    }
    Ok(schemas)
}

/// Resolve agent tool names from agent IDs in the port rows.
async fn resolve_agent_tools(
    state: &AppState,
    ports: &[crate::db::ProtocolPortRow],
) -> Result<HashMap<Uuid, Vec<String>>, AppError> {
    let mut tools_map = HashMap::new();
    for port in ports {
        if tools_map.contains_key(&port.agent_id) {
            continue;
        }
        let tools = state
            .repo()
            .get_agent_tools(port.agent_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let tool_names: Vec<String> = tools.into_iter().map(|t| t.name).collect();
        tools_map.insert(port.agent_id, tool_names);
    }
    Ok(tools_map)
}
