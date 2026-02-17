//! Protocol management endpoints — CRUD, port management, preview, and apply.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::AppError;
use crate::db::traits::UpdateProtocolInput;
use crate::server::auth as auth_utils;
use crate::server::hub::protocols::types::ProtocolExpansion;
use crate::server::services::protocols as protocol_svc;
use crate::server::state::AppState;

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
// Mapping helpers (domain types → response types)
// ============================================================================

fn map_agent_response(agent: crate::db::AgentRow) -> ProtocolAgentResponse {
    ProtocolAgentResponse {
        id: agent.id,
        name: agent.name,
        system_prompt: agent.system_prompt,
        model_provider: agent.model_provider,
        model_id: agent.model_id,
    }
}

fn map_schema_response(schema: crate::db::OutputSchemaRow) -> ProtocolSchemaResponse {
    ProtocolSchemaResponse {
        id: schema.id,
        name: schema.name,
        schema: schema.schema,
    }
}

fn map_template_response(template: crate::db::PromptTemplateRow) -> ProtocolTemplateResponse {
    ProtocolTemplateResponse {
        id: template.id,
        name: template.name,
        content: template.content,
    }
}

fn map_port_response(port: crate::db::ProtocolPortRow) -> ProtocolPortResponse {
    ProtocolPortResponse {
        id: port.id,
        port_name: port.port_name,
        description: port.description,
        agent_id: port.agent_id,
        display_order: port.display_order,
    }
}

/// Resolve protocol associations via the service and map to response types.
async fn resolve_and_map_associations(
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
    let (agent, schema, template) = protocol_svc::resolve_protocol_associations(
        state.repo().as_ref(),
        state.repos().output_schemas.as_ref(),
        state.repos().prompt_templates.as_ref(),
        row,
    )
    .await?;

    Ok((
        agent.map(map_agent_response),
        schema.map(map_schema_response),
        template.map(map_template_response),
    ))
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
    let proto_repo = state.repos().protocols.as_ref();
    let rows = protocol_svc::list_protocols(proto_repo).await?;

    let mut responses = Vec::with_capacity(rows.len());
    for row in rows {
        let ports = protocol_svc::list_protocol_ports(proto_repo, row.id).await?;
        let (agent, output_schema, prompt_template) =
            resolve_and_map_associations(&state, &row).await?;
        responses.push(ProtocolResponse {
            id: row.id,
            name: row.name,
            description: row.description,
            protocol_type: row.protocol_type,
            config: row.config,
            version: row.version,
            ports: ports.into_iter().map(map_port_response).collect(),
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
    let row = protocol_svc::create_protocol(
        state.repos().protocols.as_ref(),
        state.protocol_engine(),
        protocol_svc::CreateProtocolServiceInput {
            name: request.name,
            description: request.description,
            protocol_type: request.protocol_type,
            config: request.config,
            agent_id: request.agent_id,
            output_schema_id: request.output_schema_id,
            prompt_template_id: request.prompt_template_id,
        },
    )
    .await?;

    let (agent, output_schema, prompt_template) =
        resolve_and_map_associations(&state, &row).await?;

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
    let proto_repo = state.repos().protocols.as_ref();
    let (row, ports) = protocol_svc::get_protocol(proto_repo, id).await?;

    let (agent, output_schema, prompt_template) =
        resolve_and_map_associations(&state, &row).await?;

    Ok(Json(ProtocolResponse {
        id: row.id,
        name: row.name,
        description: row.description,
        protocol_type: row.protocol_type,
        config: row.config,
        version: row.version,
        ports: ports.into_iter().map(map_port_response).collect(),
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
    let proto_repo = state.repos().protocols.as_ref();

    let row = protocol_svc::update_protocol(
        proto_repo,
        id,
        UpdateProtocolInput {
            id,
            name: request.name,
            description: request.description,
            config: request.config,
            agent_id: request.agent_id,
            output_schema_id: request.output_schema_id,
            prompt_template_id: request.prompt_template_id,
        },
    )
    .await?;

    let ports = protocol_svc::list_protocol_ports(proto_repo, id).await?;

    let (agent, output_schema, prompt_template) =
        resolve_and_map_associations(&state, &row).await?;

    Ok(Json(ProtocolResponse {
        id: row.id,
        name: row.name,
        description: row.description,
        protocol_type: row.protocol_type,
        config: row.config,
        version: row.version,
        ports: ports.into_iter().map(map_port_response).collect(),
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
    protocol_svc::delete_protocol(state.repos().protocols.as_ref(), id).await?;
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
    let port = protocol_svc::create_port(
        state.repos().protocols.as_ref(),
        protocol_id,
        request.port_name,
        request.description,
        request.agent_id,
        request.display_order,
    )
    .await?;

    Ok((StatusCode::CREATED, Json(map_port_response(port))))
}

/// PUT /api/protocols/:protocol_id/ports/:port_id — Update a port.
pub async fn update_port(
    State(state): State<AppState>,
    Path((_, port_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<UpdatePortRequest>,
) -> Result<Json<ProtocolPortResponse>, AppError> {
    let port = protocol_svc::update_port(
        state.repos().protocols.as_ref(),
        port_id,
        request.port_name,
        request.description,
        request.agent_id,
        request.display_order,
    )
    .await?;

    Ok(Json(map_port_response(port)))
}

/// DELETE /api/protocols/:protocol_id/ports/:port_id — Delete a port.
pub async fn delete_port(
    State(state): State<AppState>,
    Path((_, port_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    protocol_svc::delete_port(state.repos().protocols.as_ref(), port_id).await?;
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
    let expansion = protocol_svc::preview_expansion(
        state.repos().protocols.as_ref(),
        state.repo().as_ref(),
        state.repos().output_schemas.as_ref(),
        state.protocol_engine(),
        protocol_id,
    )
    .await?;

    Ok(Json(PreviewResponse { expansion }))
}

/// POST /api/protocols/:id/apply/:step_id — Apply protocol to a workflow step.
pub async fn apply_protocol(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path((protocol_id, step_id)): Path<(Uuid, Uuid)>,
    Json(_request): Json<ApplyProtocolRequest>,
) -> Result<(StatusCode, Json<ApplyResponse>), AppError> {
    let result = protocol_svc::apply_protocol(
        state.repos().protocols.as_ref(),
        state.repos().workflows.as_ref(),
        state.repos().output_schemas.as_ref(),
        state.repo().as_ref(),
        state.protocol_engine(),
        auth.user_id.0,
        protocol_id,
        step_id,
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(ApplyResponse {
            output_schema_id: result.output_schema_id,
            created_steps: result
                .created_steps
                .into_iter()
                .map(|s| CreatedStepResponse {
                    port_name: s.port_name,
                    step_id: s.step_id,
                    agent_id: s.agent_id,
                })
                .collect(),
        }),
    ))
}

/// DELETE /api/protocols/:protocol_id/unapply/:step_id — Remove protocol from step.
pub async fn unapply_protocol(
    State(state): State<AppState>,
    Path((_, step_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    let repo = state.repos().protocols.as_ref();
    repo.delete_step_protocol(step_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}
