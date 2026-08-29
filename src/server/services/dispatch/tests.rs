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
            summary: "Configured 3-agent pipeline".to_string(),
            question: Some("Which repos should we target?".to_string()),
        };

        let json = serde_json::to_string(&passdown).unwrap();
        let restored: Passdown = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.summary, passdown.summary);
        assert_eq!(restored.question, passdown.question);
    }

    #[test]
    fn passdown_question_none_omitted_in_json() {
        let passdown = Passdown {
            summary: "Configured 4-agent team".to_string(),
            question: None,
        };

        let json = serde_json::to_string(&passdown).unwrap();
        assert!(!json.contains("question"));
    }

    #[test]
    fn passdown_has_question_derived() {
        let with = Passdown {
            summary: "Done".to_string(),
            question: Some("Need input".to_string()),
        };
        let without = Passdown {
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

        let result = crate::server::services::dispatch::cancel_dispatch(
            &state,
            Uuid::new_v4(),
            Uuid::new_v4(),
        );
        assert!(!result);
    }

    // ── Board spec ──────────────────────────────────────────────────────────
    //
    // The designer's whole input used to be the node's own sentence. A brief
    // carrying an output schema, a fixed vocabulary and a table of exit codes
    // had no channel to reach it, so the node text became a lossy paraphrase
    // and the contracts were gone before any agent was designed. board.md is
    // that channel, and this is where it joins the instruction.

    use crate::server::services::dispatch::sequential::append_board_spec;

    /// Most boards are a plan and nothing more. A board with no contracts must
    /// not gain an empty block that reads as "there are no rules".
    #[test]
    fn a_board_without_a_spec_appends_nothing() {
        let instruction = "Configure this new workflow node.";
        assert_eq!(append_board_spec(instruction, ""), instruction);
        assert_eq!(append_board_spec(instruction, "   \n\n  "), instruction);
    }

    /// Verbatim is the whole point: a schema that arrives reworded has lost
    /// the types and ranges that made it worth carrying.
    #[test]
    fn a_board_spec_arrives_whole_and_tagged() {
        let spec = "# Output\n\n  id     string\n  score  number  0.0 to 1.0";
        let out = append_board_spec("Configure this new workflow node.", spec);

        assert!(out.contains(spec), "spec must survive unchanged");
        assert!(out.contains("<board_spec>"));
        assert!(out.ends_with("</board_spec>"));
    }

    /// Last, not first. The node text is what the turn is about; prepending a
    /// page of schema buries the instruction it is meant to support.
    #[test]
    fn a_board_spec_follows_the_node_text() {
        let out = append_board_spec(
            "Configure this new workflow node.\n\n<user_text>\nBuild it.\n</user_text>",
            "One rule.",
        );

        let node_text = out.find("<user_text>").unwrap();
        let spec = out.find("<board_spec>").unwrap();
        assert!(node_text < spec, "node text must come before the spec");
    }

    /// Whitespace from a heredoc is not content, and the block should not
    /// carry blank lines that make the spec look like it starts elsewhere.
    #[test]
    fn a_board_spec_is_trimmed_before_it_is_wrapped() {
        let out = append_board_spec("Task.", "\n\nOne rule.\n\n");
        assert!(out.ends_with("<board_spec>\nOne rule.\n</board_spec>"));
    }
}
