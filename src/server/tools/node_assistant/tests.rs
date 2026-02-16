#[cfg(test)]
mod tests {
    use serde_json::json;
    use uuid::Uuid;

    use crate::db::traits::MockWorkflowRepo;
    use crate::db::WorkflowStepRow;

    use super::super::{execute_node_assistant_tool, StepToolContext};

    fn make_ctx() -> StepToolContext {
        StepToolContext {
            workflow_id: Uuid::new_v4(),
            step_id: Uuid::new_v4(),
        }
    }

    fn make_step(id: Uuid, workflow_id: Uuid) -> WorkflowStepRow {
        WorkflowStepRow {
            id,
            workflow_id,
            agent_id: None,
            execution_mode: "single".to_string(),
            agent_execution_mode: None,
            for_each_ref: None,
            prompt_template_id: None,
            prompt_template: String::new(),
            output_schema_id: None,
            output_variable_name: None,
            interactive_agent_id: None,
            for_each_label_field: None,
            room_id: None,
            routing_mode: None,
            routing_field: None,
            display_order: 0,
            version: 1,
            reasoning_trace: false,
            verification_agent_ids: None,
            position_x: None,
            position_y: None,
            width: None,
            height: None,
            name: Some("Test Step".to_string()),
            system_prompt_suffix: None,
            visible: true,
            description: "Test description".to_string(),
            board_context_cache: String::new(),
            board_context_updated_at: None,
            goal_summary: String::new(),
            goal_summary_updated_at: None,
            sub_workflow_template_id: None,
            child_workflow_id: None,
            is_designer_step: false,
        }
    }

    #[tokio::test]
    async fn set_archetype_updates_execution_mode() {
        let ctx = make_ctx();
        let step = make_step(ctx.step_id, ctx.workflow_id);
        let step_id = ctx.step_id;
        let step_clone = step.clone();

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_step()
            .withf(move |id| *id == step_id)
            .returning(move |_| Ok(Some(step_clone.clone())));
        repo.expect_update_step().returning(|step| {
            assert_eq!(step.execution_mode, "workforce");
            Ok(step)
        });

        let input = json!({ "archetype": "workforce" });
        let result = execute_node_assistant_tool("set_node_archetype", &input, &repo, &ctx).await;

        assert_eq!(result["archetype"], "workforce");
        assert!(result.get("error").is_none());
    }

    #[tokio::test]
    async fn set_archetype_rejects_invalid() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let input = json!({ "archetype": "invalid_type" });
        let result = execute_node_assistant_tool("set_node_archetype", &input, &repo, &ctx).await;

        assert!(result["error"]
            .as_str()
            .unwrap()
            .contains("Invalid archetype"));
    }

    #[tokio::test]
    async fn set_archetype_rejects_missing() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let input = json!({});
        let result = execute_node_assistant_tool("set_node_archetype", &input, &repo, &ctx).await;

        assert!(result["error"]
            .as_str()
            .unwrap()
            .contains("Missing required parameter"));
    }

    #[tokio::test]
    async fn set_name_updates_step() {
        let ctx = make_ctx();
        let step = make_step(ctx.step_id, ctx.workflow_id);
        let step_id = ctx.step_id;
        let step_clone = step.clone();

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_step()
            .withf(move |id| *id == step_id)
            .returning(move |_| Ok(Some(step_clone.clone())));
        repo.expect_update_step().returning(|step| {
            assert_eq!(step.name, Some("New Name".to_string()));
            Ok(step)
        });

        let input = json!({ "name": "New Name" });
        let result = execute_node_assistant_tool("set_node_name", &input, &repo, &ctx).await;

        assert_eq!(result["name"], "New Name");
    }

    #[tokio::test]
    async fn set_description_updates_step() {
        let ctx = make_ctx();
        let step = make_step(ctx.step_id, ctx.workflow_id);
        let step_id = ctx.step_id;
        let step_clone = step.clone();

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_step()
            .withf(move |id| *id == step_id)
            .returning(move |_| Ok(Some(step_clone.clone())));
        repo.expect_update_step().returning(|step| {
            assert_eq!(step.description, "New description");
            Ok(step)
        });

        let input = json!({ "description": "New description" });
        let result = execute_node_assistant_tool("set_node_description", &input, &repo, &ctx).await;

        assert_eq!(result["description"], "New description");
    }

    #[tokio::test]
    async fn render_panel_returns_content() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let input = json!({
            "content": "# Plan\n- [ ] Step 1\n- [ ] Step 2",
            "submit_label": "Approve"
        });
        let result = execute_node_assistant_tool("render_panel", &input, &repo, &ctx).await;

        assert_eq!(result["rendered"], true);
        assert_eq!(result["content"], "# Plan\n- [ ] Step 1\n- [ ] Step 2");
        assert_eq!(result["submit_label"], "Approve");
    }

    #[tokio::test]
    async fn render_panel_defaults_submit_label() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let input = json!({ "content": "# Hello" });
        let result = execute_node_assistant_tool("render_panel", &input, &repo, &ctx).await;

        assert_eq!(result["rendered"], true);
        assert_eq!(result["submit_label"], "Submit");
    }

    #[tokio::test]
    async fn render_panel_rejects_missing_content() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let input = json!({});
        let result = execute_node_assistant_tool("render_panel", &input, &repo, &ctx).await;

        assert!(result["error"]
            .as_str()
            .unwrap()
            .contains("Missing required parameter"));
    }

    #[tokio::test]
    async fn all_valid_archetypes_accepted() {
        for archetype in &["belief_capture", "room", "workforce"] {
            let ctx = make_ctx();
            let step = make_step(ctx.step_id, ctx.workflow_id);
            let step_id = ctx.step_id;
            let step_clone = step.clone();

            let mut repo = MockWorkflowRepo::new();
            repo.expect_get_step()
                .withf(move |id| *id == step_id)
                .returning(move |_| Ok(Some(step_clone.clone())));
            repo.expect_update_step().returning(|step| Ok(step));

            let input = json!({ "archetype": archetype });
            let result =
                execute_node_assistant_tool("set_node_archetype", &input, &repo, &ctx).await;

            assert_eq!(
                result["archetype"].as_str().unwrap(),
                *archetype,
                "Failed for archetype: {}",
                archetype
            );
        }
    }
}
