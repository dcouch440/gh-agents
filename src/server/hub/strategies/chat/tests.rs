#[cfg(test)]
mod tests {
    //! Tests for chat strategy

    use crate::db::traits::MockServerRepo;
    use crate::server::hub::strategies::chat::{ChatConfig, ChatStrategy, StepChatContext};
    use crate::server::hub::strategy::ExecutionStrategy;
    use crate::server::state::test_helpers::default_mock_repos;
    use crate::server::state::AppState;
    use crate::server::ws::events::{ServerEvent, WorkflowEventKind};
    use crate::types::{AppConfig, UserId};
    use std::sync::Arc;
    use uuid::Uuid;

    fn make_state() -> AppState {
        let mut mock = MockServerRepo::new();
        mock.expect_health_check().returning(|| true);
        let repo: Arc<dyn crate::db::traits::ServerRepo> = Arc::new(mock);
        let repos = default_mock_repos();
        let (state, _rx) = AppState::with_repo(None, repo, repos, AppConfig::default());
        state
    }

    #[test]
    fn chat_config_defaults() {
        let config = ChatConfig::default();
        assert_eq!(config.max_rounds, 10);
        assert_eq!(config.context_budget, 480_000);
        assert!(config.tool_names.is_empty());
    }

    #[test]
    fn strategy_properties() {
        let state = make_state();
        let config = ChatConfig {
            system_prompt: "You are helpful.".into(),
            tool_names: vec!["think".into()],
            model_id: "claude-sonnet-4-20250514".into(),
            ..Default::default()
        };
        let strategy = ChatStrategy::new(config, state, UserId::new(), None, Uuid::new_v4());

        assert_eq!(strategy.system_prompt(), "You are helpful.");
        assert_eq!(strategy.model_id(), "claude-sonnet-4-20250514");
        assert_eq!(strategy.max_rounds(), 10);
        assert!(strategy.streaming());
    }

    #[tokio::test]
    async fn build_messages_no_session() {
        let state = make_state();
        let config = ChatConfig {
            system_prompt: "sys".into(),
            model_id: "m".into(),
            ..Default::default()
        };
        let strategy = ChatStrategy::new(config, state, UserId::new(), None, Uuid::new_v4());

        let messages = strategy.build_messages("hello").await.unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].text(), "hello");
    }

    // ========================================================================
    // broadcast_documenter_event
    // ========================================================================

    #[test]
    fn broadcast_documenter_event_emits_on_create_success() {
        let state = make_state();
        let mut rx = state.events().subscribe();

        let step_id = Uuid::new_v4();
        let workflow_id = Uuid::new_v4();
        let doc_id = Uuid::new_v4();
        let strategy = ChatStrategy::with_step_context(
            ChatConfig {
                system_prompt: "sys".into(),
                model_id: "m".into(),
                ..Default::default()
            },
            state,
            UserId::new(),
            None,
            Uuid::new_v4(),
            StepChatContext {
                workflow_id,
                step_id,
                execution_mode: "documenter".into(),
            },
        );

        let input = serde_json::json!({ "name": "API Docs" });
        let result = serde_json::json!({
            "id": doc_id.to_string(),
            "name": "API Docs",
            "description": "",
            "target_length": 1500,
        });

        strategy.broadcast_documenter_event("create_doc_def", &input, &result);

        let event = rx.try_recv().unwrap();
        match event {
            ServerEvent::Workflow(e) => {
                assert!(e.run_id.is_none());
                assert_eq!(e.workflow_id, workflow_id);
                match e.kind {
                    WorkflowEventKind::DocDefCreated {
                        step_id: sid,
                        doc_def_id,
                        name,
                    } => {
                        assert_eq!(sid, step_id);
                        assert_eq!(doc_def_id, doc_id);
                        assert_eq!(name, "API Docs");
                    }
                    other => panic!("Expected DocDefCreated, got {:?}", other),
                }
            }
            other => panic!("Expected Workflow event, got {:?}", other),
        }
    }

    #[test]
    fn broadcast_documenter_event_skips_on_error() {
        let state = make_state();
        let mut rx = state.events().subscribe();

        let strategy = ChatStrategy::with_step_context(
            ChatConfig {
                system_prompt: "sys".into(),
                model_id: "m".into(),
                ..Default::default()
            },
            state,
            UserId::new(),
            None,
            Uuid::new_v4(),
            StepChatContext {
                workflow_id: Uuid::new_v4(),
                step_id: Uuid::new_v4(),
                execution_mode: "documenter".into(),
            },
        );

        let input = serde_json::json!({});
        let result = serde_json::json!({ "error": "Missing required parameter: name" });

        strategy.broadcast_documenter_event("create_doc_def", &input, &result);

        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn broadcast_documenter_event_noop_without_step_context() {
        let state = make_state();
        let mut rx = state.events().subscribe();

        let strategy = ChatStrategy::new(
            ChatConfig {
                system_prompt: "sys".into(),
                model_id: "m".into(),
                ..Default::default()
            },
            state,
            UserId::new(),
            None,
            Uuid::new_v4(),
        );

        let input = serde_json::json!({});
        let result = serde_json::json!({ "id": Uuid::new_v4().to_string() });

        strategy.broadcast_documenter_event("create_doc_def", &input, &result);

        assert!(rx.try_recv().is_err());
    }
}
