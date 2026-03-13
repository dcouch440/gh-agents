use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

use crate::db::{
    ProtocolDocumentDefRow, ProtocolExecutionRow, ProtocolPortRow, ProtocolRow,
    WorkflowStepProtocolRow,
};

// ============================================================================
// Protocol Repository
// ============================================================================

/// Input for creating a new protocol.
#[derive(Debug, Clone)]
pub struct CreateProtocolInput {
    pub name: String,
    pub description: String,
    pub protocol_type: String,
    pub config: serde_json::Value,
    pub agent_id: Option<Uuid>,
    pub output_schema_id: Option<Uuid>,
    pub prompt_template_id: Option<Uuid>,
}

/// Input for updating a protocol.
#[derive(Debug, Clone)]
pub struct UpdateProtocolInput {
    pub id: Uuid,
    pub name: Option<String>,
    pub description: Option<String>,
    pub config: Option<serde_json::Value>,
    pub agent_id: Option<Uuid>,
    pub output_schema_id: Option<Uuid>,
    pub prompt_template_id: Option<Uuid>,
}

/// Input for updating a protocol execution's status.
#[derive(Debug, Clone)]
pub struct UpdateProtocolExecutionStatusInput {
    pub id: Uuid,
    pub status: String,
    pub output_content: Option<String>,
    pub error_message: Option<String>,
    pub tokens_in: Option<i32>,
    pub tokens_out: Option<i32>,
    pub cost_usd: Option<f64>,
    pub model: Option<String>,
}

/// Database operations for protocol management (reusable execution recipes).
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ProtocolRepo: Send + Sync {
    // --- Protocols ---

    /// Create a new protocol.
    async fn create_protocol(&self, input: CreateProtocolInput) -> Result<ProtocolRow>;

    /// Get a protocol by ID.
    async fn get_protocol(&self, id: Uuid) -> Result<Option<ProtocolRow>>;

    /// Get a protocol by its protocol_type (e.g., "workforce").
    async fn get_protocol_by_type(&self, protocol_type: &str) -> Result<Option<ProtocolRow>>;

    /// List all protocols.
    async fn list_protocols(&self) -> Result<Vec<ProtocolRow>>;

    /// Update a protocol.
    async fn update_protocol(&self, input: UpdateProtocolInput) -> Result<ProtocolRow>;

    /// Delete a protocol by ID.
    async fn delete_protocol(&self, id: Uuid) -> Result<()>;

    // --- Protocol Ports ---

    /// List all ports for a protocol, ordered by display_order.
    async fn list_protocol_ports(&self, protocol_id: Uuid) -> Result<Vec<ProtocolPortRow>>;

    /// Add a port to a protocol.
    async fn create_protocol_port(
        &self,
        protocol_id: Uuid,
        port_name: String,
        description: String,
        agent_id: Uuid,
        display_order: i32,
    ) -> Result<ProtocolPortRow>;

    /// Update a protocol port.
    async fn update_protocol_port(
        &self,
        id: Uuid,
        port_name: Option<String>,
        description: Option<String>,
        agent_id: Option<Uuid>,
        display_order: Option<i32>,
    ) -> Result<ProtocolPortRow>;

    /// Delete a protocol port.
    async fn delete_protocol_port(&self, id: Uuid) -> Result<()>;

    // --- Workflow Step Protocol Linkage ---

    /// Get the protocol linkage for a workflow step.
    async fn get_step_protocol(
        &self,
        workflow_step_id: Uuid,
    ) -> Result<Option<WorkflowStepProtocolRow>>;

    /// Link a protocol to a workflow step (stores expansion snapshot).
    async fn create_step_protocol(
        &self,
        workflow_step_id: Uuid,
        protocol_id: Uuid,
        applied_expansion: serde_json::Value,
    ) -> Result<WorkflowStepProtocolRow>;

    /// Remove a protocol linkage from a workflow step.
    async fn delete_step_protocol(&self, workflow_step_id: Uuid) -> Result<()>;

    // --- Protocol-scoped Document Definitions ---

    /// List document definitions scoped to a protocol (template defs).
    async fn list_protocol_document_defs(
        &self,
        protocol_id: Uuid,
    ) -> Result<Vec<ProtocolDocumentDefRow>>;

    /// Create a protocol-scoped document definition.
    async fn create_protocol_document_def(
        &self,
        def: ProtocolDocumentDefRow,
    ) -> Result<ProtocolDocumentDefRow>;

    /// Update a protocol-scoped document definition.
    async fn update_protocol_document_def(
        &self,
        id: Uuid,
        name: String,
        description: String,
        target_length: i32,
    ) -> Result<ProtocolDocumentDefRow>;

    /// Delete a protocol-scoped document definition.
    async fn delete_protocol_document_def(&self, id: Uuid) -> Result<()>;

    // --- Protocol Executions ---

    /// Create a new protocol execution record.
    async fn create_protocol_execution(
        &self,
        row: ProtocolExecutionRow,
    ) -> Result<ProtocolExecutionRow>;

    /// Update a protocol execution's status and output fields.
    async fn update_protocol_execution_status(
        &self,
        input: UpdateProtocolExecutionStatusInput,
    ) -> Result<ProtocolExecutionRow>;

    /// List all protocol executions for a given step.
    async fn list_protocol_executions_by_step(
        &self,
        step_id: Uuid,
    ) -> Result<Vec<ProtocolExecutionRow>>;

    /// List all protocol executions for a given workflow run.
    async fn list_protocol_executions_by_run(
        &self,
        run_id: Uuid,
    ) -> Result<Vec<ProtocolExecutionRow>>;
}
