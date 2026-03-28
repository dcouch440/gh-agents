use axum::{
    middleware,
    routing::{delete, get, post, put},
    Router,
};

use super::api;
use super::AppState;
use crate::constants::routes;

/// Build the public route group (no auth required).
pub(super) fn build_public_routes() -> Router<AppState> {
    Router::new()
        .route(routes::HEALTH, get(api::health_check))
        .route(routes::AUTH_SETUP, post(api::auth_setup))
        .route(routes::AUTH_LOGIN, post(api::auth_login))
        .route(routes::AUTH_REGISTER, post(api::auth_register))
}

/// Build the protected route group (auth required).
pub(super) fn build_protected_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .merge(auth_routes())
        .merge(agent_routes())
        .merge(tool_routes())
        .merge(chat_routes())
        .merge(session_routes())
        .merge(document_routes())
        .merge(schema_routes())
        .merge(workflow_routes())
        .merge(collection_routes())
        .merge(execution_routes())
        .merge(room_routes())
        .merge(step_config_routes())
        .merge(protocol_routes())
        .merge(system_routes())
        .merge(dispatch_routes())
        .layer(middleware::from_fn_with_state(
            state.clone(),
            super::require_auth,
        ))
}

fn auth_routes() -> Router<AppState> {
    Router::new().route(routes::AUTH_ME, get(api::auth_me))
}

fn agent_routes() -> Router<AppState> {
    Router::new()
        .route(
            routes::AGENTS,
            get(api::list_agents).post(api::create_agent),
        )
        .route(
            routes::AGENT,
            get(api::get_agent)
                .patch(api::update_agent)
                .delete(api::delete_agent),
        )
        .route(
            routes::AGENT_TOOLS,
            get(api::get_agent_tools).put(api::set_agent_tools),
        )
        .route(
            routes::AGENT_CONTEXT,
            get(api::get_agent_context).put(api::set_agent_context),
        )
}

fn tool_routes() -> Router<AppState> {
    Router::new()
        .route(routes::TOOLS, get(api::list_tools).post(api::create_tool))
        .route(
            routes::TOOL,
            get(api::get_tool)
                .patch(api::update_tool)
                .delete(api::delete_tool),
        )
}

fn chat_routes() -> Router<AppState> {
    Router::new()
        .route(
            routes::CONFIG,
            get(api::get_config).patch(api::update_config),
        )
        .route(routes::CHAT, post(api::send_chat))
        .route(
            routes::CHAT_HISTORY,
            get(api::get_chat_history).delete(api::clear_chat_history),
        )
        .route(routes::CHAT_STREAM, get(api::chat_stream))
        .route(routes::MODES, get(api::list_modes))
}

fn session_routes() -> Router<AppState> {
    Router::new()
        .route(
            routes::SESSIONS,
            get(api::list_sessions).post(api::create_session),
        )
        .route(
            routes::SESSION,
            get(api::get_session)
                .patch(api::update_session)
                .delete(api::delete_session),
        )
        .route(routes::SESSION_CHAT, post(api::send_session_chat))
        .route(routes::SESSION_HISTORY, get(api::get_session_history))
        .route(routes::SESSION_CHAT_STREAM, get(api::session_chat_stream))
        .route(routes::SESSION_CHAT_CANCEL, post(api::cancel_chat_message))
        .route(
            routes::SESSION_MESSAGES,
            delete(api::clear_session_messages),
        )
}

fn document_routes() -> Router<AppState> {
    Router::new()
        .route(
            routes::DOCUMENTS,
            get(api::list_documents).post(api::create_document),
        )
        .route(routes::DOCUMENTS_SEARCH, get(api::search_documents))
        .route(
            routes::DOCUMENT,
            get(api::get_document)
                .patch(api::update_document)
                .delete(api::delete_document),
        )
}

fn schema_routes() -> Router<AppState> {
    Router::new()
        .route(
            routes::OUTPUT_SCHEMAS,
            get(api::list_output_schemas).post(api::create_output_schema),
        )
        .route(
            routes::OUTPUT_SCHEMA,
            get(api::get_output_schema)
                .put(api::update_output_schema)
                .delete(api::delete_output_schema),
        )
        .route(
            routes::PROMPT_TEMPLATES,
            get(api::list_prompt_templates).post(api::create_prompt_template),
        )
        .route(
            routes::PROMPT_TEMPLATE,
            get(api::get_prompt_template)
                .put(api::update_prompt_template)
                .delete(api::delete_prompt_template),
        )
}

fn workflow_routes() -> Router<AppState> {
    Router::new()
        .route(
            routes::WORKFLOWS,
            get(api::list_workflows).post(api::create_workflow),
        )
        .route(
            routes::WORKFLOW,
            get(api::get_workflow)
                .put(api::update_workflow)
                .delete(api::delete_workflow),
        )
        .route(
            routes::WORKFLOW_STEPS,
            get(api::list_workflow_steps).post(api::create_workflow_step),
        )
        .route(
            routes::WORKFLOW_STEP,
            get(api::get_workflow_step)
                .patch(api::update_workflow_step)
                .delete(api::delete_workflow_step),
        )
        .route(
            routes::WORKFLOW_EDGES,
            get(api::list_workflow_edges)
                .post(api::add_workflow_edge)
                .delete(api::remove_workflow_edge),
        )
        .route(
            routes::WORKFLOW_EDGE,
            delete(api::delete_workflow_edge_by_id),
        )
        .route(
            routes::WORKFLOW_STEP_DOCUMENTS,
            get(api::list_step_documents)
                .post(api::add_step_document)
                .delete(api::remove_step_document),
        )
        .route(
            routes::WORKFLOW_AGENT_SESSION,
            get(api::get_or_create_workflow_agent_session),
        )
        .route(
            routes::WORKFLOW_STEP_CHAT_SESSION,
            get(api::get_step_session).post(api::get_or_create_step_session),
        )
        .route(
            routes::WORKFLOW_STEP_CHAT_MESSAGES,
            delete(api::clear_step_messages),
        )
        .route(
            routes::WORKFLOW_STEP_CHAT_DEBUG,
            get(api::get_step_chat_debug),
        )
        .route(
            routes::WORKFLOW_STEP_DISPATCH_HISTORY,
            get(api::get_step_dispatch_history),
        )
        .route(routes::WORKFLOW_STEP_CONFIG, get(api::get_step_config))
        .route(routes::WORKFLOW_STEP_PIN, post(api::toggle_step_pin))
        .route(routes::WORKFLOW_STEP_LAST_RUN, get(api::get_step_last_run))
        .route(routes::WORKFLOW_STEP_SUB_DAG, get(api::get_step_sub_dag))
        .route(routes::WORKFLOW_PLANS, get(api::get_workflow_plans))
        .route(routes::WORKFLOW_BOARD_SUBMIT, post(api::submit_board))
        .route(
            routes::WORKFLOW_BOARD_ELEMENTS,
            get(api::get_board_elements),
        )
        .route(
            routes::WORKFLOW_QUESTION_STATES,
            get(api::list_question_states),
        )
        .route(routes::WORKFLOW_RUN, post(api::run_workflow))
        .route(
            routes::WORKFLOW_WORKSHOP,
            post(api::get_or_create_workshop).get(api::get_workshop),
        )
        .route(
            routes::WORKFLOW_WORKSHOP_STEP_EXECUTE,
            post(api::execute_workshop_step),
        )
        .route(
            routes::WORKFLOW_TEMPLATES,
            post(api::create_template).get(api::list_templates),
        )
        .route(
            routes::WORKFLOW_TEMPLATE,
            get(api::get_template).delete(api::delete_template),
        )
        .route(routes::WORKFLOW_REBASE, post(api::rebase_workshop))
        .route(
            routes::WORKFLOW_EXECUTIONS,
            get(api::list_workflow_executions),
        )
        .route(routes::WORKFLOW_EXECUTION_STEPS, get(api::get_run_detail))
        .route(
            routes::WORKFLOW_EXECUTION_STEP,
            get(api::get_step_run_for_execution),
        )
}

fn collection_routes() -> Router<AppState> {
    Router::new()
        .route(
            routes::COLLECTIONS,
            get(api::list_collections).post(api::create_collection),
        )
        .route(
            routes::COLLECTION,
            get(api::get_collection)
                .put(api::update_collection)
                .delete(api::delete_collection),
        )
        .route(routes::COLLECTION_RUN, post(api::run_collection))
        .route(
            routes::COLLECTION_RUN_STATUS,
            get(api::get_collection_run_status),
        )
}

fn execution_routes() -> Router<AppState> {
    Router::new()
        .route(
            routes::AGENT_EXECUTION_CANCEL,
            post(api::cancel_agent_execution),
        )
        .route(routes::AGENT_EXECUTIONS, get(api::list_agent_executions))
        .route(routes::AGENT_EXECUTION, get(api::get_agent_execution))
        .route(
            routes::AGENT_EXECUTION_MESSAGES,
            get(api::list_execution_messages),
        )
        .route(
            routes::AGENT_EXECUTION_MESSAGE_STREAM,
            get(api::execution_message_stream),
        )
        .route(
            routes::AGENT_EXECUTION_APPROVE,
            post(api::approve_execution),
        )
        .route(routes::AGENT_EXECUTION_EXEMPLARY, put(api::set_exemplary))
        .route(routes::EXECUTION_TIMELINE, get(api::get_execution_timeline))
        .route(routes::COSTS, get(api::get_costs))
        .route(routes::RESULTS, get(api::list_results))
        .route(
            routes::RESULT,
            get(api::get_result).delete(api::delete_result),
        )
}

fn room_routes() -> Router<AppState> {
    Router::new()
        .route(routes::ROOMS, post(api::create_room))
        .route(
            routes::ROOM,
            get(api::get_room)
                .put(api::update_room)
                .delete(api::delete_room),
        )
        .route(
            routes::ROOM_MEMBERS,
            get(api::list_room_members)
                .post(api::add_room_member)
                .put(api::set_room_members),
        )
        .route(routes::ROOM_MEMBER, delete(api::remove_room_member))
        .route(routes::ROOM_SESSIONS, post(api::create_room_session))
        .route(routes::ROOM_SESSION, get(api::get_room_session))
        .route(routes::ROOM_SESSION_MESSAGES, post(api::send_room_message))
        .route(
            routes::ROOM_SESSION_TRANSCRIPT,
            get(api::get_room_transcript),
        )
        .route(routes::ROOM_SESSION_CLOSE, post(api::close_room_session))
        .route(routes::ROOM_SESSION_OUTPUTS, get(api::list_room_outputs))
}

fn step_config_routes() -> Router<AppState> {
    Router::new()
        .route(
            routes::STEP_INPUTS,
            get(api::list_step_inputs).post(api::create_step_input),
        )
        .route(routes::STEP_INPUT, delete(api::delete_step_input))
        .route(
            routes::STEP_OUTPUTS,
            get(api::list_step_outputs).post(api::create_step_output),
        )
        .route(routes::STEP_OUTPUT, delete(api::delete_step_output))
        .route(
            routes::STEP_AGENT_ROSTER,
            get(api::list_roster_agents).post(api::create_roster_agent),
        )
        .route(routes::STEP_ROSTER_AGENT, delete(api::delete_roster_agent))
        .route(routes::STEP_ROOM_MEMBERS, get(api::list_room_step_members))
        .route(
            routes::STEP_ROUTING_RULES,
            get(api::list_routing_rules).post(api::create_routing_rule),
        )
        .route(
            routes::STEP_ROUTING_RULE,
            put(api::update_routing_rule).delete(api::delete_routing_rule),
        )
}

fn protocol_routes() -> Router<AppState> {
    Router::new()
        .route(routes::PROTOCOL_TYPES, get(api::list_protocol_types))
        .route(
            routes::PROTOCOLS,
            get(api::list_protocols).post(api::create_protocol),
        )
        .route(
            routes::PROTOCOL,
            get(api::get_protocol)
                .put(api::update_protocol)
                .delete(api::delete_protocol),
        )
        .route(routes::PROTOCOL_PORTS, post(api::create_port))
        .route(
            routes::PROTOCOL_PORT,
            put(api::update_port).delete(api::delete_port),
        )
        .route(routes::PROTOCOL_PREVIEW, post(api::preview_expansion))
        .route(routes::PROTOCOL_APPLY, post(api::apply_protocol))
        .route(routes::PROTOCOL_UNAPPLY, delete(api::unapply_protocol))
        .route(
            routes::PROTOCOL_EXECUTIONS,
            get(api::list_protocol_executions),
        )
}

fn system_routes() -> Router<AppState> {
    Router::new()
        .route(
            routes::SYSTEM_CONFIGS,
            get(api::list_system_configs).post(api::upsert_system_config),
        )
        .route(routes::SYSTEM_CONFIG, delete(api::delete_system_config))
        .route(routes::ARCHETYPES, get(api::list_archetypes))
}

fn dispatch_routes() -> Router<AppState> {
    Router::new()
        .route(routes::DISPATCH_TRACE, get(api::get_dispatch_trace))
        .route(routes::DISPATCH_STEP_TASKS, get(api::list_dispatch_tasks))
        .route(routes::DISPATCH_SEND, post(api::dispatch_send))
        .route(routes::DISPATCH_CANCEL, post(api::dispatch_cancel))
        .route(routes::DISPATCH_SESSION, get(api::get_dispatch_session))
}
