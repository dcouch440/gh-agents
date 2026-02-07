//! Repository container for all database access traits.
//!
//! Groups all repository trait objects into a single struct, eliminating
//! the need for optional fields and runtime "repo not available" checks.

use std::sync::Arc;

use crate::db::traits::{
    AgentExecutionRepo, ContextStoreRepo, DocumentRepo, OutputSchemaRepo, PromptTemplateRepo,
    ProtocolRepo, ResultRepo, RoomRepo, RouterRequestRepo, SystemConfigRepo, TokenLedgerRepo,
    ToolCapabilityRepo, ToolRouterRepo, UserRepo, WorkflowRepo,
};

/// All repository trait objects grouped together.
///
/// All fields are required - tests must provide mock implementations.
/// This eliminates runtime "repo not available" errors with compile-time guarantees.
#[derive(Clone)]
pub struct Repos {
    /// User authentication operations
    pub users: Arc<dyn UserRepo>,
    /// Document CRUD operations
    pub documents: Arc<dyn DocumentRepo>,
    /// Output schema management
    pub output_schemas: Arc<dyn OutputSchemaRepo>,
    /// Prompt template management
    pub prompt_templates: Arc<dyn PromptTemplateRepo>,
    /// Workflow management
    pub workflows: Arc<dyn WorkflowRepo>,
    /// Agent execution tracking
    pub agent_executions: Arc<dyn AgentExecutionRepo>,
    /// Token usage tracking
    pub token_ledger: Arc<dyn TokenLedgerRepo>,
    /// Result storage
    pub results: Arc<dyn ResultRepo>,
    /// Tool routing management
    pub tool_routers: Arc<dyn ToolRouterRepo>,
    /// Context storage
    pub context_store: Arc<dyn ContextStoreRepo>,
    /// Router request lifecycle
    pub router_requests: Arc<dyn RouterRequestRepo>,
    /// Agent room management
    pub rooms: Arc<dyn RoomRepo>,
    /// Tool capability taxonomy and assignments
    pub tool_capabilities: Arc<dyn ToolCapabilityRepo>,
    /// System-wide configuration
    pub system_config: Arc<dyn SystemConfigRepo>,
    /// Protocol management (execution recipes)
    pub protocols: Arc<dyn ProtocolRepo>,
}

impl Repos {
    /// Create a new Repos instance with all repositories.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        users: Arc<dyn UserRepo>,
        documents: Arc<dyn DocumentRepo>,
        output_schemas: Arc<dyn OutputSchemaRepo>,
        prompt_templates: Arc<dyn PromptTemplateRepo>,
        workflows: Arc<dyn WorkflowRepo>,
        agent_executions: Arc<dyn AgentExecutionRepo>,
        token_ledger: Arc<dyn TokenLedgerRepo>,
        results: Arc<dyn ResultRepo>,
        tool_routers: Arc<dyn ToolRouterRepo>,
        context_store: Arc<dyn ContextStoreRepo>,
        router_requests: Arc<dyn RouterRequestRepo>,
        rooms: Arc<dyn RoomRepo>,
        tool_capabilities: Arc<dyn ToolCapabilityRepo>,
        system_config: Arc<dyn SystemConfigRepo>,
        protocols: Arc<dyn ProtocolRepo>,
    ) -> Self {
        Self {
            users,
            documents,
            output_schemas,
            prompt_templates,
            workflows,
            agent_executions,
            token_ledger,
            results,
            tool_routers,
            context_store,
            router_requests,
            rooms,
            tool_capabilities,
            system_config,
            protocols,
        }
    }
}
