#[cfg(test)]
mod tests {
    use chrono::Utc;
    use serde_json::json;
    use uuid::Uuid;

    use crate::db::traits::MockWorkflowRepo;
    use crate::db::{RoomStepConfigRow, RoomStepMemberRow, WorkflowStepEdgeRow, WorkflowStepRow};

    use super::super::{build_config_snapshot, execute_room_config_tool, RoomConfigToolContext};

    fn make_ctx() -> RoomConfigToolContext {
        RoomConfigToolContext {
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
            pinned: false,
            run_results_summary: String::new(),
        }
    }

    fn make_config(step_id: Uuid) -> RoomStepConfigRow {
        RoomStepConfigRow {
            id: Uuid::new_v4(),
            step_id,
            meeting_purpose: "Review architecture".to_string(),
            max_turns: 10,
            interaction_mode: "moderated".to_string(),
            gatekeeper_enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn make_member(step_id: Uuid, name: &str, role: &str, order: i32) -> RoomStepMemberRow {
        RoomStepMemberRow {
            id: Uuid::new_v4(),
            step_id,
            name: name.to_string(),
            role: role.to_string(),
            perspective: String::new(),
            display_order: order,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    // =========================================================================
    // set_meeting_purpose
    // =========================================================================

    #[tokio::test]
    async fn set_meeting_purpose_creates_config() {
        let ctx = make_ctx();
        let step_id = ctx.step_id;

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_room_step_config().returning(|_| Ok(None));
        repo.expect_upsert_room_step_config()
            .returning(move |sid, purpose, turns, mode, gk| {
                assert_eq!(sid, step_id);
                assert_eq!(purpose, "Discuss API design");
                assert_eq!(turns, 20);
                assert_eq!(mode, "moderated");
                assert!(gk);
                Ok(RoomStepConfigRow {
                    id: Uuid::new_v4(),
                    step_id: sid,
                    meeting_purpose: purpose.to_string(),
                    max_turns: turns,
                    interaction_mode: mode.to_string(),
                    gatekeeper_enabled: gk,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                })
            });

        let input = json!({ "description": "Discuss API design" });
        let result = execute_room_config_tool("set_meeting_purpose", &input, &repo, &ctx).await;

        assert_eq!(result["meeting_purpose"], "Discuss API design");
        assert_eq!(result["step_id"], step_id.to_string());
    }

    #[tokio::test]
    async fn set_meeting_purpose_preserves_existing_fields() {
        let ctx = make_ctx();
        let existing = make_config(ctx.step_id);

        let mut repo = MockWorkflowRepo::new();
        let existing_clone = existing.clone();
        repo.expect_get_room_step_config()
            .returning(move |_| Ok(Some(existing_clone.clone())));
        repo.expect_upsert_room_step_config()
            .returning(move |_, purpose, turns, mode, gk| {
                assert_eq!(purpose, "New purpose");
                assert_eq!(turns, 10);
                assert_eq!(mode, "moderated");
                assert!(gk);
                Ok(RoomStepConfigRow {
                    id: Uuid::new_v4(),
                    step_id: Uuid::new_v4(),
                    meeting_purpose: purpose.to_string(),
                    max_turns: turns,
                    interaction_mode: mode.to_string(),
                    gatekeeper_enabled: gk,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                })
            });

        let input = json!({ "description": "New purpose" });
        let result = execute_room_config_tool("set_meeting_purpose", &input, &repo, &ctx).await;

        assert_eq!(result["meeting_purpose"], "New purpose");
    }

    #[tokio::test]
    async fn set_meeting_purpose_missing_description_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let result = execute_room_config_tool("set_meeting_purpose", &json!({}), &repo, &ctx).await;

        assert_eq!(
            result["error"].as_str().unwrap(),
            "Missing required parameter: description"
        );
    }

    // =========================================================================
    // add_member
    // =========================================================================

    #[tokio::test]
    async fn add_member_with_all_fields() {
        let ctx = make_ctx();
        let step_id = ctx.step_id;

        let mut repo = MockWorkflowRepo::new();
        repo.expect_list_room_step_members()
            .returning(|_| Ok(vec![]));
        repo.expect_add_room_step_member()
            .returning(move |sid, name, role, perspective, order| {
                assert_eq!(sid, step_id);
                assert_eq!(name, "Alice");
                assert_eq!(role, "Architect");
                assert_eq!(perspective, "Systems design");
                assert_eq!(order, 0);
                Ok(RoomStepMemberRow {
                    id: Uuid::new_v4(),
                    step_id: sid,
                    name: name.to_string(),
                    role: role.to_string(),
                    perspective: perspective.to_string(),
                    display_order: order,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                })
            });

        let input = json!({
            "name": "Alice",
            "role": "Architect",
            "perspective": "Systems design"
        });
        let result = execute_room_config_tool("add_member", &input, &repo, &ctx).await;

        assert_eq!(result["name"], "Alice");
        assert_eq!(result["role"], "Architect");
        assert_eq!(result["perspective"], "Systems design");
        assert_eq!(result["display_order"], 0);
    }

    #[tokio::test]
    async fn add_member_auto_orders() {
        let ctx = make_ctx();
        let step_id = ctx.step_id;
        let existing_member = make_member(step_id, "Alice", "Architect", 0);

        let mut repo = MockWorkflowRepo::new();
        repo.expect_list_room_step_members()
            .returning(move |_| Ok(vec![existing_member.clone()]));
        repo.expect_add_room_step_member()
            .returning(move |_, name, role, _, order| {
                assert_eq!(order, 1); // auto-incremented
                Ok(RoomStepMemberRow {
                    id: Uuid::new_v4(),
                    step_id,
                    name: name.to_string(),
                    role: role.to_string(),
                    perspective: String::new(),
                    display_order: order,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                })
            });

        let input = json!({ "name": "Bob", "role": "Reviewer" });
        let result = execute_room_config_tool("add_member", &input, &repo, &ctx).await;

        assert_eq!(result["display_order"], 1);
    }

    #[tokio::test]
    async fn add_member_missing_name_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let input = json!({ "role": "Architect" });
        let result = execute_room_config_tool("add_member", &input, &repo, &ctx).await;

        assert_eq!(
            result["error"].as_str().unwrap(),
            "Missing required parameter: name"
        );
    }

    #[tokio::test]
    async fn add_member_missing_role_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let input = json!({ "name": "Alice" });
        let result = execute_room_config_tool("add_member", &input, &repo, &ctx).await;

        assert_eq!(
            result["error"].as_str().unwrap(),
            "Missing required parameter: role"
        );
    }

    // =========================================================================
    // update_member
    // =========================================================================

    #[tokio::test]
    async fn update_member_case_insensitive_match() {
        let ctx = make_ctx();
        let member = make_member(ctx.step_id, "Alice", "Architect", 0);
        let member_id = member.id;

        let mut repo = MockWorkflowRepo::new();
        let member_clone = member.clone();
        repo.expect_list_room_step_members()
            .returning(move |_| Ok(vec![member_clone.clone()]));
        repo.expect_update_room_step_member()
            .returning(move |id, _, role, _| {
                assert_eq!(id, member_id);
                Ok(RoomStepMemberRow {
                    id,
                    step_id: Uuid::new_v4(),
                    name: "Alice".to_string(),
                    role: role.unwrap_or("Architect".to_string()),
                    perspective: String::new(),
                    display_order: 0,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                })
            });

        let input = json!({ "name": "alice", "role": "Lead Architect" });
        let result = execute_room_config_tool("update_member", &input, &repo, &ctx).await;

        assert_eq!(result["name"], "Alice");
        assert_eq!(result["role"], "Lead Architect");
    }

    #[tokio::test]
    async fn update_member_not_found_returns_error() {
        let ctx = make_ctx();

        let mut repo = MockWorkflowRepo::new();
        repo.expect_list_room_step_members()
            .returning(|_| Ok(vec![]));

        let input = json!({ "name": "NonExistent" });
        let result = execute_room_config_tool("update_member", &input, &repo, &ctx).await;

        assert!(result["error"].as_str().unwrap().contains("not found"));
    }

    #[tokio::test]
    async fn update_member_missing_name_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let result = execute_room_config_tool("update_member", &json!({}), &repo, &ctx).await;

        assert_eq!(
            result["error"].as_str().unwrap(),
            "Missing required parameter: name"
        );
    }

    // =========================================================================
    // remove_member
    // =========================================================================

    #[tokio::test]
    async fn remove_member_success() {
        let ctx = make_ctx();
        let member = make_member(ctx.step_id, "Alice", "Architect", 0);
        let member_id = member.id;

        let mut repo = MockWorkflowRepo::new();
        let member_clone = member.clone();
        repo.expect_list_room_step_members()
            .returning(move |_| Ok(vec![member_clone.clone()]));
        repo.expect_remove_room_step_member()
            .withf(move |id| *id == member_id)
            .returning(|_| Ok(()));

        let input = json!({ "name": "Alice" });
        let result = execute_room_config_tool("remove_member", &input, &repo, &ctx).await;

        assert_eq!(result["removed"], "Alice");
    }

    #[tokio::test]
    async fn remove_member_not_found_returns_error() {
        let ctx = make_ctx();

        let mut repo = MockWorkflowRepo::new();
        repo.expect_list_room_step_members()
            .returning(|_| Ok(vec![]));

        let input = json!({ "name": "Ghost" });
        let result = execute_room_config_tool("remove_member", &input, &repo, &ctx).await;

        assert!(result["error"].as_str().unwrap().contains("not found"));
    }

    #[tokio::test]
    async fn remove_member_missing_name_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let result = execute_room_config_tool("remove_member", &json!({}), &repo, &ctx).await;

        assert_eq!(
            result["error"].as_str().unwrap(),
            "Missing required parameter: name"
        );
    }

    // =========================================================================
    // set_max_turns
    // =========================================================================

    #[tokio::test]
    async fn set_max_turns_valid() {
        let ctx = make_ctx();

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_room_step_config().returning(|_| Ok(None));
        repo.expect_upsert_room_step_config()
            .returning(|_, purpose, turns, mode, gk| {
                Ok(RoomStepConfigRow {
                    id: Uuid::new_v4(),
                    step_id: Uuid::new_v4(),
                    meeting_purpose: purpose.to_string(),
                    max_turns: turns,
                    interaction_mode: mode.to_string(),
                    gatekeeper_enabled: gk,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                })
            });

        let input = json!({ "count": 50 });
        let result = execute_room_config_tool("set_max_turns", &input, &repo, &ctx).await;

        assert_eq!(result["max_turns"], 50);
    }

    #[tokio::test]
    async fn set_max_turns_zero_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let input = json!({ "count": 0 });
        let result = execute_room_config_tool("set_max_turns", &input, &repo, &ctx).await;

        assert!(result["error"]
            .as_str()
            .unwrap()
            .contains("between 1 and 100"));
    }

    #[tokio::test]
    async fn set_max_turns_over_100_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let input = json!({ "count": 101 });
        let result = execute_room_config_tool("set_max_turns", &input, &repo, &ctx).await;

        assert!(result["error"]
            .as_str()
            .unwrap()
            .contains("between 1 and 100"));
    }

    #[tokio::test]
    async fn set_max_turns_missing_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let result = execute_room_config_tool("set_max_turns", &json!({}), &repo, &ctx).await;

        assert_eq!(
            result["error"].as_str().unwrap(),
            "Missing required parameter: count"
        );
    }

    // =========================================================================
    // set_interaction_mode
    // =========================================================================

    #[tokio::test]
    async fn set_interaction_mode_all_valid_modes() {
        for (mode, expect_gk) in &[
            ("round_robin", false),
            ("moderated", true),
            ("open_floor", true),
        ] {
            let ctx = make_ctx();

            let mut repo = MockWorkflowRepo::new();
            repo.expect_get_room_step_config().returning(|_| Ok(None));
            repo.expect_upsert_room_step_config()
                .returning(|_, purpose, turns, mode, gk| {
                    Ok(RoomStepConfigRow {
                        id: Uuid::new_v4(),
                        step_id: Uuid::new_v4(),
                        meeting_purpose: purpose.to_string(),
                        max_turns: turns,
                        interaction_mode: mode.to_string(),
                        gatekeeper_enabled: gk,
                        created_at: Utc::now(),
                        updated_at: Utc::now(),
                    })
                });

            let input = json!({ "mode": *mode });
            let result =
                execute_room_config_tool("set_interaction_mode", &input, &repo, &ctx).await;

            assert_eq!(result["interaction_mode"], *mode, "mode={mode}");
            assert_eq!(
                result["gatekeeper_enabled"], *expect_gk,
                "gatekeeper for mode={mode}"
            );
        }
    }

    #[tokio::test]
    async fn set_interaction_mode_invalid_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let input = json!({ "mode": "chaos" });
        let result = execute_room_config_tool("set_interaction_mode", &input, &repo, &ctx).await;

        let err = result["error"].as_str().unwrap();
        assert!(err.contains("Invalid interaction mode"));
        assert!(err.contains("chaos"));
    }

    #[tokio::test]
    async fn set_interaction_mode_missing_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let result =
            execute_room_config_tool("set_interaction_mode", &json!({}), &repo, &ctx).await;

        assert_eq!(
            result["error"].as_str().unwrap(),
            "Missing required parameter: mode"
        );
    }

    // =========================================================================
    // build_config_snapshot
    // =========================================================================

    #[tokio::test]
    async fn build_config_snapshot_with_members_and_config() {
        let ctx = make_ctx();
        let step = make_step(ctx.step_id, ctx.workflow_id, "room");
        let config = make_config(ctx.step_id);
        let member = make_member(ctx.step_id, "Alice", "Architect", 0);

        let step_id = ctx.step_id;
        let wf_id = ctx.workflow_id;
        let step_clone = step.clone();

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_step()
            .withf(move |id| *id == step_id)
            .returning(move |_| Ok(Some(step_clone.clone())));
        repo.expect_get_room_step_config()
            .returning(move |_| Ok(Some(config.clone())));
        repo.expect_list_room_step_members()
            .returning(move |_| Ok(vec![member.clone()]));
        repo.expect_list_edges()
            .withf(move |wid| *wid == wf_id)
            .returning(|_| Ok(vec![]));

        let snapshot = build_config_snapshot(&repo, &ctx).await.unwrap();

        assert!(snapshot.contains("Name: Test Step"));
        assert!(snapshot.contains("Meeting Purpose: Review architecture"));
        assert!(snapshot.contains("Max Turns: 10"));
        assert!(snapshot.contains("Interaction Mode: moderated"));
        assert!(snapshot.contains("Gatekeeper: enabled"));
        assert!(snapshot.contains("Alice"));
        assert!(snapshot.contains("Architect"));
    }

    #[tokio::test]
    async fn build_config_snapshot_without_config() {
        let ctx = make_ctx();
        let step = make_step(ctx.step_id, ctx.workflow_id, "room");

        let step_id = ctx.step_id;
        let wf_id = ctx.workflow_id;
        let step_clone = step.clone();

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_step()
            .withf(move |id| *id == step_id)
            .returning(move |_| Ok(Some(step_clone.clone())));
        repo.expect_get_room_step_config().returning(|_| Ok(None));
        repo.expect_list_room_step_members()
            .returning(|_| Ok(vec![]));
        repo.expect_list_edges()
            .withf(move |wid| *wid == wf_id)
            .returning(|_| Ok(vec![]));

        let snapshot = build_config_snapshot(&repo, &ctx).await.unwrap();

        assert!(snapshot.contains("Meeting Purpose: (not set)"));
        assert!(snapshot.contains("Max Turns: 20"));
        assert!(snapshot.contains("(none)"));
    }

    // =========================================================================
    // Unknown tool
    // =========================================================================

    #[tokio::test]
    async fn unknown_tool_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let result = execute_room_config_tool("nonexistent_tool", &json!({}), &repo, &ctx).await;

        assert!(result["error"]
            .as_str()
            .unwrap()
            .contains("Unknown room config tool"));
    }
}
