//! OpenAPI specification and Swagger UI configuration

use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Nexor API",
        version = "0.1.0",
        description = "AI Agent Orchestration for GitHub Workflows.\n\nAll endpoints under `/api` except auth and health require a Bearer JWT token.",
        license(name = "MIT")
    ),
    servers(
        (url = "/api", description = "API base path")
    ),
    security(
        ("bearer_auth" = [])
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "Health", description = "Server health and status"),
        (name = "Auth", description = "Authentication and user management"),
        (name = "Tasks", description = "Task creation and management"),
        (name = "Agents", description = "AI agent templates"),
        (name = "Tools", description = "Tool definitions and agent-tool assignments"),
        (name = "Agent Context", description = "Document context assignments for agents"),
        (name = "Config", description = "Server configuration"),
        (name = "Chat", description = "Real-time chat with the orchestrator"),
        (name = "Sessions", description = "Chat session management"),
        (name = "Documents", description = "User documents for agent context"),
        (name = "Output Schemas", description = "Reusable structured output definitions"),
        (name = "Prompt Templates", description = "Reusable prompt text with variable refs"),
        (name = "Workflows", description = "Execution DAGs of agent steps"),
        (name = "Workflow Steps", description = "DAG nodes within workflows"),
        (name = "Workflow Edges", description = "DAG edges defining execution order"),
        (name = "Step Documents", description = "Document attachments per workflow step"),
        (name = "Agent Executions", description = "Individual agent execution records"),
        (name = "Costs", description = "Token usage and cost tracking"),
        (name = "Results", description = "Saved structured outputs"),
        (name = "Tool Routers", description = "Intelligent tool routing agents"),
        (name = "Session Context", description = "Context store and router requests for sessions"),
    ),
    paths(
        // Health & Auth
        super::api::health_check,
        super::api::auth::auth_setup,
        super::api::auth::auth_login,
        super::api::auth::auth_register,
        super::api::auth::auth_me,
        // Tasks
        super::api::tasks::list_tasks,
        super::api::tasks::get_task,
        super::api::tasks::create_task,
        // Agents
        super::api::agents::list_agents,
        super::api::agents::create_agent,
        super::api::agents::get_agent,
        super::api::agents::update_agent,
        super::api::agents::delete_agent,
        // Tools
        super::api::tools::list_tools,
        super::api::tools::create_tool,
        super::api::tools::get_tool,
        super::api::tools::update_tool,
        super::api::tools::delete_tool,
        super::api::tools::get_agent_tools,
        super::api::tools::set_agent_tools,
        // Agent Context
        super::api::agent_context::get_agent_context,
        super::api::agent_context::set_agent_context,
        // Config
        super::api::config::get_config,
        super::api::config::update_config,
        // Chat
        super::api::chat::send_chat,
        super::api::chat::get_chat_history,
        super::api::chat::chat_stream,
        super::api::chat::clear_chat_history,
        // Sessions
        super::api::sessions::list_modes,
        super::api::sessions::create_session,
        super::api::sessions::list_sessions,
        super::api::sessions::get_session,
        super::api::sessions::update_session,
        super::api::sessions::delete_session,
        super::api::sessions::send_session_chat,
        super::api::sessions::get_session_history,
        super::api::chat::session_chat_stream,
        // Documents
        super::api::documents::list_documents,
        super::api::documents::search_documents,
        super::api::documents::get_document,
        super::api::documents::create_document,
        super::api::documents::update_document,
        super::api::documents::delete_document,
        // Output Schemas
        super::api::output_schemas::list_output_schemas,
        super::api::output_schemas::create_output_schema,
        super::api::output_schemas::get_output_schema,
        super::api::output_schemas::update_output_schema,
        super::api::output_schemas::delete_output_schema,
        // Prompt Templates
        super::api::prompt_templates::list_prompt_templates,
        super::api::prompt_templates::create_prompt_template,
        super::api::prompt_templates::get_prompt_template,
        super::api::prompt_templates::update_prompt_template,
        super::api::prompt_templates::delete_prompt_template,
        // Workflows
        super::api::workflows::list_workflows,
        super::api::workflows::create_workflow,
        super::api::workflows::get_workflow,
        super::api::workflows::update_workflow,
        super::api::workflows::delete_workflow,
        // Workflow Steps
        super::api::workflows::create_workflow_step,
        super::api::workflows::list_workflow_steps,
        super::api::workflows::get_workflow_step,
        super::api::workflows::update_workflow_step,
        super::api::workflows::delete_workflow_step,
        // Workflow Edges
        super::api::workflows::list_workflow_edges,
        super::api::workflows::add_workflow_edge,
        super::api::workflows::remove_workflow_edge,
        // Step Documents
        super::api::workflows::list_step_documents,
        super::api::workflows::add_step_document,
        super::api::workflows::remove_step_document,
        // Agent Executions
        super::api::agent_executions::get_agent_execution,
        super::api::agent_executions::list_execution_messages,
        super::api::agent_executions::send_execution_message,
        super::api::agent_executions::approve_execution,
        // Costs
        super::api::costs::get_costs,
        // Results
        super::api::results::list_results,
        super::api::results::get_result,
        super::api::results::delete_result,
        // Tool Routers
        super::api::tool_routers::list_tool_routers,
        super::api::tool_routers::create_tool_router,
        super::api::tool_routers::get_tool_router,
        super::api::tool_routers::update_tool_router,
        super::api::tool_routers::delete_tool_router,
        super::api::tool_routers::get_router_tools,
        super::api::tool_routers::set_router_tools,
        // Session Context
        super::api::session_context::get_session_context,
        super::api::session_context::list_session_requests,
        // Context Response
        super::api::submit_context_response,
    ),
    components(schemas(
        // API response/request types
        super::api::HealthResponse,
        super::api::tasks::TasksQuery,
        super::api::tasks::CreateTaskRequest,
        super::api::agents::AgentResponse,
        super::api::agents::AgentsListResponse,
        super::api::agents::AgentPoolStats,
        super::api::agents::CreateAgentRequest,
        super::api::agents::UpdateAgentRequest,
        super::api::tools::ToolResponse,
        super::api::tools::CreateToolRequest,
        super::api::tools::UpdateToolRequest,
        super::api::tools::SetAgentToolsRequest,
        super::api::tools::AgentToolsResponse,
        super::api::agent_context::SetAgentContextRequest,
        super::api::agent_context::AgentContextResponse,
        super::api::config::ConfigResponse,
        super::api::config::UpdatePoolRequest,
        super::api::config::UpdateConfigRequest,
        super::api::chat::ChatRequest,
        super::api::chat::ChatResponse,
        super::api::chat::HistoryQuery,
        super::api::chat::ChatMessage,
        super::api::sessions::ModeInfo,
        super::api::sessions::CreateSessionRequest,
        super::api::sessions::UpdateSessionRequest,
        super::api::sessions::SessionResponse,
        super::api::auth::SetupRequest,
        super::api::auth::SetupResponse,
        super::api::auth::RegisterRequest,
        super::api::auth::AuthTokenResponse,
        super::api::auth::UserResponse,
        super::api::auth::LoginRequest,
        super::api::auth::LoginResponse,
        super::api::auth::MeResponse,
        super::api::documents::DocumentListItem,
        super::api::documents::DocumentResponse,
        super::api::documents::CreateDocumentRequest,
        super::api::documents::UpdateDocumentRequest,
        super::api::documents::DocumentSearchQuery,
        super::api::output_schemas::OutputSchemaResponse,
        super::api::output_schemas::CreateOutputSchemaRequest,
        super::api::output_schemas::UpdateOutputSchemaRequest,
        super::api::prompt_templates::PromptTemplateResponse,
        super::api::prompt_templates::CreatePromptTemplateRequest,
        super::api::prompt_templates::UpdatePromptTemplateRequest,
        super::api::agent_executions::AgentExecutionResponse,
        super::api::agent_executions::ExecutionMessageResponse,
        super::api::agent_executions::SendMessageRequest,
        super::api::agent_executions::ApproveExecutionRequest,
        super::api::costs::CostQuery,
        super::api::costs::CostResponse,
        super::api::results::ResultResponse,
        super::api::results::ResultQuery,
        super::api::workflows::WorkflowResponse,
        super::api::workflows::CreateWorkflowRequest,
        super::api::workflows::UpdateWorkflowRequest,
        super::api::workflows::WorkflowStepResponse,
        super::api::workflows::CreateStepRequest,
        super::api::workflows::UpdateStepRequest,
        super::api::workflows::EdgeRequest,
        super::api::workflows::EdgeResponse,
        super::api::workflows::StepDocumentRequest,
        super::api::workflows::StepDocumentResponse,
        super::api::ContextResponseRequest,
        super::api::FilePathContent,
        super::api::tool_routers::CreateToolRouterRequest,
        super::api::tool_routers::UpdateToolRouterRequest,
        super::api::tool_routers::SetRouterToolsRequest,
        // DB types used directly in responses
        crate::db::DocumentSearchResult,
        crate::db::ToolRouterRow,
        crate::db::ContextStoreRow,
        crate::db::RouterRequestRow,
        crate::db::traits::ModelSpendRow,
        // Domain types
        crate::types::Task,
        crate::types::TaskId,
        crate::types::TaskStatus,
        crate::types::Priority,
        crate::types::SliceId,
        crate::types::AgentId,
        crate::types::ModelConfig,
        crate::types::LLMProvider,
        crate::types::AgentPoolConfig,
    ))
)]
pub struct ApiDoc;

/// Adds the Bearer auth security scheme to the OpenAPI spec.
struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                utoipa::openapi::security::SecurityScheme::Http(utoipa::openapi::security::Http::new(utoipa::openapi::security::HttpAuthScheme::Bearer)),
            );
        }
    }
}
