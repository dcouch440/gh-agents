//! REST API endpoint handlers

pub mod agent_context;
pub mod agent_executions;
pub mod agents;
pub mod archetypes;
pub mod auth;
pub mod board;
pub mod cancellation;
pub mod chat;
pub mod collections;
pub mod config;
pub mod costs;
pub mod documents;
pub mod error;
pub mod health;
pub mod output_schemas;
pub mod ownership;
pub mod prompt_templates;
pub mod results;
pub mod rooms;
pub mod sessions;
pub mod tools;
pub mod workflows;

pub use error::AppError;

// Re-export board handlers and types
pub use board::{submit_board, BoardSubmitRequest, BoardSubmitResponse};

// Re-export auth handlers and types
pub use auth::{
    auth_login, auth_me, auth_register, auth_setup, AuthTokenResponse, LoginRequest, LoginResponse,
    MeResponse, RegisterRequest, SetupRequest, SetupResponse, UserResponse,
};

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
    clear_session_messages, create_session, delete_session, get_session, get_session_history,
    list_modes, list_sessions, send_session_chat, update_session, CreateSessionRequest, ModeInfo,
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
    list_execution_messages, set_exemplary, AgentExecutionResponse, ApproveExecutionRequest,
    ExecutionMessageResponse, ListExecutionsQuery, SetExemplaryRequest,
};

// Re-export cost handlers and types
pub use costs::{get_costs, CostQuery, CostResponse};

// Re-export results handlers and types
pub use results::{delete_result, get_result, list_results, ResultQuery, ResultResponse};

// Re-export workflow handlers and types
pub use workflows::{
    add_step_document, add_workflow_edge, clear_step_messages, create_template, create_workflow,
    create_workflow_step, delete_template, delete_workflow, delete_workflow_edge_by_id,
    delete_workflow_step, execute_workshop_step, get_or_create_step_session,
    get_or_create_workshop, get_run_detail, get_step_chat_debug, get_step_config,
    get_step_dispatch_history, get_step_last_run, get_step_run_for_execution, get_step_session,
    get_step_sub_dag, get_template, get_workflow, get_workflow_plans, get_workflow_step,
    get_workshop, list_question_states, list_step_documents, list_templates, list_workflow_edges,
    list_workflow_executions, list_workflow_steps, list_workflows, rebase_workshop,
    remove_step_document, remove_workflow_edge, run_workflow, toggle_step_pin, update_workflow,
    update_workflow_step, CreateStepRequest, CreateTemplateRequest, CreateWorkflowRequest,
    EdgeRequest, EdgeResponse, RebaseResponse, RunDetailResponse, RunStepResultResponse,
    RunTemplateDetailResponse, RunTemplateResponse, RunWorkflowRequest, StepDocumentRequest,
    StepDocumentResponse, UpdateStepRequest, UpdateWorkflowRequest, WorkflowExecutionResponse,
    WorkflowResponse, WorkflowRunResponse, WorkflowStepResponse, WorkshopResponse,
    WorkshopStatusResponse, WorkshopStepResponse,
};

// Re-export archetype handlers
pub use archetypes::list_archetypes;

// Re-export collection handlers and types
pub use collections::{
    create_collection, delete_collection, get_collection, get_collection_run_status,
    list_collections, run_collection, update_collection, CollectionResponse, CollectionRunResponse,
    CreateCollectionRequest, UpdateCollectionRequest,
};

// Re-export room handlers and types
pub use rooms::{
    add_room_member, close_room_session, create_room, create_room_session, delete_room, get_room,
    get_room_session, get_room_transcript, list_room_members, list_room_outputs,
    remove_room_member, send_room_message, set_room_members, update_room, AddRoomMemberRequest,
    CreateRoomRequest, RoomMessageRequest, RoomOutputResponse, SetRoomMembersRequest,
    UpdateRoomRequest,
};

// Re-export step port handlers and types
pub mod step_ports;
pub use step_ports::{
    create_step_input, create_step_output, delete_step_input, delete_step_output, list_step_inputs,
    list_step_outputs, CreateStepInputRequest, CreateStepOutputRequest, StepInputResponse,
    StepOutputResponse,
};

// Re-export agent roster handlers and types
pub mod agent_roster;
pub use agent_roster::{
    create_roster_agent, delete_roster_agent, list_roster_agents, CreateRosterAgentRequest,
    RosterAgentResponse,
};

// Re-export room step member handlers and types
pub mod room_step_members;
pub use room_step_members::list_room_step_members;

// Re-export routing rule handlers and types
pub mod routing_rules;
pub use routing_rules::{
    create_routing_rule, delete_routing_rule, list_routing_rules, update_routing_rule,
    CreateRoutingRuleRequest, RoutingRuleResponse, UpdateRoutingRuleRequest,
};

// Re-export system config handlers and types
pub mod system_config;
pub use system_config::{
    delete_system_config, list_system_configs, upsert_system_config, CreateSystemConfigRequest,
    SystemConfigQuery, SystemConfigResponse,
};

// Re-export health handler and type
pub use health::{health_check, HealthResponse};

// Re-export cancellation handlers
pub use cancellation::{cancel_agent_execution, cancel_chat_message};

// Re-export dispatch handlers and types
pub mod dispatch;
pub use dispatch::{
    dispatch_cancel, dispatch_send, get_dispatch_session, get_dispatch_trace, list_dispatch_tasks,
};

// Re-export protocol handlers and types
pub mod protocols;
pub use protocols::executions::list_protocol_executions;
pub use protocols::{
    apply_protocol, create_port, create_protocol, delete_port, delete_protocol, get_protocol,
    list_protocol_types, list_protocols, preview_expansion, unapply_protocol, update_port,
    update_protocol,
};
