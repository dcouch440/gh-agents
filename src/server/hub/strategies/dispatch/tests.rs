#[cfg(test)]
mod tests {
    use crate::server::hub::strategies::chat::tools::resolve_step_tools;
    use crate::server::state::test_helpers::default_mock_repos;
    use crate::server::state::AppState;
    use crate::server::ws::events::Topic;
    use crate::types::AppConfig;
    use uuid::Uuid;

    fn make_state() -> AppState {
        let repos = default_mock_repos();
        let (state, _rx) = AppState::with_repos(None, repos, AppConfig::default());
        state
    }

    #[test]
    fn dispatch_tools_match_workforce_tools() {
        let tools = resolve_step_tools("workforce");
        let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();

        // Must include universal tools
        assert!(tool_names.contains(&"set_node_name"));
        assert!(tool_names.contains(&"set_node_description"));
        assert!(tool_names.contains(&"think"));
        assert!(tool_names.contains(&"update_plan"));

        // Must include workforce tools
        assert!(tool_names.contains(&"set_task"));
        assert!(tool_names.contains(&"add_agent"));
        assert!(tool_names.contains(&"update_agent"));
        assert!(tool_names.contains(&"remove_agent"));
        assert!(tool_names.contains(&"set_dependency"));
        assert!(tool_names.contains(&"remove_dependency"));
        assert!(tool_names.contains(&"set_capabilities"));
        assert!(tool_names.contains(&"set_failure_mode"));
    }

    #[test]
    fn dispatch_tools_include_render_panel() {
        let tools = resolve_step_tools("workforce");
        let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(tool_names.contains(&"render_panel"));
    }

    // ========================================================================
    // broadcast_tool_event — DispatchStrategy broadcasts live updates
    // ========================================================================
    //
    // These tests exercise the shared broadcast function as the DispatchStrategy
    // calls it: with a StepChatContext and user_id = None.

    use crate::server::hub::strategies::chat::broadcast;
    use crate::server::hub::strategies::chat::config::StepChatContext;

    fn make_dispatch_ctx(step_id: Uuid, workflow_id: Uuid) -> StepChatContext {
        StepChatContext {
            workflow_id,
            step_id,
            execution_mode: "workforce".to_string(),
            step_name: String::new(),
        }
    }

    #[test]
    fn dispatch_broadcasts_roster_changed() {
        let state = make_state();
        let mut rx = state.events().subscribe();

        let step_id = Uuid::new_v4();
        let workflow_id = Uuid::new_v4();
        let ctx = make_dispatch_ctx(step_id, workflow_id);

        let input = serde_json::json!({ "name": "researcher", "role": "Finds info" });
        let result = serde_json::json!({
            "id": Uuid::new_v4().to_string(),
            "name": "researcher",
        });

        broadcast::broadcast_step_event(&state, Some(&ctx), None, "add_agent", &input, &result);

        let envelope = rx.try_recv().unwrap();
        assert_eq!(envelope.topic, Topic::Workflow);
        assert!(envelope.run_id.is_none());
        let value: serde_json::Value = serde_json::from_str(&envelope.json).unwrap();
        assert_eq!(value["event"], "roster_changed");
        assert_eq!(value["data"]["step_id"], step_id.to_string());
    }

    #[test]
    fn dispatch_broadcasts_step_name_updated() {
        let state = make_state();
        let mut rx = state.events().subscribe();

        let step_id = Uuid::new_v4();
        let workflow_id = Uuid::new_v4();
        let ctx = make_dispatch_ctx(step_id, workflow_id);

        let input = serde_json::json!({ "name": "Security Team" });
        let result = serde_json::json!({
            "name": "Security Team",
            "step_id": step_id.to_string(),
        });

        broadcast::broadcast_step_event(&state, Some(&ctx), None, "set_node_name", &input, &result);

        let envelope = rx.try_recv().unwrap();
        assert_eq!(envelope.topic, Topic::Workflow);
        let value: serde_json::Value = serde_json::from_str(&envelope.json).unwrap();
        assert_eq!(value["event"], "step_name_updated");
        assert_eq!(value["data"]["name"], "Security Team");
    }

    #[test]
    fn dispatch_skips_broadcast_on_error() {
        let state = make_state();
        let mut rx = state.events().subscribe();

        let ctx = make_dispatch_ctx(Uuid::new_v4(), Uuid::new_v4());

        let input = serde_json::json!({});
        let result = serde_json::json!({ "error": "Missing required parameter" });

        broadcast::broadcast_step_event(&state, Some(&ctx), None, "add_agent", &input, &result);

        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn dispatch_broadcast_has_no_user_id() {
        let state = make_state();
        let mut rx = state.events().subscribe();

        let ctx = make_dispatch_ctx(Uuid::new_v4(), Uuid::new_v4());

        let input = serde_json::json!({ "description": "Scan repos" });
        let result = serde_json::json!({ "status": "ok" });

        broadcast::broadcast_step_event(&state, Some(&ctx), None, "set_task", &input, &result);

        let envelope = rx.try_recv().unwrap();
        let value: serde_json::Value = serde_json::from_str(&envelope.json).unwrap();
        // Background dispatch has no user_id
        assert!(value["user_id"].is_null());
    }
}
