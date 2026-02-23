#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::server::services::dispatch::{DispatchInput, DispatchOutput, Passdown};
    use crate::types::UserId;

    #[test]
    fn dispatch_input_fields() {
        let input = DispatchInput {
            step_id: Uuid::new_v4(),
            workflow_id: Uuid::new_v4(),
            user_id: UserId(Uuid::new_v4()),
            instruction: "Configure a 3-agent team".to_string(),
            execution_mode: "workforce".to_string(),
        };

        assert!(!input.instruction.is_empty());
        assert_eq!(input.execution_mode, "workforce");
    }

    #[test]
    fn dispatch_output_fields() {
        let output = DispatchOutput {
            execution_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
        };

        assert_ne!(output.execution_id, output.session_id);
    }

    #[test]
    fn passdown_serialization_round_trip() {
        let passdown = Passdown {
            plan: "## Objective\nScan for vulnerabilities".to_string(),
            summary: "Configured 3-agent pipeline".to_string(),
            question: Some("Which repos should we target?".to_string()),
        };

        let json = serde_json::to_string(&passdown).unwrap();
        let restored: Passdown = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.plan, passdown.plan);
        assert_eq!(restored.summary, passdown.summary);
        assert_eq!(restored.question, passdown.question);
    }

    #[test]
    fn passdown_question_none_omitted_in_json() {
        let passdown = Passdown {
            plan: "## Objective\nResearch competitors".to_string(),
            summary: "Configured 4-agent team".to_string(),
            question: None,
        };

        let json = serde_json::to_string(&passdown).unwrap();
        assert!(!json.contains("question"));
    }

    #[test]
    fn passdown_has_question_derived() {
        let with = Passdown {
            plan: String::new(),
            summary: "Done".to_string(),
            question: Some("Need input".to_string()),
        };
        let without = Passdown {
            plan: String::new(),
            summary: "Done".to_string(),
            question: None,
        };

        assert!(with.question.is_some());
        assert!(without.question.is_none());
    }

    #[test]
    fn cancel_dispatch_returns_false_for_unknown_id() {
        use crate::server::state::test_helpers::default_mock_repos;
        use crate::server::state::AppState;
        use crate::types::AppConfig;

        let repos = default_mock_repos();
        let (state, _rx) = AppState::with_repos(None, repos, AppConfig::default());

        let result =
            crate::server::services::dispatch::cancel_dispatch(&state, Uuid::new_v4(), Uuid::new_v4());
        assert!(!result);
    }
}
