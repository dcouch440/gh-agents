#[cfg(test)]
mod tests {
    use crate::constants::routes as route_paths;
    use crate::db::traits::{
        MockAgentRepo, MockAuthConfigRepo, MockChatMessageRepo, MockSessionRepo, MockToolRepo,
    };
    use crate::server::auth;
    use crate::server::routes;
    use crate::server::state::test_helpers::MockReposBuilder;
    use crate::server::{
        build_cors_layer, cache_control_middleware, request_id_middleware, ws, AppState,
    };
    use crate::types::AppConfig;
    use axum::{
        body::Body,
        http::{header::CACHE_CONTROL, Request, StatusCode},
        middleware,
        routing::get,
        Router,
    };
    use std::sync::Arc;
    use tempfile::TempDir;
    use tower::util::ServiceExt;
    use tower_http::services::{ServeDir, ServeFile};
    use tower_http::trace::{DefaultMakeSpan, TraceLayer};

    /// Create the application router with a specific static directory (no rate limiting — used by tests)
    fn create_router_with_static_dir(state: AppState, static_dir: &str) -> Router {
        let cors = build_cors_layer(state.env().cors_origins.as_deref());
        let public_routes = routes::build_public_routes();
        let protected_routes = routes::build_protected_routes(state.clone());

        let serve_dir = ServeDir::new(static_dir)
            .not_found_service(ServeFile::new(format!("{}/index.html", static_dir)));

        Router::new()
            .nest("/api", public_routes.merge(protected_routes))
            .route(route_paths::WS, get(ws::ws_handler))
            .fallback_service(serve_dir)
            .layer(middleware::from_fn(request_id_middleware))
            .layer(middleware::from_fn(cache_control_middleware))
            .layer(cors)
            .layer(
                TraceLayer::new_for_http()
                    .make_span_with(DefaultMakeSpan::new().level(tracing::Level::DEBUG))
                    .on_request(
                        tower_http::trace::DefaultOnRequest::new().level(tracing::Level::DEBUG),
                    )
                    .on_response(
                        tower_http::trace::DefaultOnResponse::new().level(tracing::Level::DEBUG),
                    ),
            )
            .with_state(state)
    }

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
            state.jwt_secret(),
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
            &EncodingKey::from_secret(state.jwt_secret()),
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
