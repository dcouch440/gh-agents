//! REST API endpoint handlers

pub mod agent_context;
pub mod agent_executions;
pub mod agents;
pub mod auth;
pub mod cancellation;
pub mod chat;
pub mod collections;
pub mod config;
pub mod costs;
pub mod documents;
pub mod health;
pub mod output_schemas;
pub mod prompt_templates;
pub mod results;
pub mod rooms;
pub mod router_modes;
pub mod session_context;
pub mod sessions;
pub mod tasks;
pub mod tool_routers;
pub mod tools;
pub mod workflows;

// Re-export auth handlers and types
pub use auth::{
    auth_login, auth_me, auth_register, auth_setup, AuthTokenResponse, LoginRequest, LoginResponse,
    MeResponse, RegisterRequest, SetupRequest, SetupResponse, UserResponse,
};

// Re-export task handlers and types
pub use tasks::{create_task, get_task, list_tasks, CreateTaskRequest, TasksQuery};

// Re-export agent handlers and types
pub use agents::{
    create_agent, delete_agent, get_agent, list_agents, update_agent, AgentPoolStats,
    AgentResponse, AgentsListResponse, CreateAgentRequest, UpdateAgentRequest,
};

// Re-export tool handlers and types
pub use tools::{
    create_tool, delete_tool, get_agent_tools, get_tool, list_tools, set_agent_tools, update_tool,
    AgentToolsResponse, CreateToolRequest, SetAgentToolsRequest, ToolResponse, UpdateToolRequest,
};

// Re-export config handlers and types
pub use config::{
    get_config, update_config, ConfigResponse, UpdateConfigRequest, UpdatePoolRequest,
};

// Re-export agent context handlers and types
pub use agent_context::{
    get_agent_context, set_agent_context, AgentContextResponse, SetAgentContextRequest,
};

// Re-export chat handlers and types
pub use chat::{
    chat_stream, clear_chat_history, get_chat_history, send_chat, session_chat_stream, ChatMessage,
    ChatRequest, ChatResponse, HistoryQuery,
};

// Re-export document handlers and types
pub use documents::{
    create_document, delete_document, get_document, list_documents, search_documents,
    update_document, CreateDocumentRequest, DocumentListItem, DocumentResponse,
    DocumentSearchQuery, UpdateDocumentRequest,
};

// Re-export session handlers and types
pub use sessions::{
    create_agent_mode, create_session, delete_agent_mode, delete_session, get_session,
    get_session_history, list_agent_modes, list_modes, list_sessions, send_session_chat,
    update_session, AgentModeResponse, CreateAgentModeRequest, CreateSessionRequest, ModeInfo,
    SessionResponse, UpdateSessionRequest,
};

// Re-export output schema handlers and types
pub use output_schemas::{
    create_output_schema, delete_output_schema, get_output_schema, list_output_schemas,
    update_output_schema, CreateOutputSchemaRequest, OutputSchemaResponse,
    UpdateOutputSchemaRequest,
};

// Re-export prompt template handlers and types
pub use prompt_templates::{
    create_prompt_template, delete_prompt_template, get_prompt_template, list_prompt_templates,
    update_prompt_template, CreatePromptTemplateRequest, PromptTemplateResponse,
    UpdatePromptTemplateRequest,
};

// Re-export agent execution handlers and types
pub use agent_executions::{
    approve_execution, execution_message_stream, get_agent_execution, list_agent_executions,
    list_execution_messages, send_execution_message, AgentExecutionResponse,
    ApproveExecutionRequest, ExecutionMessageResponse, ListExecutionsQuery, SendMessageRequest,
    SendMessageResponse,
};

// Re-export cost handlers and types
pub use costs::{get_costs, CostQuery, CostResponse};

// Re-export results handlers and types
pub use results::{delete_result, get_result, list_results, ResultQuery, ResultResponse};

// Re-export workflow handlers and types
pub use workflows::{
    add_step_document, add_workflow_edge, create_workflow, create_workflow_step, delete_workflow,
    delete_workflow_step, get_workflow, get_workflow_step, list_step_documents,
    list_workflow_edges, list_workflow_steps, list_workflows, remove_step_document,
    remove_workflow_edge, update_workflow, update_workflow_step, CreateStepRequest,
    CreateWorkflowRequest, EdgeRequest, EdgeResponse, StepDocumentRequest, StepDocumentResponse,
    UpdateStepRequest, UpdateWorkflowRequest, WorkflowResponse, WorkflowStepResponse,
};

// Re-export collection handlers and types
pub use collections::{
    create_collection, delete_collection, get_collection, get_collection_run_status,
    get_collection_variables, list_collections, run_collection, update_collection,
    CollectionResponse, CollectionRunResponse, CreateCollectionRequest, ExecutionVariableResponse,
    UpdateCollectionRequest,
};

// Re-export tool router handlers and types
pub use tool_routers::{
    create_tool_router, delete_tool_router, get_router_tools, get_tool_router, list_tool_routers,
    set_router_tools, update_tool_router, CreateToolRouterRequest, SetRouterToolsRequest,
    UpdateToolRouterRequest,
};

// Re-export router mode handlers and types
pub use router_modes::{
    create_router_mode, delete_router_mode, get_mode_tools, get_router_mode, list_router_modes,
    set_mode_tools, update_router_mode, CreateRouterModeRequest, RouterModeResponse,
    SetModeToolsRequest, UpdateRouterModeRequest,
};

// Re-export session context handlers
pub use session_context::{get_session_context, list_session_requests};

// Re-export room handlers and types
pub use rooms::{
    add_room_member, close_room_session, create_room, create_room_session, delete_room, get_room,
    get_room_session, get_room_transcript, list_room_members, remove_room_member,
    send_room_message, set_room_members, update_room, AddRoomMemberRequest, CreateRoomRequest,
    RoomMessageRequest, SetRoomMembersRequest, UpdateRoomRequest,
};

// Re-export health handler and type
pub use health::{health_check, HealthResponse};

// Re-export cancellation handler
pub use cancellation::cancel_agent_execution;

use crate::constants::{MAX_DESCRIPTION_LENGTH, MAX_PROMPT_LENGTH, MAX_TITLE_LENGTH};
