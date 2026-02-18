#[cfg(test)]
mod tests {
    //! Tests for hub module

    use uuid::Uuid;

    use crate::server::hub::ChatConfig;
    use crate::server::state::test_helpers::default_mock_repos;
    use crate::server::state::AppState;
    use crate::types::AppConfig;

    fn make_state() -> AppState {
        let repos = default_mock_repos();
        let (state, _rx) = AppState::with_repos(None, repos, AppConfig::default());
        state
    }

    #[test]
    fn chat_config_default_has_sane_values() {
        let config = ChatConfig::default();
        assert!(config.system_prompt.is_empty());
        assert!(config.tool_names.is_empty());
        assert!(config.max_rounds > 0);
        assert!(config.context_budget > 0);
    }

    // ========================================================================
    // build_dispatch_status
    // ========================================================================

    #[test]
    fn build_dispatch_status_empty_when_no_tasks() {
        let state = make_state();
        let result = crate::server::hub::build_dispatch_status(&state, Uuid::new_v4());
        assert!(result.is_empty());
    }

    #[test]
    fn build_dispatch_status_shows_running_task() {
        let state = make_state();
        let step_id = Uuid::new_v4();

        state.task_registry().spawn_task(
            step_id,
            Uuid::new_v4(),
            Uuid::new_v4(),
            "Add a researcher agent".to_string(),
        );

        let result = crate::server::hub::build_dispatch_status(&state, step_id);
        assert!(result.contains("<dispatch_status>"));
        assert!(result.contains("RUNNING"));
        assert!(result.contains("Add a researcher agent"));
    }

    #[test]
    fn build_dispatch_status_shows_completed_task() {
        let state = make_state();
        let step_id = Uuid::new_v4();

        let (exec_id, _) = state.task_registry().spawn_task(
            step_id,
            Uuid::new_v4(),
            Uuid::new_v4(),
            "Set up the team".to_string(),
        );
        state
            .task_registry()
            .mark_completed(exec_id, Some("Added 3 agents".to_string()));

        let result = crate::server::hub::build_dispatch_status(&state, step_id);
        assert!(result.contains("DONE"));
        assert!(result.contains("Added 3 agents"));
    }

    #[test]
    fn build_dispatch_status_ignores_other_steps() {
        let state = make_state();
        let step_a = Uuid::new_v4();
        let step_b = Uuid::new_v4();

        state.task_registry().spawn_task(
            step_a,
            Uuid::new_v4(),
            Uuid::new_v4(),
            "task for step A".to_string(),
        );

        let result = crate::server::hub::build_dispatch_status(&state, step_b);
        assert!(result.is_empty());
    }

    // ========================================================================
    // truncate_str
    // ========================================================================

    #[test]
    fn truncate_str_short_input() {
        assert_eq!(crate::server::hub::truncate_str("hello", 10), "hello");
    }

    #[test]
    fn truncate_str_exact_length() {
        assert_eq!(crate::server::hub::truncate_str("hello", 5), "hello");
    }

    #[test]
    fn truncate_str_long_input() {
        let result = crate::server::hub::truncate_str("hello world", 5);
        assert_eq!(result, "hello");
    }
}
