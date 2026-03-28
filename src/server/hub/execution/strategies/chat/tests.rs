#[cfg(test)]
mod tests {
    //! Tests for chat strategy

    use crate::server::hub::strategies::chat::{ChatConfig, ChatStrategy, StepChatContext};
    use crate::server::hub::strategy::ExecutionStrategy;
    use crate::server::state::test_helpers::default_mock_repos;
    use crate::server::state::AppState;
    use crate::server::ws::events::Topic;
    use crate::types::{AppConfig, UserId};
    use uuid::Uuid;

    fn make_state() -> AppState {
        let repos = default_mock_repos();
        let (state, _rx) = AppState::with_repos(None, repos, AppConfig::default());
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
            model_id: crate::constants::DEFAULT_MODEL.into(),
            ..Default::default()
        };
        let strategy = ChatStrategy::new(config, state, UserId::new(), None, Uuid::new_v4());

        assert_eq!(strategy.system_prompt(), "You are helpful.");
        assert_eq!(strategy.model_id(), crate::constants::DEFAULT_MODEL);
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
    // broadcast_step_event
    // ========================================================================

    #[test]
    fn broadcast_step_event_skips_on_error() {
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
                execution_mode: "workforce".into(),
                step_name: "Test Step".into(),
            },
        );

        let input = serde_json::json!({});
        let result = serde_json::json!({ "error": "Missing required parameter: name" });

        strategy.broadcast_step_event("update_config", &input, &result);

        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn broadcast_step_event_noop_without_step_context() {
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
        let result = serde_json::json!({ "name": "Updated Config" });

        strategy.broadcast_step_event("update_config", &input, &result);

        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn broadcast_step_event_emits_plan_updated() {
        let state = make_state();
        let mut rx = state.events().subscribe();

        let step_id = Uuid::new_v4();
        let workflow_id = Uuid::new_v4();
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
                execution_mode: "single".into(),
                step_name: "Test Step".into(),
            },
        );

        let input = serde_json::json!({ "content": "## Direction\n- Build auth system" });
        let result = serde_json::json!("Notes updated.");

        strategy.broadcast_step_event("update_plan", &input, &result);

        let envelope = rx.try_recv().unwrap();
        assert_eq!(envelope.topic, Topic::Workflow);
        assert!(envelope.run_id.is_none());
        let value: serde_json::Value = serde_json::from_str(&envelope.json).unwrap();
        assert_eq!(value["event"], "plan_updated");
        assert_eq!(value["data"]["workflow_id"], workflow_id.to_string());
        assert_eq!(value["data"]["step_id"], step_id.to_string());
        assert_eq!(
            value["data"]["content"],
            "## Direction\n- Build auth system"
        );
    }

    // ========================================================================
    // resolve_chat_step_tools — workforce gets only universal tools
    // ========================================================================

    #[test]
    fn resolve_chat_step_tools_workforce_only_universal() {
        let tools = super::super::resolve_chat_step_tools("workforce");
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();

        // Has universal tools (minus update_plan which is dispatch-only)
        assert!(names.contains(&"dispatch"));
        assert!(names.contains(&"cancel_dispatch"));
        assert!(names.contains(&"render_panel"));
        assert!(names.contains(&"think"));

        // set_node_name is a universal tool (quick rename is allowed in chat)
        assert!(names.contains(&"set_node_name"));
        // set_node_description is dispatch-only — assistant must dispatch it
        assert!(!names.contains(&"set_node_description"));

        // update_plan is now owned by the assistant (plan is user's design doc)
        assert!(names.contains(&"update_plan"));
    }

    // ========================================================================
    // broadcast_step_event — universal tools
    // ========================================================================

    #[test]
    fn broadcast_step_event_emits_step_name_updated() {
        let state = make_state();
        let mut rx = state.events().subscribe();

        let step_id = Uuid::new_v4();
        let workflow_id = Uuid::new_v4();
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
                execution_mode: "single".into(),
                step_name: "Test Step".into(),
            },
        );

        let input = serde_json::json!({ "name": "My Node" });
        let result = serde_json::json!({
            "name": "My Node",
            "step_id": step_id.to_string(),
        });

        strategy.broadcast_step_event("set_node_name", &input, &result);

        let envelope = rx.try_recv().unwrap();
        assert_eq!(envelope.topic, Topic::Workflow);
        let value: serde_json::Value = serde_json::from_str(&envelope.json).unwrap();
        assert_eq!(value["event"], "step_name_updated");
        assert_eq!(value["data"]["step_id"], step_id.to_string());
        assert_eq!(value["data"]["name"], "My Node");
    }
}
