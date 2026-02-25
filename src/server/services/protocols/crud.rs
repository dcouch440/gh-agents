//! CRUD operations for protocols and ports.

use uuid::Uuid;

use crate::db::traits::{CreateProtocolInput, ProtocolRepo, UpdateProtocolInput};
use crate::db::{ProtocolPortRow, ProtocolRow};
use crate::server::hub::protocols::ProtocolEngine;
use crate::server::services::error::ServiceError;

use super::{validate_port_name, CreateProtocolServiceInput};

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
