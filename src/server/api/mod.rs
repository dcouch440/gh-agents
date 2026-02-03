//! REST API endpoint handlers

pub mod agent_context;
pub mod agent_executions;
pub mod agents;
pub mod auth;
pub mod cancellation;
pub mod chat;
pub mod config;
pub mod costs;
pub mod documents;
pub mod health;
pub mod output_schemas;
pub mod prompt_templates;
pub mod results;
pub mod rooms;
pub mod session_context;
pub mod sessions;
pub mod tasks;
pub mod tool_routers;
pub mod tools;
pub mod workflows;

// Re-export auth handlers and types
pub use auth::{auth_login, auth_me, auth_register, auth_setup, AuthTokenResponse, LoginRequest, LoginResponse, MeResponse, RegisterRequest, SetupRequest, SetupResponse, UserResponse};

// Re-export task handlers and types
pub use tasks::{create_task, get_task, list_tasks, CreateTaskRequest, TasksQuery};

// Re-export agent handlers and types
pub use agents::{create_agent, delete_agent, get_agent, list_agents, update_agent, AgentPoolStats, AgentResponse, AgentsListResponse, CreateAgentRequest, UpdateAgentRequest};

// Re-export tool handlers and types
pub use tools::{
    create_tool, delete_tool, get_agent_tools, get_tool, list_tools, set_agent_tools, update_tool, AgentToolsResponse, CreateToolRequest, SetAgentToolsRequest, ToolResponse, UpdateToolRequest,
};

// Re-export config handlers and types
pub use config::{get_config, update_config, ConfigResponse, UpdateConfigRequest, UpdatePoolRequest};

// Re-export agent context handlers and types
pub use agent_context::{get_agent_context, set_agent_context, AgentContextResponse, SetAgentContextRequest};

// Re-export chat handlers and types
pub use chat::{chat_stream, clear_chat_history, get_chat_history, send_chat, session_chat_stream, ChatMessage, ChatRequest, ChatResponse, HistoryQuery};

// Re-export document handlers and types
pub use documents::{
    create_document, delete_document, get_document, list_documents, search_documents, update_document, CreateDocumentRequest, DocumentListItem, DocumentResponse, DocumentSearchQuery,
    UpdateDocumentRequest,
};

// Re-export session handlers and types
pub use sessions::{
    create_agent_mode, create_session, delete_agent_mode, delete_session, get_session, get_session_history, list_agent_modes, list_modes, list_sessions, send_session_chat, update_session,
    AgentModeResponse, CreateAgentModeRequest, CreateSessionRequest, ModeInfo, SessionResponse, UpdateSessionRequest,
};

// Re-export output schema handlers and types
pub use output_schemas::{
    create_output_schema, delete_output_schema, get_output_schema, list_output_schemas, update_output_schema, CreateOutputSchemaRequest, OutputSchemaResponse, UpdateOutputSchemaRequest,
};

// Re-export prompt template handlers and types
pub use prompt_templates::{
    create_prompt_template, delete_prompt_template, get_prompt_template, list_prompt_templates, update_prompt_template, CreatePromptTemplateRequest, PromptTemplateResponse,
    UpdatePromptTemplateRequest,
};

// Re-export agent execution handlers and types
pub use agent_executions::{
    approve_execution, get_agent_execution, list_execution_messages, send_execution_message, AgentExecutionResponse, ApproveExecutionRequest, ExecutionMessageResponse, SendMessageRequest,
};

// Re-export cost handlers and types
pub use costs::{get_costs, CostQuery, CostResponse};

// Re-export results handlers and types
pub use results::{delete_result, get_result, list_results, ResultQuery, ResultResponse};

// Re-export workflow handlers and types
pub use workflows::{
    add_step_document, add_workflow_edge, create_workflow, create_workflow_step, delete_workflow, delete_workflow_step, get_workflow, get_workflow_step, list_step_documents, list_workflow_edges,
    list_workflow_steps, list_workflows, remove_step_document, remove_workflow_edge, update_workflow, update_workflow_step, CreateStepRequest, CreateWorkflowRequest, EdgeRequest, EdgeResponse,
    StepDocumentRequest, StepDocumentResponse, UpdateStepRequest, UpdateWorkflowRequest, WorkflowResponse, WorkflowStepResponse,
};

// Re-export tool router handlers and types
pub use tool_routers::{
    create_tool_router, delete_tool_router, get_router_tools, get_tool_router, list_tool_routers, set_router_tools, update_tool_router, CreateToolRouterRequest, SetRouterToolsRequest,
    UpdateToolRouterRequest,
};

// Re-export session context handlers
pub use session_context::{get_session_context, list_session_requests};

// Re-export room handlers and types
pub use rooms::{
    add_room_member, close_room_session, create_room, create_room_session, delete_room, get_room, get_room_session,
    get_room_transcript, list_room_members, remove_room_member, send_room_message, set_room_members, update_room,
    AddRoomMemberRequest, CreateRoomRequest, RoomMessageRequest, SetRoomMembersRequest, UpdateRoomRequest,
};

// Re-export health handler and type
pub use health::{health_check, HealthResponse};

// Re-export cancellation handler
pub use cancellation::cancel_agent_execution;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::auth as auth_utils;
use super::state::AppState;
use crate::constants::{MAX_DESCRIPTION_LENGTH, MAX_PROMPT_LENGTH, MAX_TITLE_LENGTH};

#[cfg(test)]
use crate::types::{AgentPoolConfig, Task};
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_task_request_deserializes() {
        let json = r#"{"title": "Test task", "priority": "high"}"#;
        let request: CreateTaskRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.title, "Test task");
        assert_eq!(request.priority, Some("high".to_string()));
    }

    #[test]
    fn health_response_serializes() {
        let response = HealthResponse {
            status: "ok".to_string(),
            version: "1.0.0".to_string(),
            db_connected: true,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"status\":\"ok\""));
        assert!(json.contains("\"db_connected\":true"));
    }

    #[test]
    fn config_response_serializes() {
        let response = ConfigResponse {
            verbosity: "normal".to_string(),
            pool: AgentPoolConfig::default(),
            autonomy: "approval_gates".to_string(),
            git_strategy: "branch_per_slice".to_string(),
            sandbox_mode: "docker".to_string(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"verbosity\":\"normal\""));
    }

    #[test]
    fn tasks_query_deserializes() {
        let json = r#"{"status": "pending", "limit": 10}"#;
        let query: TasksQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.status, Some("pending".to_string()));
        assert_eq!(query.limit, Some(10));
    }

    #[test]
    fn agent_pool_stats_serializes() {
        let stats = AgentPoolStats { total: 6, available: 5, max: 12 };
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("\"total\""));
        assert!(json.contains("\"available\""));
        assert!(json.contains("\"max\""));
    }

    // Chat endpoint tests

    #[test]
    fn chat_request_deserializes() {
        let json = r#"{"message": "Hello, world!"}"#;
        let request: ChatRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.message, "Hello, world!");
    }

    #[test]
    fn chat_response_serializes() {
        let response = ChatResponse {
            message_id: Uuid::new_v4(),
            status: "queued".to_string(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"message_id\""));
        assert!(json.contains("\"status\":\"queued\""));
    }

    #[test]
    fn history_query_deserializes() {
        let json = r#"{"limit": 25, "offset": 10}"#;
        let query: HistoryQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.limit, Some(25));
        assert_eq!(query.offset, Some(10));
    }

    #[test]
    fn history_query_with_defaults() {
        let json = r#"{}"#;
        let query: HistoryQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.limit, None);
        assert_eq!(query.offset, None);
    }

    #[test]
    fn chat_message_serializes() {
        let message = ChatMessage {
            id: Uuid::new_v4(),
            role: "user".to_string(),
            content: "Hello!".to_string(),
            timestamp: Utc::now(),
        };
        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"content\":\"Hello!\""));
    }

    // === Integration tests using setup_test_app (mock-based, no Postgres) ===

    use axum::body::Body;
    use axum::http::Request;
    use tower::util::ServiceExt;

    use std::sync::Arc;

    use crate::db::traits::{MockUserRepo, ServerRepo};
    use crate::db::{ChatMessageRow, PipelineRow, PipelineStageRow};
    use crate::types::{User, UserId};

    /// In-memory implementation of ServerRepo for tests (no Postgres needed).
    struct InMemoryServerRepo {
        tasks: std::sync::Mutex<Vec<Task>>,
        chat_messages: std::sync::Mutex<Vec<ChatMessageRow>>,
        password_hash: std::sync::Mutex<Option<String>>,
        agents: std::sync::Mutex<Vec<crate::db::AgentRow>>,
        tools: std::sync::Mutex<Vec<crate::db::ToolRow>>,
        agent_tools: std::sync::Mutex<Vec<(Uuid, Uuid)>>,
    }

    impl InMemoryServerRepo {
        fn new() -> Self {
            Self {
                tasks: std::sync::Mutex::new(vec![]),
                chat_messages: std::sync::Mutex::new(vec![]),
                password_hash: std::sync::Mutex::new(None),
                agents: std::sync::Mutex::new(vec![]),
                tools: std::sync::Mutex::new(vec![]),
                agent_tools: std::sync::Mutex::new(vec![]),
            }
        }
    }

    #[async_trait::async_trait]
    impl ServerRepo for InMemoryServerRepo {
        async fn health_check(&self) -> bool {
            true
        }

        async fn list_tasks(&self, _user_id: UserId, status: Option<String>, limit: Option<u32>) -> anyhow::Result<Vec<Task>> {
            let tasks = self.tasks.lock().unwrap();
            let limit = limit.unwrap_or(crate::constants::DEFAULT_QUERY_LIMIT as u32).min(crate::constants::MAX_QUERY_LIMIT as u32) as usize;
            let filtered: Vec<Task> = tasks
                .iter()
                .filter(|t| {
                    if let Some(ref s) = status {
                        let ts = format!("{:?}", t.status).to_lowercase();
                        &ts == s
                    } else {
                        true
                    }
                })
                .rev()
                .take(limit)
                .cloned()
                .collect();
            Ok(filtered)
        }

        async fn get_task_by_uuid(&self, _user_id: UserId, id: Uuid) -> anyhow::Result<Option<Task>> {
            let tasks = self.tasks.lock().unwrap();
            Ok(tasks.iter().find(|t| t.id.0 == id).cloned())
        }

        async fn insert_task(&self, _user_id: UserId, task: Task) -> anyhow::Result<()> {
            self.tasks.lock().unwrap().push(task);
            Ok(())
        }

        async fn insert_chat_message(&self, _user_id: UserId, id: Uuid, role: String, content: String) -> anyhow::Result<()> {
            self.chat_messages.lock().unwrap().push(ChatMessageRow {
                id,
                role,
                content,
                timestamp: Utc::now(),
            });
            Ok(())
        }

        async fn get_chat_history(&self, _user_id: UserId, limit: u32, offset: u32) -> anyhow::Result<Vec<ChatMessageRow>> {
            let msgs = self.chat_messages.lock().unwrap();
            let result: Vec<ChatMessageRow> = msgs.iter().skip(offset as usize).take(limit.min(1000) as usize).cloned().collect();
            Ok(result)
        }

        async fn clear_chat_history(&self, _user_id: UserId) -> anyhow::Result<()> {
            self.chat_messages.lock().unwrap().clear();
            Ok(())
        }

        async fn has_password(&self) -> anyhow::Result<bool> {
            Ok(self.password_hash.lock().unwrap().is_some())
        }

        async fn set_password(&self, password_hash: String) -> anyhow::Result<()> {
            *self.password_hash.lock().unwrap() = Some(password_hash);
            Ok(())
        }

        async fn get_password(&self) -> anyhow::Result<Option<String>> {
            Ok(self.password_hash.lock().unwrap().clone())
        }
        async fn list_persisted_agents(&self, _user_id: UserId) -> anyhow::Result<Vec<crate::db::AgentRow>> {
            Ok(self.agents.lock().unwrap().clone())
        }
        async fn get_persisted_agent(&self, agent_id: Uuid) -> anyhow::Result<Option<crate::db::AgentRow>> {
            Ok(self.agents.lock().unwrap().iter().find(|a| a.id == agent_id).cloned())
        }
        async fn upsert_agent(&self, _user_id: UserId, agent: crate::db::AgentRow) -> anyhow::Result<()> {
            let mut agents = self.agents.lock().unwrap();
            if let Some(existing) = agents.iter_mut().find(|a| a.id == agent.id) {
                *existing = agent;
            } else {
                agents.push(agent);
            }
            Ok(())
        }
        async fn delete_persisted_agent(&self, agent_id: Uuid) -> anyhow::Result<()> {
            self.agents.lock().unwrap().retain(|a| a.id != agent_id);
            Ok(())
        }
        async fn list_tools(&self, _user_id: UserId) -> anyhow::Result<Vec<crate::db::ToolRow>> {
            Ok(self.tools.lock().unwrap().clone())
        }
        async fn get_tool(&self, tool_id: Uuid) -> anyhow::Result<Option<crate::db::ToolRow>> {
            Ok(self.tools.lock().unwrap().iter().find(|t| t.id == tool_id).cloned())
        }
        async fn upsert_tool(&self, _user_id: UserId, tool: crate::db::ToolRow) -> anyhow::Result<()> {
            let mut tools = self.tools.lock().unwrap();
            if let Some(existing) = tools.iter_mut().find(|t| t.id == tool.id) {
                *existing = tool;
            } else {
                tools.push(tool);
            }
            Ok(())
        }
        async fn delete_tool(&self, tool_id: Uuid) -> anyhow::Result<()> {
            self.tools.lock().unwrap().retain(|t| t.id != tool_id);
            self.agent_tools.lock().unwrap().retain(|(_, tid)| *tid != tool_id);
            Ok(())
        }
        async fn get_agent_tools(&self, agent_id: Uuid) -> anyhow::Result<Vec<crate::db::ToolRow>> {
            let at = self.agent_tools.lock().unwrap();
            let tool_ids: Vec<Uuid> = at.iter().filter(|(aid, _)| *aid == agent_id).map(|(_, tid)| *tid).collect();
            let tools = self.tools.lock().unwrap();
            Ok(tools.iter().filter(|t| tool_ids.contains(&t.id)).cloned().collect())
        }
        async fn set_agent_tools(&self, agent_id: Uuid, tool_ids: Vec<Uuid>) -> anyhow::Result<()> {
            let mut at = self.agent_tools.lock().unwrap();
            at.retain(|(aid, _)| *aid != agent_id);
            for tid in tool_ids {
                at.push((agent_id, tid));
            }
            Ok(())
        }
        async fn seed_builtin_tools(&self, _user_id: UserId) -> anyhow::Result<()> {
            Ok(())
        }
        async fn get_agent_context(&self, _agent_id: Uuid) -> anyhow::Result<Vec<crate::db::DocumentRow>> {
            Ok(vec![])
        }
        async fn set_agent_context(&self, _agent_id: Uuid, _document_ids: Vec<Uuid>) -> anyhow::Result<()> {
            Ok(())
        }
        async fn list_pipelines(&self, _user_id: UserId) -> anyhow::Result<Vec<PipelineRow>> {
            Ok(vec![])
        }
        async fn upsert_pipeline(&self, _user_id: UserId, _pipeline: PipelineRow) -> anyhow::Result<()> {
            Ok(())
        }
        async fn delete_pipeline(&self, _pipeline_id: Uuid) -> anyhow::Result<()> {
            Ok(())
        }
        async fn list_pipeline_stages(&self, _pipeline_id: Uuid) -> anyhow::Result<Vec<PipelineStageRow>> {
            Ok(vec![])
        }
        async fn upsert_pipeline_stage(&self, _stage: PipelineStageRow) -> anyhow::Result<()> {
            Ok(())
        }
        async fn create_session(&self, _user_id: UserId, _session_id: Uuid, _mode_id: &str, _title: &str, _agent_id: Option<Uuid>) -> anyhow::Result<()> {
            Ok(())
        }
        async fn list_sessions(&self, _user_id: UserId) -> anyhow::Result<Vec<crate::db::SessionRow>> {
            Ok(vec![])
        }
        async fn get_session(&self, _session_id: Uuid) -> anyhow::Result<Option<crate::db::SessionRow>> {
            Ok(None)
        }
        async fn delete_session(&self, _session_id: Uuid) -> anyhow::Result<()> {
            Ok(())
        }
        async fn insert_session_message(&self, _user_id: UserId, _session_id: Uuid, _id: Uuid, _role: String, _content: String) -> anyhow::Result<()> {
            Ok(())
        }
        async fn get_session_history(&self, _session_id: Uuid, _limit: u32) -> anyhow::Result<Vec<ChatMessageRow>> {
            Ok(vec![])
        }
        async fn update_session_title(&self, _session_id: Uuid, _title: &str) -> anyhow::Result<()> {
            Ok(())
        }
        async fn update_session_summary(&self, _session_id: Uuid, _summary: &str) -> anyhow::Result<()> {
            Ok(())
        }
        async fn count_session_messages(&self, _session_id: Uuid) -> anyhow::Result<u32> {
            Ok(0)
        }
        async fn create_pipeline_run(&self, _run: &crate::db::PipelineRunRow) -> anyhow::Result<()> {
            Ok(())
        }
        async fn update_pipeline_run(&self, _run: &crate::db::PipelineRunRow) -> anyhow::Result<()> {
            Ok(())
        }
        async fn get_pipeline_run(&self, _run_id: Uuid) -> anyhow::Result<Option<crate::db::PipelineRunRow>> {
            Ok(None)
        }
        async fn list_pipeline_runs(&self, _pipeline_id: Uuid) -> anyhow::Result<Vec<crate::db::PipelineRunRow>> {
            Ok(vec![])
        }
        async fn create_stage_execution(&self, _exec: &crate::db::StageExecutionRow) -> anyhow::Result<()> {
            Ok(())
        }
        async fn update_stage_execution(&self, _exec: &crate::db::StageExecutionRow) -> anyhow::Result<()> {
            Ok(())
        }
        async fn list_stage_executions(&self, _run_id: Uuid) -> anyhow::Result<Vec<crate::db::StageExecutionRow>> {
            Ok(vec![])
        }
        async fn get_agent_modes(&self, _agent_id: Uuid) -> anyhow::Result<Vec<crate::db::AgentModeRow>> {
            Ok(vec![])
        }
        async fn create_agent_mode(&self, _mode: &crate::db::AgentModeRow) -> anyhow::Result<()> {
            Ok(())
        }
        async fn delete_agent_mode(&self, _mode_id: Uuid) -> anyhow::Result<()> {
            Ok(())
        }
    }

    fn create_test_token(jwt_secret: &[u8]) -> String {
        super::super::auth::create_token(jwt_secret, 24, UserId::new(), "test@test.com").unwrap()
    }

    fn setup_test_app() -> (axum::Router, Vec<u8>) {
        setup_test_app_with_user_repo(None)
    }

    fn setup_test_app_with_user_repo(user_repo: Option<Arc<dyn crate::db::traits::UserRepo>>) -> (axum::Router, Vec<u8>) {
        let repo: Arc<dyn ServerRepo> = Arc::new(InMemoryServerRepo::new());
        let config = crate::types::AppConfig::default();
        let (mut state, rx) = AppState::with_repo(None, repo, config);
        // Keep the receiver alive so chat_tx.send() doesn't fail
        std::mem::forget(rx);
        if let Some(ur) = user_repo {
            state.user_repo = Some(ur);
        }
        let jwt_secret = state.jwt_secret.clone();
        let router = super::super::create_router_with_static_dir(state, "nonexistent_static");
        (router, jwt_secret)
    }

    #[tokio::test]
    async fn create_task_valid_returns_created() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"title":"My task","description":"desc","priority":"high"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn create_task_empty_title_returns_bad_request() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"title":"   "}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn update_config_valid_verbosity() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/config")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"verbosity":"verbose"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn send_chat_valid_message_returns_accepted() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/chat")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"message":"Hello agent"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::ACCEPTED);
    }

    #[tokio::test]
    async fn send_chat_empty_message_returns_bad_request() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/chat")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"message":"  "}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn clear_chat_history_returns_no_content() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/chat/history")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn health_check_returns_ok() {
        let (app, _jwt_secret) = setup_test_app();

        let response = app.oneshot(Request::builder().uri("/api/health").body(Body::empty()).unwrap()).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("\"status\":\"ok\""));
        assert!(body_str.contains("\"db_connected\":true"));
    }

    // === Priority parsing tests ===

    #[tokio::test]
    async fn create_task_with_low_priority() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"title":"Low prio","priority":"low"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("\"priority\":\"low\""));
    }

    #[tokio::test]
    async fn create_task_with_urgent_priority() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"title":"Urgent","priority":"urgent"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("\"priority\":\"urgent\""));
    }

    #[tokio::test]
    async fn create_task_with_unknown_priority_defaults_to_normal() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"title":"Default prio","priority":"critical"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("\"priority\":\"normal\""));
    }

    #[tokio::test]
    async fn create_task_with_no_priority_defaults_to_normal() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"title":"No prio"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("\"priority\":\"normal\""));
    }

    // === get_task: found and not found ===

    #[tokio::test]
    async fn get_task_returns_created_task() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        // Create a task first
        let create_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"title":"Findable task"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_resp.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(create_resp.into_body(), usize::MAX).await.unwrap();
        let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let task_id = created["id"].as_str().unwrap();

        // Verify through list endpoint that the task was persisted
        let list_resp = app
            .oneshot(Request::builder().uri("/api/tasks").header("authorization", format!("Bearer {}", token)).body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(list_resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(list_resp.into_body(), usize::MAX).await.unwrap();
        let tasks: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        let found = tasks.iter().find(|t| t["id"].as_str() == Some(task_id));
        assert!(found.is_some(), "Created task should appear in task list");
        assert_eq!(found.unwrap()["title"].as_str().unwrap(), "Findable task");
    }

    #[tokio::test]
    async fn get_task_not_found() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/tasks/00000000-0000-0000-0000-000000000000")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Note: This may return 404 from the handler OR from the static fallback.
        // Both are acceptable for a non-existent task.
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    // === list_tasks with filters ===

    #[tokio::test]
    async fn list_tasks_returns_empty_initially() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(Request::builder().uri("/api/tasks").header("authorization", format!("Bearer {}", token)).body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let tasks: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert!(tasks.is_empty());
    }

    #[tokio::test]
    async fn list_tasks_with_limit() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        // Create two tasks
        for title in ["Task A", "Task B"] {
            app.clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/tasks")
                        .header("content-type", "application/json")
                        .header("authorization", format!("Bearer {}", token))
                        .body(Body::from(format!(r#"{{"title":"{}"}}"#, title)))
                        .unwrap(),
                )
                .await
                .unwrap();
        }

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/tasks?limit=1")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let tasks: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(tasks.len(), 1);
    }

    #[tokio::test]
    async fn list_tasks_with_status_filter() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        // Create a task (default status is pending)
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"title":"Pending task"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Filter for in_progress - should return nothing
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/tasks?status=in_progress")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let tasks: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert!(tasks.is_empty());
    }

    // === update_config invalid verbosity ===

    #[tokio::test]
    async fn update_config_invalid_verbosity_returns_bad_request() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/config")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"verbosity":"extreme"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn update_config_quiet_verbosity() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/config")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"verbosity":"quiet"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn update_config_normal_verbosity() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/config")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"verbosity":"normal"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn update_config_no_verbosity_returns_ok() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/config")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    // === list_agents response body ===

    #[tokio::test]
    async fn list_agents_returns_stats() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(Request::builder().uri("/api/agents").header("authorization", format!("Bearer {}", token)).body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(resp["agents"].is_array());
        assert!(resp["stats"]["total"].is_number());
        assert!(resp["stats"]["available"].is_number());
        assert!(resp["stats"]["max"].is_number());
    }

    // === Agent CRUD tests ===

    #[tokio::test]
    async fn create_agent_returns_created() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/agents")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"tier":"worker","persona_name":"Builder","model_id":"claude-sonnet-4-20250514"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let agent: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(agent["tier"].as_str().unwrap(), "worker");
        assert_eq!(agent["persona_name"].as_str().unwrap(), "Builder");
        assert_eq!(agent["persona_style"].as_str().unwrap(), "casual");
        assert_eq!(agent["model_provider"].as_str().unwrap(), "anthropic");
        assert_eq!(agent["model_max_tokens"].as_i64().unwrap(), 4096);
        assert_eq!(agent["status"].as_str().unwrap(), "idle");
    }

    #[tokio::test]
    async fn create_agent_empty_tier_returns_bad_request() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/agents")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"tier":"","persona_name":"X","model_id":"claude-sonnet-4-20250514"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn list_agents_includes_created_agent() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        // Create an agent
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/agents")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"tier":"orchestrator","persona_name":"Planner","model_id":"claude-sonnet-4-20250514"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        // List agents
        let response = app
            .oneshot(Request::builder().uri("/api/agents").header("authorization", format!("Bearer {}", token)).body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let agents = resp["agents"].as_array().unwrap();
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0]["persona_name"].as_str().unwrap(), "Planner");
    }

    #[tokio::test]
    async fn get_agent_returns_created_agent() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        // Create
        let create_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/agents")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"tier":"utility","persona_name":"Helper","model_id":"claude-haiku-35-20241022"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = axum::body::to_bytes(create_resp.into_body(), usize::MAX).await.unwrap();
        let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let agent_id = created["id"].as_str().unwrap();

        // Get by ID
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/agents/{}", agent_id))
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let agent: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(agent["persona_name"].as_str().unwrap(), "Helper");
        assert_eq!(agent["tier"].as_str().unwrap(), "utility");
    }

    #[tokio::test]
    async fn get_agent_not_found() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/agents/00000000-0000-0000-0000-000000000000")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn update_agent_partial_fields() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        // Create
        let create_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/agents")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"tier":"worker","persona_name":"OldName","model_id":"claude-sonnet-4-20250514"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = axum::body::to_bytes(create_resp.into_body(), usize::MAX).await.unwrap();
        let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let agent_id = created["id"].as_str().unwrap();

        // Update only persona_name
        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/api/agents/{}", agent_id))
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"persona_name":"NewName"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let agent: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(agent["persona_name"].as_str().unwrap(), "NewName");
        assert_eq!(agent["tier"].as_str().unwrap(), "worker"); // unchanged
    }

    #[tokio::test]
    async fn delete_agent_returns_no_content() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        // Create
        let create_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/agents")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"tier":"worker","persona_name":"ToDelete","model_id":"claude-sonnet-4-20250514"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = axum::body::to_bytes(create_resp.into_body(), usize::MAX).await.unwrap();
        let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let agent_id = created["id"].as_str().unwrap();

        // Delete
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/agents/{}", agent_id))
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        // Verify gone
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/agents/{}", agent_id))
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    // === Tool CRUD tests ===

    #[tokio::test]
    async fn create_tool_returns_created() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tools")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(
                        r#"{"name":"read_file","display_name":"Read File","description":"Read a file","parameters":{"type":"object"}}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let tool: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(tool["name"], "read_file");
        assert_eq!(tool["display_name"], "Read File");
    }

    #[tokio::test]
    async fn create_tool_empty_name_returns_bad_request() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tools")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"name":"  "}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn list_tools_includes_created() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        // Create
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tools")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"name":"git_status","description":"Show git status","category":"git"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        // List
        let response = app
            .oneshot(Request::builder().uri("/api/tools").header("authorization", format!("Bearer {}", token)).body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let tools: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"], "git_status");
    }

    #[tokio::test]
    async fn get_tool_by_id() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let create_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tools")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"name":"run_tests","description":"Run tests"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = axum::body::to_bytes(create_resp.into_body(), usize::MAX).await.unwrap();
        let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let tool_id = created["id"].as_str().unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/tools/{}", tool_id))
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let tool: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(tool["name"], "run_tests");
    }

    #[tokio::test]
    async fn get_tool_not_found() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/tools/{}", Uuid::new_v4()))
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn update_tool_partial() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let create_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tools")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"name":"write_file","description":"Write a file"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = axum::body::to_bytes(create_resp.into_body(), usize::MAX).await.unwrap();
        let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let tool_id = created["id"].as_str().unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/api/tools/{}", tool_id))
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"description":"Write content to a file"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let tool: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(tool["description"], "Write content to a file");
        assert_eq!(tool["name"], "write_file");
    }

    #[tokio::test]
    async fn delete_tool_returns_no_content() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let create_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tools")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"name":"to_delete","description":"temp"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = axum::body::to_bytes(create_resp.into_body(), usize::MAX).await.unwrap();
        let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let tool_id = created["id"].as_str().unwrap();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/tools/{}", tool_id))
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        // Verify gone
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/tools/{}", tool_id))
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn set_and_get_agent_tools() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        // Create an agent
        let agent_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/agents")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"tier":"worker","persona_name":"ToolUser","model_id":"claude-sonnet-4-20250514"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = axum::body::to_bytes(agent_resp.into_body(), usize::MAX).await.unwrap();
        let agent: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let agent_id = agent["id"].as_str().unwrap();

        // Create two tools
        let tool1_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tools")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"name":"read_file","description":"Read"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(tool1_resp.into_body(), usize::MAX).await.unwrap();
        let tool1: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let tool1_id = tool1["id"].as_str().unwrap();

        let tool2_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tools")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"name":"write_file","description":"Write"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(tool2_resp.into_body(), usize::MAX).await.unwrap();
        let tool2: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let tool2_id = tool2["id"].as_str().unwrap();

        // Set agent tools
        let set_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/agents/{}/tools", agent_id))
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(serde_json::json!({"tool_ids": [tool1_id, tool2_id]}).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(set_resp.status(), StatusCode::OK);

        // Get agent tools
        let get_resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/agents/{}/tools", agent_id))
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(get_resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(get_resp.into_body(), usize::MAX).await.unwrap();
        let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(result["agent_id"], agent_id);
        assert_eq!(result["tools"].as_array().unwrap().len(), 2);
    }

    // === get_config response body ===

    #[tokio::test]
    async fn get_config_returns_expected_fields() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(Request::builder().uri("/api/config").header("authorization", format!("Bearer {}", token)).body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(resp["verbosity"].is_string());
        assert!(resp["pool"].is_object());
        assert!(resp["autonomy"].is_string());
        assert!(resp["git_strategy"].is_string());
        assert!(resp["sandbox_mode"].is_string());
    }

    // === Auth endpoints ===

    #[tokio::test]
    async fn auth_setup_short_password_returns_bad_request() {
        let (app, _jwt_secret) = setup_test_app();

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

    #[tokio::test]
    async fn auth_setup_success() {
        let (app, _jwt_secret) = setup_test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/setup")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"password":"longpassword123"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(resp["message"].as_str().unwrap(), "Password configured successfully");
    }

    #[tokio::test]
    async fn auth_setup_conflict_when_already_configured() {
        let (app, _jwt_secret) = setup_test_app();

        // First setup
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/setup")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"password":"longpassword123"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Second setup should conflict
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/setup")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"password":"anotherpassword"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    fn setup_login_test_app(password: &str) -> (axum::Router, Vec<u8>) {
        let password_hash = super::super::auth::hash_password(password).unwrap();
        let test_user = User {
            id: UserId::new(),
            email: "test@test.com".to_string(),
            password_hash: Some(password_hash),
            github_id: None,
            github_login: None,
            github_token_encrypted: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let mut mock = MockUserRepo::new();
        let user_clone = test_user.clone();
        mock.expect_get_user_by_email()
            .returning(move |email| if email == "test@test.com" { Ok(Some(user_clone.clone())) } else { Ok(None) });
        setup_test_app_with_user_repo(Some(Arc::new(mock)))
    }

    #[tokio::test]
    async fn auth_login_no_password_configured() {
        // No user_repo means login returns 500 (user service unavailable)
        let (app, _jwt_secret) = setup_test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"email":"test@test.com","password":"anything"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        // No user_repo configured, so we get 500
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn auth_login_wrong_password() {
        let (app, _jwt_secret) = setup_login_test_app("correctpassword");

        // Login with wrong password
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"email":"test@test.com","password":"wrongpassword!"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn auth_login_success() {
        let (app, _jwt_secret) = setup_login_test_app("correctpassword");

        // Login with correct password
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"email":"test@test.com","password":"correctpassword"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(resp["token"].is_string());
        assert_eq!(resp["expires_in"].as_u64().unwrap(), 86400);
    }

    // === Chat history with data ===

    #[tokio::test]
    async fn chat_history_returns_messages_after_send() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        // Send a message
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/chat")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"message":"Hello agent"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Get history
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/chat/history")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let messages: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0]["role"].as_str().unwrap(), "user");
        assert_eq!(messages[0]["content"].as_str().unwrap(), "Hello agent");
    }

    #[tokio::test]
    async fn chat_history_with_pagination() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        // Send two messages
        for msg in ["First", "Second"] {
            app.clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/chat")
                        .header("content-type", "application/json")
                        .header("authorization", format!("Bearer {}", token))
                        .body(Body::from(format!(r#"{{"message":"{}"}}"#, msg)))
                        .unwrap(),
                )
                .await
                .unwrap();
        }

        // Get with limit=1
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/chat/history?limit=1")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let messages: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(messages.len(), 1);

        // Get with offset=1
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/chat/history?limit=10&offset=1")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let messages: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(messages.len(), 1);
    }

    // === Serialization edge cases ===

    #[test]
    fn create_task_request_all_fields() {
        let json = r#"{"title":"T","description":"D","priority":"low","tier":"orchestrator"}"#;
        let request: CreateTaskRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.title, "T");
        assert_eq!(request.description, Some("D".to_string()));
        assert_eq!(request.priority, Some("low".to_string()));
        assert_eq!(request.tier, Some("orchestrator".to_string()));
    }

    #[test]
    fn create_task_request_minimal() {
        let json = r#"{"title":"T"}"#;
        let request: CreateTaskRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.title, "T");
        assert!(request.description.is_none());
        assert!(request.priority.is_none());
        assert!(request.tier.is_none());
    }

    #[test]
    fn tasks_query_with_no_fields() {
        let json = r#"{}"#;
        let query: TasksQuery = serde_json::from_str(json).unwrap();
        assert!(query.status.is_none());
        assert!(query.limit.is_none());
    }

    #[test]
    fn setup_request_deserializes() {
        let json = r#"{"password":"mypassword"}"#;
        let request: SetupRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.password, "mypassword");
    }

    #[test]
    fn login_request_deserializes() {
        let json = r#"{"email":"test@test.com","password":"mypassword"}"#;
        let request: LoginRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.email, "test@test.com");
        assert_eq!(request.password, "mypassword");
    }

    #[test]
    fn setup_response_serializes() {
        let response = SetupResponse { message: "ok".to_string() };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"message\":\"ok\""));
    }

    #[test]
    fn login_response_serializes() {
        let response = LoginResponse {
            token: "abc123".to_string(),
            expires_in: 86400,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"token\":\"abc123\""));
        assert!(json.contains("\"expires_in\":86400"));
    }

    #[test]
    fn me_response_serializes() {
        let response = MeResponse {
            id: "user-123".to_string(),
            email: "admin@example.com".to_string(),
            github_login: None,
            authenticated: true,
            token_expires: 99999,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"id\":\"user-123\""));
        assert!(json.contains("\"authenticated\":true"));
        assert!(json.contains("\"token_expires\":99999"));
    }

    fn test_agent_response() -> AgentResponse {
        AgentResponse {
            id: "agent-1".to_string(),
            name: "Test Agent".to_string(),
            system_prompt: "You are a test agent".to_string(),
            persona_style: "casual".to_string(),
            model_provider: "anthropic".to_string(),
            model_id: "claude-sonnet-4-20250514".to_string(),
            model_max_tokens: 4096,
            model_temperature: 0.7,
            status: "idle".to_string(),
            version: 1,
        }
    }

    #[test]
    fn agents_list_response_serializes() {
        let response = AgentsListResponse {
            agents: vec![test_agent_response()],
            stats: AgentPoolStats { total: 1, available: 1, max: 12 },
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"agent-1\""));
        assert!(json.contains("\"total\""));
    }

    #[test]
    fn agent_response_serializes_all_fields() {
        let response = test_agent_response();
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"persona_name\":\"Test Agent\""));
        assert!(json.contains("\"model_provider\":\"anthropic\""));
        assert!(json.contains("\"model_max_tokens\":4096"));
    }

    // === send_chat response body ===

    #[tokio::test]
    async fn send_chat_response_contains_message_id() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/chat")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"message":"test msg"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::ACCEPTED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(resp["message_id"].is_string());
        assert_eq!(resp["status"].as_str().unwrap(), "queued");
        // Verify it's a valid UUID
        Uuid::parse_str(resp["message_id"].as_str().unwrap()).unwrap();
    }

    // === create_task response body validation ===

    #[tokio::test]
    async fn create_task_response_body_has_expected_fields() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"title":"Full task","description":"A description","priority":"high","tier":"worker"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let task: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(task["title"].as_str().unwrap(), "Full task");
        assert_eq!(task["description"].as_str().unwrap(), "A description");
        assert!(task["id"].is_string());
        assert!(task["created_at"].is_string());
        assert!(task["updated_at"].is_string());
    }

    // === clear chat then verify empty ===

    #[tokio::test]
    async fn clear_chat_then_history_is_empty() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        // Send a message
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/chat")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"message":"To be cleared"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Clear history
        app.clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/chat/history")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Verify empty
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/chat/history")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let messages: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert!(messages.is_empty());
    }
}
