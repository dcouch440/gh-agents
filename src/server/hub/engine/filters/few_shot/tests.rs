#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::db::traits::MockAgentExecutionRepo;
    use crate::db::AgentExecutionRow;
    use crate::llm::Message;
    use crate::server::hub::engine::filters::FilterContext;
    use chrono::Utc;
    use std::sync::Arc;
    use uuid::Uuid;

    fn make_exemplary_row(
        agent_id: Uuid,
        step_id: Option<Uuid>,
        input: &str,
        output: &str,
    ) -> AgentExecutionRow {
        AgentExecutionRow {
            id: Uuid::new_v4(),
            agent_id,
            workflow_step_id: step_id,
            workflow_execution_id: None,
            is_interactive: false,
            parent_agent_execution_id: None,
            system_prompt_rendered: String::new(),
            input: input.to_string(),
            output: Some(output.to_string()),
            structured_output: None,
            selected_mode_id: None,
            room_session_id: None,
            speaker_order: None,
            status: "completed".into(),
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
            routing_analysis: None,
            selected_routing_document_id: None,
            is_exemplary: true,
        }
    }

    fn mock_repo(rows: Vec<AgentExecutionRow>) -> Arc<dyn crate::db::traits::AgentExecutionRepo> {
        let mut mock = MockAgentExecutionRepo::new();
        mock.expect_list_exemplary_executions()
            .returning(move |_agent_id, _step_id, _limit| Ok(rows.clone()));
        Arc::new(mock)
    }

    #[tokio::test]
    async fn no_examples_passthrough() {
        let repo = mock_repo(vec![]);
        let filter = FewShotFilter::new(repo);
        let ctx = FilterContext::new("model-1", Uuid::new_v4());

        let original_prompt = "You are a helpful assistant.".to_string();
        let original_messages = vec![Message::user("Do something")];
        let (sys, msgs) = filter
            .on_start(&ctx, original_prompt.clone(), original_messages.clone())
            .await
            .unwrap();

        assert_eq!(sys, original_prompt);
        assert_eq!(msgs.len(), 1);
    }

    #[tokio::test]
    async fn injects_examples_as_message_pairs() {
        let agent_id = Uuid::new_v4();
        let step_id = Uuid::new_v4();
        let rows = vec![
            make_exemplary_row(agent_id, Some(step_id), "What is 2+2?", "4"),
            make_exemplary_row(agent_id, Some(step_id), "What is 3+3?", "6"),
        ];
        let repo = mock_repo(rows);
        let filter = FewShotFilter::new(repo);
        let ctx = FilterContext::new("model-1", agent_id).with_step_id(step_id);

        let (sys, msgs) = filter
            .on_start(
                &ctx,
                "Base prompt.".into(),
                vec![Message::user("What is 5+5?")],
            )
            .await
            .unwrap();

        // System prompt should contain the few-shot note.
        assert!(sys.contains("<examples>"));
        assert!(sys.contains("</examples>"));
        assert!(sys.contains("Base prompt."));

        // 2 example pairs (4 messages) + 1 original user message = 5
        assert_eq!(msgs.len(), 5);

        // First pair
        assert_eq!(msgs[0].text(), "What is 2+2?");
        assert_eq!(msgs[1].text(), "4");

        // Second pair
        assert_eq!(msgs[2].text(), "What is 3+3?");
        assert_eq!(msgs[3].text(), "6");

        // Original user message last
        assert_eq!(msgs[4].text(), "What is 5+5?");
    }

    #[tokio::test]
    async fn skips_examples_with_no_output() {
        let agent_id = Uuid::new_v4();
        let row_with_output = make_exemplary_row(agent_id, None, "input1", "output1");

        let mut row_without_output = make_exemplary_row(agent_id, None, "input2", "");
        row_without_output.output = None;

        let repo = mock_repo(vec![row_with_output, row_without_output]);
        let filter = FewShotFilter::new(repo);
        let ctx = FilterContext::new("model-1", agent_id);

        let (sys, msgs) = filter
            .on_start(&ctx, "Prompt.".into(), vec![Message::user("task")])
            .await
            .unwrap();

        // Only 1 example pair (2 messages) + 1 original = 3
        assert!(sys.contains("<examples>"));
        assert!(sys.contains("</examples>"));
        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[0].text(), "input1");
        assert_eq!(msgs[1].text(), "output1");
        assert_eq!(msgs[2].text(), "task");
    }

    #[tokio::test]
    async fn preserves_original_messages() {
        let agent_id = Uuid::new_v4();
        let rows = vec![make_exemplary_row(agent_id, None, "ex_in", "ex_out")];
        let repo = mock_repo(rows);
        let filter = FewShotFilter::new(repo);
        let ctx = FilterContext::new("model-1", agent_id);

        let original = vec![
            Message::user("first user msg"),
            Message::assistant("first assistant msg"),
            Message::user("second user msg"),
        ];

        let (_, msgs) = filter
            .on_start(&ctx, "Prompt.".into(), original)
            .await
            .unwrap();

        // 1 example pair (2 msgs) + 3 original = 5
        assert_eq!(msgs.len(), 5);
        // Examples first
        assert_eq!(msgs[0].text(), "ex_in");
        assert_eq!(msgs[1].text(), "ex_out");
        // Then originals in order
        assert_eq!(msgs[2].text(), "first user msg");
        assert_eq!(msgs[3].text(), "first assistant msg");
        assert_eq!(msgs[4].text(), "second user msg");
    }

    #[tokio::test]
    async fn all_examples_have_no_output_is_passthrough() {
        let agent_id = Uuid::new_v4();
        let mut row = make_exemplary_row(agent_id, None, "input", "");
        row.output = None;

        let repo = mock_repo(vec![row]);
        let filter = FewShotFilter::new(repo);
        let ctx = FilterContext::new("model-1", agent_id);

        let (sys, msgs) = filter
            .on_start(&ctx, "Prompt.".into(), vec![Message::user("task")])
            .await
            .unwrap();

        // No valid examples -> passthrough
        assert_eq!(sys, "Prompt.");
        assert_eq!(msgs.len(), 1);
    }
}
