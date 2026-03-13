//! Repository container for all database access traits.
//!
//! Groups all repository trait objects into a single struct, eliminating
//! the need for optional fields and runtime "repo not available" checks.

use std::sync::Arc;

use crate::db::traits::{
    AgentExecutionRepo, AgentRepo, AuthConfigRepo, ChatMessageRepo, ContentVersionRepo,
    DocumentRepo, OutputSchemaRepo, PromptTemplateRepo, ProtocolRepo, ResultRepo, RoomRepo,
    SessionRepo, SystemConfigRepo, SystemFileRepo, TokenLedgerRepo, ToolCapabilityRepo, ToolRepo,
    UserRepo, WorkflowRepo,
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
    /// Agent room management
    pub rooms: Arc<dyn RoomRepo>,
    /// Tool capability taxonomy and assignments
    pub tool_capabilities: Arc<dyn ToolCapabilityRepo>,
    /// System-wide configuration
    pub system_config: Arc<dyn SystemConfigRepo>,
    /// Protocol management (execution recipes)
    pub protocols: Arc<dyn ProtocolRepo>,
    /// Content versioning operations
    pub content_versions: Arc<dyn ContentVersionRepo>,
    /// Agent persistence, context, and guidance
    pub agents: Arc<dyn AgentRepo>,
    /// Tool persistence and agent-tool linkage
    pub tools: Arc<dyn ToolRepo>,
    /// Chat session management
    pub sessions: Arc<dyn SessionRepo>,
    /// Global chat messages
    pub chat_messages: Arc<dyn ChatMessageRepo>,
    /// Authentication configuration and health checks
    pub auth_config: Arc<dyn AuthConfigRepo>,
    /// System store file metadata
    pub system_files: Arc<dyn SystemFileRepo>,
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
        rooms: Arc<dyn RoomRepo>,
        tool_capabilities: Arc<dyn ToolCapabilityRepo>,
        system_config: Arc<dyn SystemConfigRepo>,
        protocols: Arc<dyn ProtocolRepo>,
        content_versions: Arc<dyn ContentVersionRepo>,
        agents: Arc<dyn AgentRepo>,
        tools: Arc<dyn ToolRepo>,
        sessions: Arc<dyn SessionRepo>,
        chat_messages: Arc<dyn ChatMessageRepo>,
        auth_config: Arc<dyn AuthConfigRepo>,
        system_files: Arc<dyn SystemFileRepo>,
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
            rooms,
            tool_capabilities,
            system_config,
            protocols,
            content_versions,
            agents,
            tools,
            sessions,
            chat_messages,
            auth_config,
            system_files,
        }
    }
}
