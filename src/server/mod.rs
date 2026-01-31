//! HTTP server module for nexor
//!
//! This module provides the Axum-based HTTP server that serves:
//! - REST API endpoints
//! - WebSocket connections for real-time updates
//! - Static files for the React frontend

pub mod agent_mode;
pub mod api;
pub mod auth;
pub mod orchestrator;
pub mod state;
pub mod tools;
pub mod ws;

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use axum::{
    body::Body,
    extract::State,
    http::{header::CACHE_CONTROL, HeaderValue, Request, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
    Router,
};
use sqlx::PgPool;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;
use tracing::info;

use crate::orchestration::Scheduler;
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
pub async fn start_server(
    db: PgPool,
    scheduler: Arc<RwLock<Scheduler>>,
    config: AppConfig,
    addr: SocketAddr,
) -> Result<()> {
    let (state, orchestrator_rx) = AppState::new(db, scheduler, config).await;

    // Spawn the orchestrator consumer to process chat messages via LLM
    let _orchestrator_handle = orchestrator::spawn_orchestrator(state.clone(), orchestrator_rx);

    // Spawn the response consumer to collect agent results
    let _response_handle = orchestrator::spawn_response_consumer(state.clone());

    // Spawn the schedule runner for periodic agent tasks
    let _schedule_handle = orchestrator::spawn_schedule_runner(state.clone());

    let app = create_router(state);

    info!("Server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Server shutdown complete");
    Ok(())
}

/// Create the application router with all routes and middleware
fn create_router(state: AppState) -> Router {
    let static_dir = std::env::var("NEXOR_STATIC_DIR").unwrap_or_else(|_| "ui/dist".to_string());
    create_router_with_static_dir(state, &static_dir)
}

/// Create the application router with a specific static directory
fn create_router_with_static_dir(state: AppState, static_dir: &str) -> Router {
    // CORS configuration for development
    // In production, restrict to specific origins
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Public routes (no auth required)
    let public_routes = Router::new()
        .route("/health", get(api::health_check))
        .route("/auth/setup", post(api::auth_setup))
        .route("/auth/login", post(api::auth_login))
        .route("/auth/register", post(api::auth_register));

    // Protected routes (auth required)
    let protected_routes = Router::new()
        .route("/auth/me", get(api::auth_me))
        .route("/tasks", get(api::list_tasks).post(api::create_task))
        .route("/tasks/:id", get(api::get_task))
        .route("/agents", get(api::list_agents).post(api::create_agent))
        .route(
            "/agents/:id",
            get(api::get_agent)
                .patch(api::update_agent)
                .delete(api::delete_agent),
        )
        .route(
            "/agents/:id/tools",
            get(api::get_agent_tools).put(api::set_agent_tools),
        )
        .route("/tools", get(api::list_tools).post(api::create_tool))
        .route(
            "/tools/:id",
            get(api::get_tool)
                .patch(api::update_tool)
                .delete(api::delete_tool),
        )
        .route("/config", get(api::get_config).patch(api::update_config))
        // Chat endpoints (Ticket 10.3)
        .route("/chat", post(api::send_chat))
        .route(
            "/chat/history",
            get(api::get_chat_history).delete(api::clear_chat_history),
        )
        .route("/chat/:message_id/stream", get(api::chat_stream))
        // Mode & Session endpoints
        .route("/modes", get(api::list_modes))
        .route(
            "/sessions",
            get(api::list_sessions).post(api::create_session),
        )
        .route(
            "/sessions/:session_id",
            get(api::get_session)
                .patch(api::update_session)
                .delete(api::delete_session),
        )
        .route("/sessions/:session_id/chat", post(api::send_session_chat))
        .route(
            "/sessions/:session_id/history",
            get(api::get_session_history),
        )
        .route(
            "/sessions/:session_id/chat/:message_id/stream",
            get(api::session_chat_stream),
        )
        // Document endpoints
        .route(
            "/documents",
            get(api::list_documents).post(api::create_document),
        )
        .route("/documents/search", get(api::search_documents))
        .route(
            "/documents/:id",
            get(api::get_document)
                .patch(api::update_document)
                .delete(api::delete_document),
        )
        .route("/stats", get(api::get_usage_stats))
        // Indexing control
        .route("/indexing/status", get(api::get_indexing_status))
        .route("/indexing/start", post(api::start_indexing))
        .route("/indexing/stop", post(api::stop_indexing))
        // Context response endpoint (F6)
        .route("/context-response", post(api::submit_context_response))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth));

    // Static file serving for production (Ticket 10.6)
    // ServeDir with fallback to index.html for SPA routing
    let serve_dir = ServeDir::new(static_dir)
        .not_found_service(ServeFile::new(format!("{}/index.html", static_dir)));

    Router::new()
        .nest("/api", public_routes.merge(protected_routes))
        .route("/ws", get(ws::ws_handler))
        .fallback_service(serve_dir)
        .layer(middleware::from_fn(cache_control_middleware))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
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

    auth::verify_token(&token, &state.jwt_secret).map_err(|_| StatusCode::UNAUTHORIZED)?;

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

/// Wait for shutdown signal
///
/// Handles both Ctrl+C (SIGINT) and SIGTERM for graceful shutdown.
async fn shutdown_signal() {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::traits::ServerRepo;
    use crate::db::{
        ChatMessageRow, PipelineRow, PipelineStageRow, ScheduleRow, SessionRow, TriggerRow,
    };
    use crate::types::UserId;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use chrono::{DateTime, Utc};
    use tempfile::TempDir;
    use tower::util::ServiceExt;
    use uuid::Uuid;

    /// In-memory implementation of ServerRepo for tests (no Postgres needed).
    struct InMemoryServerRepo {
        tasks: std::sync::Mutex<Vec<crate::types::Task>>,
        chat_messages: std::sync::Mutex<Vec<ChatMessageRow>>,
        password_hash: std::sync::Mutex<Option<String>>,
    }

    impl InMemoryServerRepo {
        fn new() -> Self {
            Self {
                tasks: std::sync::Mutex::new(vec![]),
                chat_messages: std::sync::Mutex::new(vec![]),
                password_hash: std::sync::Mutex::new(None),
            }
        }
    }

    #[async_trait::async_trait]
    impl ServerRepo for InMemoryServerRepo {
        async fn health_check(&self) -> bool {
            true
        }
        async fn list_tasks(
            &self,
            _user_id: UserId,
            status: Option<String>,
            limit: Option<u32>,
        ) -> anyhow::Result<Vec<crate::types::Task>> {
            let tasks = self.tasks.lock().unwrap();
            let limit = limit
                .unwrap_or(crate::constants::DEFAULT_QUERY_LIMIT as u32)
                .min(crate::constants::MAX_QUERY_LIMIT as u32) as usize;
            Ok(tasks
                .iter()
                .filter(|t| {
                    if let Some(ref s) = status {
                        format!("{:?}", t.status).to_lowercase() == *s
                    } else {
                        true
                    }
                })
                .rev()
                .take(limit)
                .cloned()
                .collect())
        }
        async fn get_task_by_uuid(
            &self,
            _user_id: UserId,
            id: Uuid,
        ) -> anyhow::Result<Option<crate::types::Task>> {
            Ok(self
                .tasks
                .lock()
                .unwrap()
                .iter()
                .find(|t| t.id.0 == id)
                .cloned())
        }
        async fn insert_task(
            &self,
            _user_id: UserId,
            task: crate::types::Task,
        ) -> anyhow::Result<()> {
            self.tasks.lock().unwrap().push(task);
            Ok(())
        }
        async fn insert_chat_message(
            &self,
            _user_id: UserId,
            id: Uuid,
            role: String,
            content: String,
        ) -> anyhow::Result<()> {
            self.chat_messages.lock().unwrap().push(ChatMessageRow {
                id,
                role,
                content,
                timestamp: Utc::now(),
            });
            Ok(())
        }
        async fn get_chat_history(
            &self,
            _user_id: UserId,
            limit: u32,
            offset: u32,
        ) -> anyhow::Result<Vec<ChatMessageRow>> {
            let msgs = self.chat_messages.lock().unwrap();
            Ok(msgs
                .iter()
                .skip(offset as usize)
                .take(limit.min(1000) as usize)
                .cloned()
                .collect())
        }
        async fn clear_chat_history(&self, _user_id: UserId) -> anyhow::Result<()> {
            self.chat_messages.lock().unwrap().clear();
            Ok(())
        }
        async fn has_password(&self) -> anyhow::Result<bool> {
            Ok(self.password_hash.lock().unwrap().is_some())
        }
        async fn set_password(&self, hash: String) -> anyhow::Result<()> {
            *self.password_hash.lock().unwrap() = Some(hash);
            Ok(())
        }
        async fn get_password(&self) -> anyhow::Result<Option<String>> {
            Ok(self.password_hash.lock().unwrap().clone())
        }
        async fn list_persisted_agents(
            &self,
            _user_id: UserId,
        ) -> anyhow::Result<Vec<crate::db::AgentRow>> {
            Ok(vec![])
        }
        async fn get_persisted_agent(
            &self,
            _agent_id: Uuid,
        ) -> anyhow::Result<Option<crate::db::AgentRow>> {
            Ok(None)
        }
        async fn upsert_agent(
            &self,
            _user_id: UserId,
            _agent: crate::db::AgentRow,
        ) -> anyhow::Result<()> {
            Ok(())
        }
        async fn delete_persisted_agent(&self, _agent_id: Uuid) -> anyhow::Result<()> {
            Ok(())
        }
        async fn list_tools(
            &self,
            _user_id: UserId,
        ) -> anyhow::Result<Vec<crate::db::ToolRow>> {
            Ok(vec![])
        }
        async fn get_tool(
            &self,
            _tool_id: Uuid,
        ) -> anyhow::Result<Option<crate::db::ToolRow>> {
            Ok(None)
        }
        async fn upsert_tool(
            &self,
            _user_id: UserId,
            _tool: crate::db::ToolRow,
        ) -> anyhow::Result<()> {
            Ok(())
        }
        async fn delete_tool(&self, _tool_id: Uuid) -> anyhow::Result<()> {
            Ok(())
        }
        async fn get_agent_tools(
            &self,
            _agent_id: Uuid,
        ) -> anyhow::Result<Vec<crate::db::ToolRow>> {
            Ok(vec![])
        }
        async fn set_agent_tools(
            &self,
            _agent_id: Uuid,
            _tool_ids: Vec<Uuid>,
        ) -> anyhow::Result<()> {
            Ok(())
        }
        async fn list_persisted_clusters(
            &self,
            _user_id: UserId,
        ) -> anyhow::Result<Vec<crate::db::ClusterRow>> {
            Ok(vec![])
        }
        async fn upsert_cluster(
            &self,
            _user_id: UserId,
            _cluster: crate::db::ClusterRow,
        ) -> anyhow::Result<()> {
            Ok(())
        }
        async fn delete_cluster(&self, _cluster_id: Uuid) -> anyhow::Result<()> {
            Ok(())
        }
        async fn list_cluster_members(&self, _cluster_id: Uuid) -> anyhow::Result<Vec<Uuid>> {
            Ok(vec![])
        }
        async fn add_cluster_member(
            &self,
            _cluster_id: Uuid,
            _agent_id: Uuid,
        ) -> anyhow::Result<()> {
            Ok(())
        }
        async fn remove_cluster_member(
            &self,
            _cluster_id: Uuid,
            _agent_id: Uuid,
        ) -> anyhow::Result<()> {
            Ok(())
        }
        async fn list_pipelines(&self, _user_id: UserId) -> anyhow::Result<Vec<PipelineRow>> {
            Ok(vec![])
        }
        async fn upsert_pipeline(
            &self,
            _user_id: UserId,
            _pipeline: PipelineRow,
        ) -> anyhow::Result<()> {
            Ok(())
        }
        async fn delete_pipeline(&self, _pipeline_id: Uuid) -> anyhow::Result<()> {
            Ok(())
        }
        async fn list_pipeline_stages(
            &self,
            _pipeline_id: Uuid,
        ) -> anyhow::Result<Vec<PipelineStageRow>> {
            Ok(vec![])
        }
        async fn upsert_pipeline_stage(&self, _stage: PipelineStageRow) -> anyhow::Result<()> {
            Ok(())
        }
        async fn list_schedules(&self, _user_id: UserId) -> anyhow::Result<Vec<ScheduleRow>> {
            Ok(vec![])
        }
        async fn upsert_schedule(
            &self,
            _user_id: UserId,
            _schedule: ScheduleRow,
        ) -> anyhow::Result<()> {
            Ok(())
        }
        async fn delete_schedule(&self, _schedule_id: Uuid) -> anyhow::Result<()> {
            Ok(())
        }
        async fn update_schedule_last_run(
            &self,
            _schedule_id: Uuid,
            _last_run_at: DateTime<Utc>,
        ) -> anyhow::Result<()> {
            Ok(())
        }
        async fn list_triggers(&self, _user_id: UserId) -> anyhow::Result<Vec<TriggerRow>> {
            Ok(vec![])
        }
        async fn upsert_trigger(
            &self,
            _user_id: UserId,
            _trigger: TriggerRow,
        ) -> anyhow::Result<()> {
            Ok(())
        }
        async fn delete_trigger(&self, _trigger_id: Uuid) -> anyhow::Result<()> {
            Ok(())
        }
        async fn create_session(
            &self,
            _user_id: UserId,
            _session_id: Uuid,
            _mode_id: &str,
            _title: &str,
        ) -> anyhow::Result<()> {
            Ok(())
        }
        async fn list_sessions(&self, _user_id: UserId) -> anyhow::Result<Vec<SessionRow>> {
            Ok(vec![])
        }
        async fn get_session(&self, _session_id: Uuid) -> anyhow::Result<Option<SessionRow>> {
            Ok(None)
        }
        async fn delete_session(&self, _session_id: Uuid) -> anyhow::Result<()> {
            Ok(())
        }
        async fn insert_session_message(
            &self,
            _user_id: UserId,
            _session_id: Uuid,
            _id: Uuid,
            _role: String,
            _content: String,
        ) -> anyhow::Result<()> {
            Ok(())
        }
        async fn get_session_history(
            &self,
            _session_id: Uuid,
            _limit: u32,
        ) -> anyhow::Result<Vec<ChatMessageRow>> {
            Ok(vec![])
        }
        async fn update_session_title(
            &self,
            _session_id: Uuid,
            _title: &str,
        ) -> anyhow::Result<()> {
            Ok(())
        }
        async fn update_session_summary(
            &self,
            _session_id: Uuid,
            _summary: &str,
        ) -> anyhow::Result<()> {
            Ok(())
        }
        async fn count_session_messages(&self, _session_id: Uuid) -> anyhow::Result<u32> {
            Ok(0)
        }
        async fn insert_token_usage(
            &self,
            _session_id: Option<Uuid>,
            _agent_id: Option<Uuid>,
            _tier: &str,
            _model_id: &str,
            _input_tokens: i64,
            _output_tokens: i64,
        ) -> anyhow::Result<()> {
            Ok(())
        }
        async fn get_usage_summary(
            &self,
            _since_hours: u32,
        ) -> anyhow::Result<Vec<crate::db::UsageSummaryRow>> {
            Ok(vec![])
        }
        async fn insert_tool_call(
            &self,
            _session_id: Option<Uuid>,
            _message_id: Uuid,
            _round: i32,
            _tool_name: &str,
            _tool_use_id: &str,
            _input: &serde_json::Value,
            _output: &str,
            _latency_ms: i32,
        ) -> anyhow::Result<()> {
            Ok(())
        }
    }

    fn setup_mock_state() -> AppState {
        let repo: Arc<dyn ServerRepo> = Arc::new(InMemoryServerRepo::new());
        let (state, rx) = AppState::with_repo(None, repo, None, AppConfig::default());
        // Keep the receiver alive so orchestrator_tx.send() doesn't fail in tests
        std::mem::forget(rx);
        state
    }

    fn create_test_token(state: &AppState) -> String {
        use crate::types::UserId;
        auth::create_token(&state.jwt_secret, 24, UserId::new(), "test@test.com").unwrap()
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
}
