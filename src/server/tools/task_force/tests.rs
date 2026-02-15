#[cfg(test)]
mod tests {
    use chrono::Utc;
    use serde_json::json;
    use uuid::Uuid;

    use crate::db::traits::MockWorkflowRepo;
    use crate::db::{
        TaskAgentRosterRow, TaskMissionBriefRow, WorkflowStepEdgeRow, WorkflowStepRow,
    };

    use super::super::{
        build_config_snapshot, execute_task_force_tool, TaskForceToolContext, VALID_FAILURE_MODES,
    };

    fn make_ctx() -> TaskForceToolContext {
        TaskForceToolContext {
            workflow_id: Uuid::new_v4(),
            step_id: Uuid::new_v4(),
        }
    }

    fn make_step(id: Uuid, workflow_id: Uuid, mode: &str) -> WorkflowStepRow {
        WorkflowStepRow {
            id,
            workflow_id,
            agent_id: None,
            execution_mode: mode.to_string(),
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
            height: None,            name: Some("Test Step".to_string()),
            system_prompt_suffix: None,
            visible: true,
            description: "Test description".to_string(),
            board_context_cache: String::new(),
            board_context_updated_at: None,
            goal_summary: String::new(),
            goal_summary_updated_at: None,
        }
    }

    fn make_brief(step_id: Uuid) -> TaskMissionBriefRow {
        TaskMissionBriefRow {
            id: Uuid::new_v4(),
            step_id,
            task_description: "Implement auth flow".to_string(),
            available_capabilities: vec!["code_gen".to_string(), "testing".to_string()],
            failure_mode: "fail_fast".to_string(),
            downstream_context: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn make_roster_agent(brief_id: Uuid, name: &str, order: i32) -> TaskAgentRosterRow {
        TaskAgentRosterRow {
            id: Uuid::new_v4(),
            mission_brief_id: brief_id,
            name: name.to_string(),
            role_description: "Worker".to_string(),
            capabilities: vec!["code_gen".to_string()],
            execution_order: order,
            created_at: Utc::now(),
        }
    }

    // =========================================================================
    // VALID_FAILURE_MODES
    // =========================================================================

    #[test]
    fn valid_failure_modes_contains_expected() {
        assert!(VALID_FAILURE_MODES.contains(&"fail_fast"));
        assert!(VALID_FAILURE_MODES.contains(&"skip_and_continue"));
        assert!(VALID_FAILURE_MODES.contains(&"retry"));
        assert!(!VALID_FAILURE_MODES.contains(&"explode"));
    }

    // =========================================================================
    // set_task
    // =========================================================================

    #[tokio::test]
    async fn set_task_creates_brief() {
        let ctx = make_ctx();
        let step_id = ctx.step_id;

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_mission_brief().returning(|_| Ok(None));
        repo.expect_upsert_mission_brief()
            .returning(move |sid, desc, caps, fm, dc| {
                assert_eq!(sid, step_id);
                assert_eq!(desc, "Build login page");
                assert_eq!(caps, &[] as &[String]);
                assert_eq!(fm, "fail_fast");
                assert!(dc.is_none());
                Ok(TaskMissionBriefRow {
                    id: Uuid::new_v4(),
                    step_id: sid,
                    task_description: desc.to_string(),
                    available_capabilities: caps.to_vec(),
                    failure_mode: fm.to_string(),
                    downstream_context: dc,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                })
            });

        let input = json!({ "description": "Build login page" });
        let result = execute_task_force_tool("set_task", &input, &repo, &ctx).await;

        assert_eq!(result["task_description"], "Build login page");
        assert_eq!(result["step_id"], step_id.to_string());
    }

    #[tokio::test]
    async fn set_task_preserves_existing_fields() {
        let ctx = make_ctx();
        let existing = make_brief(ctx.step_id);

        let mut repo = MockWorkflowRepo::new();
        let existing_clone = existing.clone();
        repo.expect_get_mission_brief()
            .returning(move |_| Ok(Some(existing_clone.clone())));
        repo.expect_upsert_mission_brief()
            .returning(move |_, desc, caps, fm, _| {
                assert_eq!(desc, "New task");
                assert_eq!(caps, &["code_gen".to_string(), "testing".to_string()]);
                assert_eq!(fm, "fail_fast");
                Ok(TaskMissionBriefRow {
                    id: Uuid::new_v4(),
                    step_id: Uuid::new_v4(),
                    task_description: desc.to_string(),
                    available_capabilities: caps.to_vec(),
                    failure_mode: fm.to_string(),
                    downstream_context: None,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                })
            });

        let input = json!({ "description": "New task" });
        let result = execute_task_force_tool("set_task", &input, &repo, &ctx).await;

        assert_eq!(result["task_description"], "New task");
    }

    #[tokio::test]
    async fn set_task_missing_description_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let result = execute_task_force_tool("set_task", &json!({}), &repo, &ctx).await;

        assert_eq!(
            result["error"].as_str().unwrap(),
            "Missing required parameter: description"
        );
    }

    // =========================================================================
    // add_agent
    // =========================================================================

    #[tokio::test]
    async fn add_agent_with_all_fields() {
        let ctx = make_ctx();
        let brief = make_brief(ctx.step_id);
        let brief_id = brief.id;

        let mut repo = MockWorkflowRepo::new();
        let brief_clone = brief.clone();
        repo.expect_get_mission_brief()
            .returning(move |_| Ok(Some(brief_clone.clone())));
        repo.expect_list_agent_roster().returning(|_| Ok(vec![]));
        repo.expect_add_roster_agent()
            .returning(move |bid, name, role, caps, order| {
                assert_eq!(bid, brief_id);
                assert_eq!(name, "Coder");
                assert_eq!(role, "Backend developer");
                assert_eq!(caps, &["rust".to_string(), "sql".to_string()]);
                assert_eq!(order, 0);
                Ok(TaskAgentRosterRow {
                    id: Uuid::new_v4(),
                    mission_brief_id: bid,
                    name: name.to_string(),
                    role_description: role.to_string(),
                    capabilities: caps.to_vec(),
                    execution_order: order,
                    created_at: Utc::now(),
                })
            });

        let input = json!({
            "name": "Coder",
            "role": "Backend developer",
            "capabilities": ["rust", "sql"]
        });
        let result = execute_task_force_tool("add_agent", &input, &repo, &ctx).await;

        assert_eq!(result["name"], "Coder");
        assert_eq!(result["role"], "Backend developer");
        assert_eq!(result["execution_order"], 0);
    }

    #[tokio::test]
    async fn add_agent_auto_orders() {
        let ctx = make_ctx();
        let brief = make_brief(ctx.step_id);
        let brief_id = brief.id;
        let existing_agent = make_roster_agent(brief_id, "Agent1", 0);

        let mut repo = MockWorkflowRepo::new();
        let brief_clone = brief.clone();
        repo.expect_get_mission_brief()
            .returning(move |_| Ok(Some(brief_clone.clone())));
        repo.expect_list_agent_roster()
            .returning(move |_| Ok(vec![existing_agent.clone()]));
        repo.expect_add_roster_agent()
            .returning(move |bid, name, role, caps, order| {
                assert_eq!(order, 1); // auto-incremented
                Ok(TaskAgentRosterRow {
                    id: Uuid::new_v4(),
                    mission_brief_id: bid,
                    name: name.to_string(),
                    role_description: role.to_string(),
                    capabilities: caps.to_vec(),
                    execution_order: order,
                    created_at: Utc::now(),
                })
            });

        let input = json!({ "name": "Agent2" });
        let result = execute_task_force_tool("add_agent", &input, &repo, &ctx).await;

        assert_eq!(result["execution_order"], 1);
    }

    #[tokio::test]
    async fn add_agent_missing_name_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let input = json!({ "role": "Worker" });
        let result = execute_task_force_tool("add_agent", &input, &repo, &ctx).await;

        assert_eq!(
            result["error"].as_str().unwrap(),
            "Missing required parameter: name"
        );
    }

    // =========================================================================
    // update_agent
    // =========================================================================

    #[tokio::test]
    async fn update_agent_partial_fields() {
        let ctx = make_ctx();
        let agent_id = Uuid::new_v4();

        let mut repo = MockWorkflowRepo::new();
        repo.expect_update_roster_agent()
            .returning(move |id, name, role, caps| {
                assert_eq!(id, agent_id);
                assert!(name.is_none());
                assert_eq!(role.as_deref(), Some("Senior Dev"));
                assert!(caps.is_none());
                Ok(TaskAgentRosterRow {
                    id,
                    mission_brief_id: Uuid::new_v4(),
                    name: "Coder".to_string(),
                    role_description: "Senior Dev".to_string(),
                    capabilities: vec![],
                    execution_order: 0,
                    created_at: Utc::now(),
                })
            });

        let input = json!({
            "agent_id": agent_id.to_string(),
            "role": "Senior Dev"
        });
        let result = execute_task_force_tool("update_agent", &input, &repo, &ctx).await;

        assert_eq!(result["role"], "Senior Dev");
        assert_eq!(result["name"], "Coder");
    }

    #[tokio::test]
    async fn update_agent_missing_id_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let input = json!({ "name": "Test" });
        let result = execute_task_force_tool("update_agent", &input, &repo, &ctx).await;

        assert_eq!(
            result["error"].as_str().unwrap(),
            "Missing required parameter: agent_id"
        );
    }

    #[tokio::test]
    async fn update_agent_invalid_uuid_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let input = json!({ "agent_id": "not-a-uuid" });
        let result = execute_task_force_tool("update_agent", &input, &repo, &ctx).await;

        assert!(result["error"].as_str().unwrap().contains("Invalid UUID"));
    }

    // =========================================================================
    // remove_agent
    // =========================================================================

    #[tokio::test]
    async fn remove_agent_success() {
        let ctx = make_ctx();
        let agent_id = Uuid::new_v4();

        let mut repo = MockWorkflowRepo::new();
        repo.expect_remove_roster_agent()
            .withf(move |id| *id == agent_id)
            .returning(|_| Ok(()));

        let input = json!({ "agent_id": agent_id.to_string() });
        let result = execute_task_force_tool("remove_agent", &input, &repo, &ctx).await;

        assert_eq!(result["deleted"], true);
    }

    #[tokio::test]
    async fn remove_agent_missing_id_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let result = execute_task_force_tool("remove_agent", &json!({}), &repo, &ctx).await;

        assert_eq!(
            result["error"].as_str().unwrap(),
            "Missing required parameter: agent_id"
        );
    }

    #[tokio::test]
    async fn remove_agent_invalid_uuid_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let input = json!({ "agent_id": "bad" });
        let result = execute_task_force_tool("remove_agent", &input, &repo, &ctx).await;

        assert!(result["error"].as_str().unwrap().contains("Invalid UUID"));
    }

    // =========================================================================
    // set_capabilities
    // =========================================================================

    #[tokio::test]
    async fn set_capabilities_creates_brief() {
        let ctx = make_ctx();

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_mission_brief().returning(|_| Ok(None));
        repo.expect_upsert_mission_brief()
            .returning(move |_, desc, caps, fm, dc| {
                assert_eq!(desc, "");
                assert_eq!(caps, &["read_file".to_string(), "write_file".to_string()]);
                assert_eq!(fm, "fail_fast");
                assert!(dc.is_none());
                Ok(TaskMissionBriefRow {
                    id: Uuid::new_v4(),
                    step_id: Uuid::new_v4(),
                    task_description: desc.to_string(),
                    available_capabilities: caps.to_vec(),
                    failure_mode: fm.to_string(),
                    downstream_context: dc,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                })
            });

        let input = json!({ "capabilities": ["read_file", "write_file"] });
        let result = execute_task_force_tool("set_capabilities", &input, &repo, &ctx).await;

        let caps = result["capabilities"].as_array().unwrap();
        assert_eq!(caps.len(), 2);
        assert_eq!(caps[0], "read_file");
    }

    #[tokio::test]
    async fn set_capabilities_preserves_existing_fields() {
        let ctx = make_ctx();
        let existing = make_brief(ctx.step_id);

        let mut repo = MockWorkflowRepo::new();
        let existing_clone = existing.clone();
        repo.expect_get_mission_brief()
            .returning(move |_| Ok(Some(existing_clone.clone())));
        repo.expect_upsert_mission_brief()
            .returning(move |_, desc, _, fm, _| {
                assert_eq!(desc, "Implement auth flow");
                assert_eq!(fm, "fail_fast");
                Ok(TaskMissionBriefRow {
                    id: Uuid::new_v4(),
                    step_id: Uuid::new_v4(),
                    task_description: desc.to_string(),
                    available_capabilities: vec!["new_cap".to_string()],
                    failure_mode: fm.to_string(),
                    downstream_context: None,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                })
            });

        let input = json!({ "capabilities": ["new_cap"] });
        let result = execute_task_force_tool("set_capabilities", &input, &repo, &ctx).await;

        let caps = result["capabilities"].as_array().unwrap();
        assert_eq!(caps[0], "new_cap");
    }

    #[tokio::test]
    async fn set_capabilities_missing_array_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let result = execute_task_force_tool("set_capabilities", &json!({}), &repo, &ctx).await;

        assert!(result["error"].as_str().unwrap().contains("capabilities"));
    }

    // =========================================================================
    // set_failure_mode
    // =========================================================================

    #[tokio::test]
    async fn set_failure_mode_all_valid_modes() {
        for mode in &["fail_fast", "skip_and_continue", "retry"] {
            let ctx = make_ctx();
            let expected = mode.to_string();

            let mut repo = MockWorkflowRepo::new();
            repo.expect_get_mission_brief().returning(|_| Ok(None));
            repo.expect_upsert_mission_brief()
                .returning(move |_, _, _, fm, _| {
                    Ok(TaskMissionBriefRow {
                        id: Uuid::new_v4(),
                        step_id: Uuid::new_v4(),
                        task_description: String::new(),
                        available_capabilities: vec![],
                        failure_mode: fm.to_string(),
                        downstream_context: None,
                        created_at: Utc::now(),
                        updated_at: Utc::now(),
                    })
                });

            let input = json!({ "mode": *mode });
            let result = execute_task_force_tool("set_failure_mode", &input, &repo, &ctx).await;

            assert_eq!(result["failure_mode"], expected, "mode={mode}");
        }
    }

    #[tokio::test]
    async fn set_failure_mode_invalid_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let input = json!({ "mode": "explode" });
        let result = execute_task_force_tool("set_failure_mode", &input, &repo, &ctx).await;

        let err = result["error"].as_str().unwrap();
        assert!(err.contains("Invalid failure mode"));
        assert!(err.contains("explode"));
    }

    #[tokio::test]
    async fn set_failure_mode_missing_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let result = execute_task_force_tool("set_failure_mode", &json!({}), &repo, &ctx).await;

        assert_eq!(
            result["error"].as_str().unwrap(),
            "Missing required parameter: mode"
        );
    }

    // =========================================================================
    // build_config_snapshot
    // =========================================================================

    #[tokio::test]
    async fn build_config_snapshot_with_brief_and_roster() {
        let ctx = make_ctx();
        let step = make_step(ctx.step_id, ctx.workflow_id, "task_force");
        let brief = make_brief(ctx.step_id);
        let brief_id = brief.id;
        let agent = make_roster_agent(brief_id, "Coder", 0);

        let step_id = ctx.step_id;
        let wf_id = ctx.workflow_id;
        let step_clone = step.clone();

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_step()
            .withf(move |id| *id == step_id)
            .returning(move |_| Ok(Some(step_clone.clone())));
        repo.expect_get_mission_brief()
            .returning(move |_| Ok(Some(brief.clone())));
        repo.expect_list_agent_roster()
            .returning(move |_| Ok(vec![agent.clone()]));
        repo.expect_list_edges()
            .withf(move |wid| *wid == wf_id)
            .returning(|_| Ok(vec![]));

        let snapshot = build_config_snapshot(&repo, &ctx).await.unwrap();

        assert!(snapshot.contains("Name: Test Step"));
        assert!(snapshot.contains("Task: Implement auth flow"));
        assert!(snapshot.contains("Failure Mode: fail_fast"));
        assert!(snapshot.contains("code_gen, testing"));
        assert!(snapshot.contains("Coder"));
    }

    #[tokio::test]
    async fn build_config_snapshot_without_brief() {
        let ctx = make_ctx();
        let step = make_step(ctx.step_id, ctx.workflow_id, "task_force");

        let step_id = ctx.step_id;
        let wf_id = ctx.workflow_id;
        let step_clone = step.clone();

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_step()
            .withf(move |id| *id == step_id)
            .returning(move |_| Ok(Some(step_clone.clone())));
        repo.expect_get_mission_brief().returning(|_| Ok(None));
        repo.expect_list_edges()
            .withf(move |wid| *wid == wf_id)
            .returning(|_| Ok(vec![]));

        let snapshot = build_config_snapshot(&repo, &ctx).await.unwrap();

        assert!(snapshot.contains("Task: (not set)"));
        assert!(snapshot.contains("(none)"));
    }

    // =========================================================================
    // Unknown tool
    // =========================================================================

    #[tokio::test]
    async fn unknown_tool_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let result = execute_task_force_tool("nonexistent_tool", &json!({}), &repo, &ctx).await;

        assert!(result["error"]
            .as_str()
            .unwrap()
            .contains("Unknown task force tool"));
    }
}
