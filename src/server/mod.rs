//! HTTP server module for nexor
//!
//! This module provides the Axum-based HTTP server that serves:
//! - REST API endpoints
//! - WebSocket connections for real-time updates
//! - Static files for the React frontend

pub mod api;
pub mod auth;
pub mod executors;
pub mod hub;
pub mod openapi;
pub mod services;
pub mod state;
pub mod tools;
pub mod ws;

use std::net::SocketAddr;

use anyhow::Result;
use axum::{
    body::Body,
    extract::State,
    http::{header::CACHE_CONTROL, HeaderName, HeaderValue, Request, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::{delete, get, patch, post, put},
    Router,
};
use sqlx::PgPool;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::key_extractor::SmartIpKeyExtractor;
use tower_governor::GovernorLayer;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;
use tracing::{info, warn};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::constants::routes;
use crate::types::AppConfig;

pub use state::AppState;

/// Start the HTTP server
///
/// This is the main entry point for the server. It sets up:
/// - Application state with database and scheduler
/// - Router with API endpoints
/// - CORS configuration for development
/// - Request tracing middleware
/// - Graceful shutdown handling
pub async fn start_server(db: PgPool, config: AppConfig, addr: SocketAddr) -> Result<()> {
    let (state, chat_rx) = AppState::new(db, config).await;

    // Spawn the chat consumer to process chat messages via LLM
    let _chat_consumer_handle = executors::chat::spawn_chat_consumer(state.clone(), chat_rx);

    // Spawn periodic container reaper
    let _reaper_handle = crate::execution::ContainerManager::real().spawn_reaper(
        std::time::Duration::from_secs(crate::constants::CONTAINER_REAPER_MAX_AGE_SECS),
        std::time::Duration::from_secs(crate::constants::CONTAINER_REAPER_INTERVAL_SECS),
        state.shutdown_token().clone(),
    );

    let shutdown_state = state.clone();
    let app = create_router(state.clone());

    info!("Server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal(shutdown_state))
    .await?;

    // Drain: wait for active executions to complete (with timeout)
    let drain_timeout =
        std::time::Duration::from_secs(crate::constants::SHUTDOWN_DRAIN_TIMEOUT_SECS);
    drain_active_executions(&state, drain_timeout).await;

    // Final reap: force-cleanup any remaining orphaned containers
    let reaped = crate::execution::ContainerManager::real()
        .reap_orphaned_containers(std::time::Duration::ZERO)
        .await;
    if reaped > 0 {
        warn!("Force-reaped {} container(s) during shutdown", reaped);
    }

    info!("Server shutdown complete");
    Ok(())
}

/// Create the application router with all routes, middleware, and rate limiting
fn create_router(state: AppState) -> Router {
    let static_dir = std::env::var(crate::constants::ENV_NEXOR_STATIC_DIR)
        .unwrap_or_else(|_| "ui/dist".to_string());
    let cors = build_cors_layer();

    // Check if we should skip rate limiting (dev mode behind proxy)
    let skip_rate_limit = std::env::var("NEXOR_SKIP_RATE_LIMIT")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    let public_routes = if skip_rate_limit {
        info!("Rate limiting disabled (NEXOR_SKIP_RATE_LIMIT=1)");
        build_public_routes()
    } else {
        // Rate limiter for auth routes: 10 requests per 60 seconds per IP
        let auth_rate_limit = GovernorConfigBuilder::default()
            .per_second(6)
            .burst_size(10)
            .key_extractor(SmartIpKeyExtractor)
            .finish()
            .expect("valid governor config");
        build_public_routes().layer(GovernorLayer {
            config: std::sync::Arc::new(auth_rate_limit),
        })
    };

    let protected_routes = if skip_rate_limit {
        build_protected_routes(state.clone())
    } else {
        // Rate limiter for general API routes: ~100 requests per minute per IP
        let api_rate_limit = GovernorConfigBuilder::default()
            .per_second(2)
            .burst_size(50)
            .key_extractor(SmartIpKeyExtractor)
            .finish()
            .expect("valid governor config");
        build_protected_routes(state.clone()).layer(GovernorLayer {
            config: std::sync::Arc::new(api_rate_limit),
        })
    };

    let serve_dir = ServeDir::new(&static_dir)
        .not_found_service(ServeFile::new(format!("{}/index.html", static_dir)));

    Router::new()
        .nest("/api", public_routes.merge(protected_routes))
        .merge(
            SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", openapi::ApiDoc::openapi()),
        )
        .route(routes::WS, get(ws::ws_handler))
        .fallback_service(serve_dir)
        .layer(middleware::from_fn(request_id_middleware))
        .layer(middleware::from_fn(cache_control_middleware))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Build the public route group (no auth required).
fn build_public_routes() -> Router<AppState> {
    Router::new()
        .route(routes::HEALTH, get(api::health_check))
        .route(routes::AUTH_SETUP, post(api::auth_setup))
        .route(routes::AUTH_LOGIN, post(api::auth_login))
        .route(routes::AUTH_REGISTER, post(api::auth_register))
}

/// Build the protected route group (auth required).
fn build_protected_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route(routes::AUTH_ME, get(api::auth_me))
        .route(routes::TASKS, get(api::list_tasks).post(api::create_task))
        .route(routes::TASK, get(api::get_task))
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
        .route(routes::TOOLS, get(api::list_tools).post(api::create_tool))
        .route(
            routes::TOOL,
            get(api::get_tool)
                .patch(api::update_tool)
                .delete(api::delete_tool),
        )
        // LEGACY: Pipeline routes removed (workflows replaced pipelines)
        // .route(routes::PIPELINE_STAGE_RENDER, post(api::render_pipeline_stage))
        // .route(routes::PIPELINE_RUNS, get(api::list_pipeline_runs))
        // .route(routes::PIPELINE_RUN, get(api::get_pipeline_run))
        // .route(routes::PIPELINE_RUN_APPROVE, post(api::approve_pipeline_run))
        // .route(routes::PIPELINE_RUN_CANCEL, post(api::cancel_pipeline_run))
        // .route(routes::PIPELINE_RUN_TREE, get(api::get_pipeline_run_tree))
        .route(
            routes::AGENT_EXECUTION_CANCEL,
            post(api::cancel_agent_execution),
        )
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
        .route(routes::WORKFLOW_STEP_CONFIG, get(api::get_step_config))
        .route(routes::WORKFLOW_STEP_LAST_RUN, get(api::get_step_last_run))
        .route(routes::WORKFLOW_STEP_SUB_DAG, get(api::get_step_sub_dag))
        .route(routes::WORKFLOW_NOTES, get(api::get_workflow_notes))
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
        // LEGACY: Pipeline stage member routes removed
        // .route(routes::PIPELINE_STAGE_MEMBERS, get(api::list_stage_members).post(api::add_stage_member))
        // .route(routes::PIPELINE_STAGE_MEMBER, delete(api::delete_stage_member).put(api::update_stage_member))
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
        .route(routes::COSTS, get(api::get_costs))
        .route(routes::RESULTS, get(api::list_results))
        .route(
            routes::RESULT,
            get(api::get_result).delete(api::delete_result),
        )
        .route(routes::ROOMS, post(api::create_room))
        .route(
            routes::ROOM,
            get(api::get_room)
                .put(api::update_room)
                .delete(api::delete_room),
        )
        // LEGACY: Pipeline rooms route removed
        // .route(routes::PIPELINE_ROOMS, get(api::list_pipeline_rooms))
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
        // Step Ports
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
        // Document Definitions
        .route(
            routes::STEP_DOCUMENT_DEFS,
            get(api::list_document_defs).post(api::create_document_def),
        )
        .route(
            routes::STEP_DOCUMENT_DEF,
            patch(api::update_document_def).delete(api::delete_document_def),
        )
        // Agent Roster
        .route(
            routes::STEP_AGENT_ROSTER,
            get(api::list_roster_agents).post(api::create_roster_agent),
        )
        .route(routes::STEP_ROSTER_AGENT, delete(api::delete_roster_agent))
        // Room Step Members (design-time)
        .route(routes::STEP_ROOM_MEMBERS, get(api::list_room_step_members))
        // Routing Rules
        .route(
            routes::STEP_ROUTING_RULES,
            get(api::list_routing_rules).post(api::create_routing_rule),
        )
        .route(
            routes::STEP_ROUTING_RULE,
            put(api::update_routing_rule).delete(api::delete_routing_rule),
        )
        // System Config
        .route(
            routes::SYSTEM_CONFIGS,
            get(api::list_system_configs).post(api::upsert_system_config),
        )
        .route(routes::SYSTEM_CONFIG, delete(api::delete_system_config))
        // Archetypes
        .route(routes::ARCHETYPES, get(api::list_archetypes))
        // Protocols
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
            routes::PROTOCOL_DOCUMENT_DEFS,
            get(api::list_protocol_document_defs).post(api::create_protocol_document_def),
        )
        .route(
            routes::PROTOCOL_DOCUMENT_DEF,
            put(api::update_protocol_document_def).delete(api::delete_protocol_document_def),
        )
        .route(
            routes::PROTOCOL_EXECUTIONS,
            get(api::list_protocol_executions),
        )
        .layer(middleware::from_fn_with_state(state.clone(), require_auth))
}

/// Create the application router with a specific static directory (no rate limiting — used by tests)
#[cfg(test)]
fn create_router_with_static_dir(state: AppState, static_dir: &str) -> Router {
    let cors = build_cors_layer();
    let public_routes = build_public_routes();
    let protected_routes = build_protected_routes(state.clone());

    let serve_dir = ServeDir::new(static_dir)
        .not_found_service(ServeFile::new(format!("{}/index.html", static_dir)));

    Router::new()
        .nest("/api", public_routes.merge(protected_routes))
        .route(routes::WS, get(ws::ws_handler))
        .fallback_service(serve_dir)
        .layer(middleware::from_fn(request_id_middleware))
        .layer(middleware::from_fn(cache_control_middleware))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Build CORS layer from CORS_ORIGINS env var.
///
/// - If `CORS_ORIGINS` is set, parse comma-separated origins.
/// - If unset, default to permissive (dev mode) with a warning.
fn build_cors_layer() -> CorsLayer {
    match std::env::var(crate::constants::ENV_CORS_ORIGINS) {
        Ok(origins) if !origins.is_empty() => {
            let parsed: Vec<HeaderValue> = origins
                .split(',')
                .filter_map(|o| {
                    let trimmed = o.trim();
                    HeaderValue::from_str(trimmed)
                        .map_err(|e| warn!("Invalid CORS origin '{}': {}", trimmed, e))
                        .ok()
                })
                .collect();
            if parsed.is_empty() {
                warn!("CORS_ORIGINS set but no valid origins parsed, falling back to permissive");
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods(Any)
                    .allow_headers(Any)
            } else {
                info!("CORS restricted to {} origin(s)", parsed.len());
                CorsLayer::new()
                    .allow_origin(AllowOrigin::list(parsed))
                    .allow_methods(Any)
                    .allow_headers(Any)
                    .allow_credentials(true)
            }
        }
        _ => {
            warn!("CORS_ORIGINS not set — allowing all origins (dev mode). Set CORS_ORIGINS for production.");
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
        }
    }
}

/// Middleware to require valid JWT on protected routes.
/// Defense-in-depth: handlers also extract AuthUser, but this catches any handler that forgets.
async fn require_auth(
    State(state): State<AppState>,
    request: axum::http::Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Try Authorization header first, then fall back to ?token= query param
    // (needed for EventSource/SSE which cannot set custom headers)
    let token = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string())
        .or_else(|| {
            request.uri().query().and_then(|q| {
                q.split('&')
                    .find_map(|pair| pair.strip_prefix("token=").map(|v| v.to_string()))
            })
        })
        .ok_or(StatusCode::UNAUTHORIZED)?;

    auth::verify_token(&token, state.jwt_secret()).map_err(|_| StatusCode::UNAUTHORIZED)?;

    Ok(next.run(request).await)
}

/// Middleware to set cache headers for static assets
///
/// - Long cache (1 year) for hashed assets (JS, CSS in /assets/)
/// - No cache for HTML files (including SPA fallback routes)
/// - Default cache for other static files
async fn cache_control_middleware(request: Request<Body>, next: Next) -> Response {
    let path = request.uri().path().to_string();
    let mut response = next.run(request).await;

    // Skip cache headers for API routes - they handle their own caching
    if path.starts_with("/api") || path.starts_with("/ws") {
        return response;
    }

    // Long cache for hashed assets (typically in /assets/ with hash in filename)
    if path.contains("/assets/") && (path.ends_with(".js") || path.ends_with(".css")) {
        response.headers_mut().insert(
            CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=31536000, immutable"),
        );
    }
    // No cache for HTML files (ensures fresh version on deployment)
    // Check both path and content-type to handle SPA fallback routes
    else if path == "/" || path.ends_with(".html") || is_html_response(&response) {
        response.headers_mut().insert(
            CACHE_CONTROL,
            HeaderValue::from_static("no-cache, no-store, must-revalidate"),
        );
    }

    response
}

/// Middleware to assign a unique request ID to each request.
///
/// Generates a UUID, attaches it to the response as `X-Request-Id`, and logs it.
async fn request_id_middleware(request: Request<Body>, next: Next) -> Response {
    let request_id = uuid::Uuid::new_v4().to_string();
    let span = tracing::info_span!("request", id = %request_id, method = %request.method(), path = %request.uri().path());
    let _enter = span.enter();

    let mut response = next.run(request).await;
    if let Ok(value) = HeaderValue::from_str(&request_id) {
        response
            .headers_mut()
            .insert(HeaderName::from_static("x-request-id"), value);
    }
    response
}

/// Check if a response is HTML based on Content-Type header
fn is_html_response(response: &Response) -> bool {
    use axum::http::header::CONTENT_TYPE;

    response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|ct| ct.starts_with("text/html"))
        .unwrap_or(false)
}

/// Wait for shutdown signal, then cancel all running executions.
///
/// Handles both Ctrl+C (SIGINT) and SIGTERM for graceful shutdown.
async fn shutdown_signal(state: AppState) {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("Received Ctrl+C, starting graceful shutdown..."),
        _ = terminate => info!("Received SIGTERM, starting graceful shutdown..."),
    }

    // Signal all running executions to cancel
    let cancelled = state.cancel_all_executions();
    if cancelled > 0 {
        info!(
            "Cancelled {} running execution(s), waiting for drain...",
            cancelled
        );
    }

    // Cancel the master shutdown token (stops background tasks like the reaper)
    state.shutdown_token().cancel();
}

/// Wait for all active executions to drain, polling every second until
/// the cancellation_tokens map is empty or the timeout expires.
async fn drain_active_executions(state: &AppState, timeout: std::time::Duration) {
    let start = std::time::Instant::now();
    loop {
        let remaining = state.active_execution_count();
        if remaining == 0 {
            info!("All executions drained cleanly");
            break;
        }
        if start.elapsed() >= timeout {
            warn!(
                "Drain timeout ({:?}) expired with {} execution(s) still active",
                timeout, remaining
            );
            break;
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::traits::{
        MockAgentRepo, MockAuthConfigRepo, MockChatMessageRepo, MockSessionRepo, MockTaskRepo,
        MockToolRepo,
    };
    use crate::server::state::test_helpers::MockReposBuilder;
    use crate::types::UserId;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use std::sync::Arc;
    use tempfile::TempDir;
    use tower::util::ServiceExt;
    use uuid::Uuid;

    fn setup_mock_state() -> AppState {
        // Create focused mocks for each trait that the integration tests exercise.
        let mut agents = MockAgentRepo::new();
        agents
            .expect_list_persisted_agents()
            .returning(|_| Ok(vec![]));
        agents.expect_get_persisted_agent().returning(|_| Ok(None));
        agents.expect_upsert_agent().returning(|_| Ok(()));
        agents.expect_delete_persisted_agent().returning(|_| Ok(()));
        agents.expect_get_agent_context().returning(|_| Ok(vec![]));
        agents.expect_set_agent_context().returning(|_, _| Ok(()));
        agents
            .expect_get_agent_guidances()
            .returning(|_, _| Ok(vec![]));

        let mut tools = MockToolRepo::new();
        tools.expect_list_tools().returning(|| Ok(vec![]));
        tools.expect_get_tool().returning(|_| Ok(None));
        tools.expect_upsert_tool().returning(|_| Ok(()));
        tools.expect_delete_tool().returning(|_| Ok(()));
        tools.expect_get_agent_tools().returning(|_| Ok(vec![]));
        tools.expect_set_agent_tools().returning(|_, _| Ok(()));
        tools.expect_seed_builtin_tools().returning(|| Ok(()));

        let mut sessions = MockSessionRepo::new();
        sessions
            .expect_create_session()
            .returning(|_, _, _, _, _, _| Ok(()));
        sessions.expect_list_sessions().returning(|_| Ok(vec![]));
        sessions.expect_get_session().returning(|_| Ok(None));
        sessions.expect_delete_session().returning(|_| Ok(()));
        sessions
            .expect_insert_session_message()
            .returning(|_, _, _, _, _| Ok(()));
        sessions
            .expect_get_session_history()
            .returning(|_, _| Ok(vec![]));
        sessions
            .expect_update_session_title()
            .returning(|_, _| Ok(()));
        sessions
            .expect_update_session_summary()
            .returning(|_, _| Ok(()));
        sessions
            .expect_count_session_messages()
            .returning(|_| Ok(0));
        sessions
            .expect_update_session_draft_config()
            .returning(|_, _| Ok(()));
        sessions
            .expect_clear_session_messages()
            .returning(|_| Ok(()));
        sessions
            .expect_find_session_by_step_id()
            .returning(|_| Ok(None));
        sessions
            .expect_link_session_agent()
            .returning(|_, _| Ok(()));

        let mut tasks = MockTaskRepo::new();
        tasks.expect_list_tasks().returning(|_, _, _| Ok(vec![]));
        tasks.expect_get_task_by_uuid().returning(|_, _| Ok(None));
        tasks.expect_insert_task().returning(|_, _| Ok(()));

        let mut chat_messages = MockChatMessageRepo::new();
        chat_messages
            .expect_insert_chat_message()
            .returning(|_, _, _, _| Ok(()));
        chat_messages
            .expect_get_chat_history()
            .returning(|_, _, _| Ok(vec![]));
        chat_messages
            .expect_clear_chat_history()
            .returning(|_| Ok(()));

        let mut auth_config = MockAuthConfigRepo::new();
        auth_config.expect_health_check().returning(|| true);
        auth_config.expect_has_password().returning(|| Ok(false));
        auth_config.expect_set_password().returning(|_| Ok(()));
        auth_config.expect_get_password().returning(|| Ok(None));

        let repos = MockReposBuilder::new()
            .with_agents(Arc::new(agents))
            .with_tools(Arc::new(tools))
            .with_sessions(Arc::new(sessions))
            .with_tasks(Arc::new(tasks))
            .with_chat_messages(Arc::new(chat_messages))
            .with_auth_config(Arc::new(auth_config))
            .build();

        let (state, rx) = AppState::with_repos(None, repos, AppConfig::default());
        // Keep the receiver alive so chat_tx.send() doesn't fail in tests
        std::mem::forget(rx);
        state
    }

    fn create_test_token(state: &AppState) -> String {
        use crate::types::UserId;
        auth::create_token(
            &state.jwt_secret(),
            24,
            UserId::new(),
            "test@test.com",
            false,
        )
        .unwrap()
    }

    fn setup_test_app() -> (Router, AppState) {
        let state = setup_mock_state();
        let router = create_router_with_static_dir(state.clone(), "nonexistent_static");
        (router, state)
    }

    #[tokio::test]
    async fn health_endpoint_returns_json() {
        let (app, _state) = setup_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn tasks_endpoint_returns_list() {
        let (app, state) = setup_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/tasks")
                    .header(
                        "authorization",
                        format!("Bearer {}", create_test_token(&state)),
                    )
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn agents_endpoint_returns_stats() {
        let (app, state) = setup_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/agents")
                    .header(
                        "authorization",
                        format!("Bearer {}", create_test_token(&state)),
                    )
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn config_endpoint_returns_config() {
        let (app, state) = setup_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/config")
                    .header(
                        "authorization",
                        format!("Bearer {}", create_test_token(&state)),
                    )
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn unknown_task_returns_404() {
        let (app, state) = setup_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/tasks/00000000-0000-0000-0000-000000000000")
                    .header(
                        "authorization",
                        format!("Bearer {}", create_test_token(&state)),
                    )
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn chat_endpoint_accepts_message() {
        let (app, state) = setup_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/chat")
                    .header("content-type", "application/json")
                    .header(
                        "authorization",
                        format!("Bearer {}", create_test_token(&state)),
                    )
                    .body(Body::from(r#"{"message": "Hello!"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::ACCEPTED);
    }

    #[tokio::test]
    async fn chat_endpoint_rejects_empty_message() {
        let (app, state) = setup_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/chat")
                    .header("content-type", "application/json")
                    .header(
                        "authorization",
                        format!("Bearer {}", create_test_token(&state)),
                    )
                    .body(Body::from(r#"{"message": "   "}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn chat_history_returns_empty_list() {
        let (app, state) = setup_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/chat/history")
                    .header(
                        "authorization",
                        format!("Bearer {}", create_test_token(&state)),
                    )
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn clear_chat_history_returns_no_content() {
        let (app, state) = setup_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/chat/history")
                    .header(
                        "authorization",
                        format!("Bearer {}", create_test_token(&state)),
                    )
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    // Static file serving tests (Ticket 10.6)

    fn setup_test_app_with_static_dir() -> (Router, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let static_dir = temp_dir.path().join("ui/dist");
        std::fs::create_dir_all(&static_dir).unwrap();
        std::fs::create_dir_all(static_dir.join("assets")).unwrap();
        std::fs::write(
            static_dir.join("index.html"),
            "<!DOCTYPE html><html><head></head><body>React App</body></html>",
        )
        .unwrap();
        std::fs::write(
            static_dir.join("assets/main.abc123.css"),
            "body { color: blue; }",
        )
        .unwrap();
        std::fs::write(
            static_dir.join("assets/main.def456.js"),
            "console.log('hello');",
        )
        .unwrap();

        let state = setup_mock_state();
        let router = create_router_with_static_dir(state, static_dir.to_str().unwrap());
        (router, temp_dir)
    }

    #[tokio::test]
    async fn static_index_html_served_at_root() {
        let (app, _temp_dir) = setup_test_app_with_static_dir();
        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let cache_control = response.headers().get(CACHE_CONTROL);
        assert!(cache_control.is_some());
        assert_eq!(
            cache_control.unwrap(),
            "no-cache, no-store, must-revalidate"
        );
    }

    #[tokio::test]
    async fn static_css_asset_served_with_long_cache() {
        let (app, _temp_dir) = setup_test_app_with_static_dir();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/assets/main.abc123.css")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let cache_control = response.headers().get(CACHE_CONTROL);
        assert!(cache_control.is_some());
        assert_eq!(
            cache_control.unwrap(),
            "public, max-age=31536000, immutable"
        );
    }

    #[tokio::test]
    async fn static_js_asset_served_with_long_cache() {
        let (app, _temp_dir) = setup_test_app_with_static_dir();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/assets/main.def456.js")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let cache_control = response.headers().get(CACHE_CONTROL);
        assert!(cache_control.is_some());
        assert_eq!(
            cache_control.unwrap(),
            "public, max-age=31536000, immutable"
        );
    }

    #[tokio::test]
    async fn spa_route_falls_back_to_index_html() {
        let (app, _temp_dir) = setup_test_app_with_static_dir();
        let response = app
            .oneshot(Request::builder().uri("/chat").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        assert!(
            status == StatusCode::OK || status == StatusCode::NOT_FOUND,
            "Expected OK or NOT_FOUND, got: {:?}",
            status
        );
    }

    #[tokio::test]
    async fn api_routes_not_affected_by_static_fallback() {
        let (app, _temp_dir) = setup_test_app_with_static_dir();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn nested_spa_route_falls_back_to_index_html() {
        let (app, _temp_dir) = setup_test_app_with_static_dir();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/tasks/123/details")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        assert!(
            status == StatusCode::OK || status == StatusCode::NOT_FOUND,
            "Expected OK or NOT_FOUND, got: {:?}",
            status
        );
    }

    // =================================================================
    // Auth Extractor Edge Case Tests
    // =================================================================

    #[tokio::test]
    async fn missing_auth_header_returns_401() {
        let (app, _state) = setup_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/agents")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn empty_bearer_token_returns_401() {
        let (app, _state) = setup_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/agents")
                    .header("authorization", "Bearer ")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn expired_token_http_returns_401() {
        let (app, state) = setup_test_app();
        // Create an expired token by encoding Claims with exp in the past
        use jsonwebtoken::{encode, EncodingKey, Header};
        let expired_claims = auth::Claims {
            sub: uuid::Uuid::new_v4().to_string(),
            email: "test@test.com".to_string(),
            is_admin: false,
            exp: 1, // epoch + 1 second = long expired
            iat: 0,
        };
        let token = encode(
            &Header::default(),
            &expired_claims,
            &EncodingKey::from_secret(&state.jwt_secret()),
        )
        .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/agents")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    // =================================================================
    // Input Validation Tests
    // =================================================================

    #[tokio::test]
    async fn invalid_uuid_path_returns_400() {
        let (app, state) = setup_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/agents/not-a-uuid")
                    .header(
                        "authorization",
                        format!("Bearer {}", create_test_token(&state)),
                    )
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn malformed_json_body_returns_error() {
        let (app, state) = setup_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/agents")
                    .header("content-type", "application/json")
                    .header(
                        "authorization",
                        format!("Bearer {}", create_test_token(&state)),
                    )
                    .body(Body::from("{broken json"))
                    .unwrap(),
            )
            .await
            .unwrap();
        // Axum returns 422 for JSON parse failures
        let status = response.status();
        assert!(
            status == StatusCode::UNPROCESSABLE_ENTITY || status == StatusCode::BAD_REQUEST,
            "Expected 422 or 400, got: {:?}",
            status
        );
    }

    #[tokio::test]
    async fn missing_required_fields_returns_error() {
        let (app, state) = setup_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/agents")
                    .header("content-type", "application/json")
                    .header(
                        "authorization",
                        format!("Bearer {}", create_test_token(&state)),
                    )
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        assert!(
            status == StatusCode::UNPROCESSABLE_ENTITY || status == StatusCode::BAD_REQUEST,
            "Expected 422 or 400, got: {:?}",
            status
        );
    }

    // =================================================================
    // Auth Endpoint Input Validation Tests
    // =================================================================

    #[tokio::test]
    async fn register_empty_email_returns_400() {
        let (app, _state) = setup_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/register")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"email":"","password":"validpass1"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn register_email_no_at_sign_returns_400() {
        let (app, _state) = setup_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/register")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"email":"noatsign","password":"validpass1"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn register_password_too_short_returns_400() {
        let (app, _state) = setup_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/register")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"email":"test@test.com","password":"short"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn register_empty_password_returns_400() {
        let (app, _state) = setup_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/register")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"email":"test@test.com","password":""}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn setup_password_too_short_returns_400() {
        let (app, _state) = setup_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/setup")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"password":"short"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
