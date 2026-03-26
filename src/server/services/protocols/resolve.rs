//! Resolution helpers for protocol associations, agent names, schemas, and tools.

use std::collections::{HashMap, HashSet};

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

/// Collect unique agent IDs from port rows.
fn unique_agent_ids(ports: &[ProtocolPortRow]) -> Vec<Uuid> {
    ports
        .iter()
        .map(|p| p.agent_id)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect()
}

/// Resolve agent names from agent IDs found in the port rows.
///
/// Returns a validation error if any referenced agent is missing from the DB.
pub(crate) async fn resolve_agent_names(
    agent_repo: &dyn AgentRepo,
    ports: &[ProtocolPortRow],
) -> Result<HashMap<Uuid, String>, ServiceError> {
    let agents = agent_repo
        .get_agents_by_ids(&unique_agent_ids(ports))
        .await?;
    let names: HashMap<Uuid, String> = agents.into_iter().map(|a| (a.id, a.name)).collect();

    // Validate all referenced agents were found (original contract)
    for port in ports {
        if !names.contains_key(&port.agent_id) {
            return Err(ServiceError::validation(format!(
                "Agent {} not found for port {}",
                port.agent_id, port.port_name
            )));
        }
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
    let agents = agent_repo
        .get_agents_by_ids(&unique_agent_ids(ports))
        .await?;

    let mut schemas = HashMap::new();
    for agent in agents {
        if let Some(schema_id) = agent.output_schema_id {
            if let Some(row) = schema_repo.get_output_schema(schema_id).await? {
                schemas.insert(agent.id, row.schema);
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
    let agent_ids = unique_agent_ids(ports);
    let all_tools = tool_repo.get_tools_for_agents(&agent_ids).await?;

    let mut tools_map: HashMap<Uuid, Vec<String>> = HashMap::new();
    for (agent_id, tool) in all_tools {
        tools_map.entry(agent_id).or_default().push(tool.name);
    }
    Ok(tools_map)
}
