//! Resolution helpers for protocol associations, agent names, schemas, and tools.

use std::collections::HashMap;

use uuid::Uuid;

use crate::db::traits::{AgentRepo, OutputSchemaRepo, PromptTemplateRepo, ToolRepo};
use crate::db::{AgentRow, OutputSchemaRow, PromptTemplateRow, ProtocolPortRow, ProtocolRow};
use crate::server::services::error::ServiceError;

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
