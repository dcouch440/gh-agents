#[cfg(test)]
mod tests {
    use chrono::Utc;
    use serde_json::json;
    use uuid::Uuid;

    use crate::db::traits::MockWorkflowRepo;
    use crate::db::{BeliefExtractionPlanRow, WorkflowStepEdgeRow, WorkflowStepRow};

    use super::super::{
        build_config_snapshot, execute_belief_capture_tool, BeliefCaptureToolContext,
    };

    fn make_ctx() -> BeliefCaptureToolContext {
        BeliefCaptureToolContext {
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

    fn make_plan(step_id: Uuid) -> BeliefExtractionPlanRow {
        BeliefExtractionPlanRow {
            id: Uuid::new_v4(),
            step_id,
            extraction_focus: "Key decisions".to_string(),
            tag_vocabulary: vec!["decision".to_string(), "risk".to_string()],
            contradiction_handling: "flag".to_string(),
            confidence_threshold: "medium".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    // =========================================================================
    // set_extraction_focus
    // =========================================================================

    #[tokio::test]
    async fn set_extraction_focus_creates_plan() {
        let ctx = make_ctx();
        let step_id = ctx.step_id;

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_extraction_plan().returning(|_| Ok(None));
        repo.expect_upsert_extraction_plan()
            .returning(move |sid, focus, tags, contra, conf| {
                assert_eq!(sid, step_id);
                assert_eq!(focus, "Extract key decisions");
                assert_eq!(tags, &[] as &[String]);
                assert_eq!(contra, "flag");
                assert_eq!(conf, "low");
                Ok(BeliefExtractionPlanRow {
                    id: Uuid::new_v4(),
                    step_id: sid,
                    extraction_focus: focus.to_string(),
                    tag_vocabulary: tags.to_vec(),
                    contradiction_handling: contra.to_string(),
                    confidence_threshold: conf.to_string(),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                })
            });

        let input = json!({ "guidance": "Extract key decisions" });
        let result = execute_belief_capture_tool("set_extraction_focus", &input, &repo, &ctx).await;

        assert_eq!(result["extraction_focus"], "Extract key decisions");
        assert_eq!(result["step_id"], step_id.to_string());
    }

    #[tokio::test]
    async fn set_extraction_focus_preserves_existing_fields() {
        let ctx = make_ctx();
        let existing = make_plan(ctx.step_id);

        let mut repo = MockWorkflowRepo::new();
        let existing_clone = existing.clone();
        repo.expect_get_extraction_plan()
            .returning(move |_| Ok(Some(existing_clone.clone())));
        repo.expect_upsert_extraction_plan()
            .returning(move |_, focus, tags, contra, conf| {
                assert_eq!(focus, "New focus");
                assert_eq!(tags, &["decision".to_string(), "risk".to_string()]);
                assert_eq!(contra, "flag");
                assert_eq!(conf, "medium");
                Ok(BeliefExtractionPlanRow {
                    id: Uuid::new_v4(),
                    step_id: Uuid::new_v4(),
                    extraction_focus: focus.to_string(),
                    tag_vocabulary: tags.to_vec(),
                    contradiction_handling: contra.to_string(),
                    confidence_threshold: conf.to_string(),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                })
            });

        let input = json!({ "guidance": "New focus" });
        let result = execute_belief_capture_tool("set_extraction_focus", &input, &repo, &ctx).await;

        assert_eq!(result["extraction_focus"], "New focus");
    }

    #[tokio::test]
    async fn set_extraction_focus_missing_guidance_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let result =
            execute_belief_capture_tool("set_extraction_focus", &json!({}), &repo, &ctx).await;

        assert_eq!(
            result["error"].as_str().unwrap(),
            "Missing required parameter: guidance"
        );
    }

    // =========================================================================
    // set_tag_vocabulary
    // =========================================================================

    #[tokio::test]
    async fn set_tag_vocabulary_creates_plan() {
        let ctx = make_ctx();

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_extraction_plan().returning(|_| Ok(None));
        repo.expect_upsert_extraction_plan()
            .returning(move |_, focus, tags, contra, conf| {
                assert_eq!(focus, "");
                assert_eq!(tags, &["arch".to_string(), "perf".to_string()]);
                assert_eq!(contra, "flag");
                assert_eq!(conf, "low");
                Ok(BeliefExtractionPlanRow {
                    id: Uuid::new_v4(),
                    step_id: Uuid::new_v4(),
                    extraction_focus: focus.to_string(),
                    tag_vocabulary: tags.to_vec(),
                    contradiction_handling: contra.to_string(),
                    confidence_threshold: conf.to_string(),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                })
            });

        let input = json!({ "tags": ["arch", "perf"] });
        let result = execute_belief_capture_tool("set_tag_vocabulary", &input, &repo, &ctx).await;

        let tags = result["tag_vocabulary"].as_array().unwrap();
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0], "arch");
    }

    #[tokio::test]
    async fn set_tag_vocabulary_preserves_existing_fields() {
        let ctx = make_ctx();
        let existing = make_plan(ctx.step_id);

        let mut repo = MockWorkflowRepo::new();
        let existing_clone = existing.clone();
        repo.expect_get_extraction_plan()
            .returning(move |_| Ok(Some(existing_clone.clone())));
        repo.expect_upsert_extraction_plan()
            .returning(move |_, focus, tags, contra, conf| {
                assert_eq!(focus, "Key decisions");
                assert_eq!(contra, "flag");
                assert_eq!(conf, "medium");
                Ok(BeliefExtractionPlanRow {
                    id: Uuid::new_v4(),
                    step_id: Uuid::new_v4(),
                    extraction_focus: focus.to_string(),
                    tag_vocabulary: tags.to_vec(),
                    contradiction_handling: contra.to_string(),
                    confidence_threshold: conf.to_string(),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                })
            });

        let input = json!({ "tags": ["new_tag"] });
        let result = execute_belief_capture_tool("set_tag_vocabulary", &input, &repo, &ctx).await;

        let tags = result["tag_vocabulary"].as_array().unwrap();
        assert_eq!(tags[0], "new_tag");
    }

    #[tokio::test]
    async fn set_tag_vocabulary_missing_tags_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let result =
            execute_belief_capture_tool("set_tag_vocabulary", &json!({}), &repo, &ctx).await;

        assert!(result["error"].as_str().unwrap().contains("tags"));
    }

    // =========================================================================
    // set_contradiction_handling
    // =========================================================================

    #[tokio::test]
    async fn set_contradiction_handling_valid_modes() {
        for mode in &["flag", "resolve", "keep_both"] {
            let ctx = make_ctx();
            let mode_str = mode.to_string();

            let mut repo = MockWorkflowRepo::new();
            repo.expect_get_extraction_plan().returning(|_| Ok(None));
            let expected = mode_str.clone();
            repo.expect_upsert_extraction_plan()
                .returning(move |_, _, _, contra, _| {
                    Ok(BeliefExtractionPlanRow {
                        id: Uuid::new_v4(),
                        step_id: Uuid::new_v4(),
                        extraction_focus: String::new(),
                        tag_vocabulary: vec![],
                        contradiction_handling: contra.to_string(),
                        confidence_threshold: "low".to_string(),
                        created_at: Utc::now(),
                        updated_at: Utc::now(),
                    })
                });

            let input = json!({ "mode": *mode });
            let result =
                execute_belief_capture_tool("set_contradiction_handling", &input, &repo, &ctx)
                    .await;

            assert_eq!(result["contradiction_handling"], expected, "mode={mode}");
        }
    }

    #[tokio::test]
    async fn set_contradiction_handling_invalid_mode_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let input = json!({ "mode": "ignore" });
        let result =
            execute_belief_capture_tool("set_contradiction_handling", &input, &repo, &ctx).await;

        let err = result["error"].as_str().unwrap();
        assert!(err.contains("Invalid contradiction handling mode"));
        assert!(err.contains("ignore"));
    }

    #[tokio::test]
    async fn set_contradiction_handling_missing_mode_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let result =
            execute_belief_capture_tool("set_contradiction_handling", &json!({}), &repo, &ctx)
                .await;

        assert_eq!(
            result["error"].as_str().unwrap(),
            "Missing required parameter: mode"
        );
    }

    // =========================================================================
    // set_confidence_threshold
    // =========================================================================

    #[tokio::test]
    async fn set_confidence_threshold_valid_values() {
        for threshold in &["low", "medium", "high"] {
            let ctx = make_ctx();
            let expected = threshold.to_string();

            let mut repo = MockWorkflowRepo::new();
            repo.expect_get_extraction_plan().returning(|_| Ok(None));
            repo.expect_upsert_extraction_plan()
                .returning(move |_, _, _, _, conf| {
                    Ok(BeliefExtractionPlanRow {
                        id: Uuid::new_v4(),
                        step_id: Uuid::new_v4(),
                        extraction_focus: String::new(),
                        tag_vocabulary: vec![],
                        contradiction_handling: "flag".to_string(),
                        confidence_threshold: conf.to_string(),
                        created_at: Utc::now(),
                        updated_at: Utc::now(),
                    })
                });

            let input = json!({ "threshold": *threshold });
            let result =
                execute_belief_capture_tool("set_confidence_threshold", &input, &repo, &ctx).await;

            assert_eq!(
                result["confidence_threshold"], expected,
                "threshold={threshold}"
            );
        }
    }

    #[tokio::test]
    async fn set_confidence_threshold_invalid_value_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let input = json!({ "threshold": "ultra" });
        let result =
            execute_belief_capture_tool("set_confidence_threshold", &input, &repo, &ctx).await;

        let err = result["error"].as_str().unwrap();
        assert!(err.contains("Invalid confidence threshold"));
        assert!(err.contains("ultra"));
    }

    #[tokio::test]
    async fn set_confidence_threshold_missing_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let result =
            execute_belief_capture_tool("set_confidence_threshold", &json!({}), &repo, &ctx).await;

        assert_eq!(
            result["error"].as_str().unwrap(),
            "Missing required parameter: threshold"
        );
    }

    // =========================================================================
    // build_config_snapshot
    // =========================================================================

    #[tokio::test]
    async fn build_config_snapshot_with_plan() {
        let ctx = make_ctx();
        let step = make_step(ctx.step_id, ctx.workflow_id, "belief_capture");
        let plan = make_plan(ctx.step_id);

        let step_id = ctx.step_id;
        let wf_id = ctx.workflow_id;
        let step_clone = step.clone();

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_step()
            .withf(move |id| *id == step_id)
            .returning(move |_| Ok(Some(step_clone.clone())));
        repo.expect_get_extraction_plan()
            .returning(move |_| Ok(Some(plan.clone())));
        repo.expect_list_edges()
            .withf(move |wid| *wid == wf_id)
            .returning(|_| Ok(vec![]));

        let snapshot = build_config_snapshot(&repo, &ctx).await.unwrap();

        assert!(snapshot.contains("Name: Test Step"));
        assert!(snapshot.contains("Extraction Focus: Key decisions"));
        assert!(snapshot.contains("decision, risk"));
        assert!(snapshot.contains("Contradiction Handling: flag"));
        assert!(snapshot.contains("Confidence Threshold: medium"));
    }

    #[tokio::test]
    async fn build_config_snapshot_without_plan() {
        let ctx = make_ctx();
        let step = make_step(ctx.step_id, ctx.workflow_id, "belief_capture");

        let step_id = ctx.step_id;
        let wf_id = ctx.workflow_id;
        let step_clone = step.clone();

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_step()
            .withf(move |id| *id == step_id)
            .returning(move |_| Ok(Some(step_clone.clone())));
        repo.expect_get_extraction_plan().returning(|_| Ok(None));
        repo.expect_list_edges()
            .withf(move |wid| *wid == wf_id)
            .returning(|_| Ok(vec![]));

        let snapshot = build_config_snapshot(&repo, &ctx).await.unwrap();

        assert!(snapshot.contains("Extraction Focus: (not set)"));
        assert!(snapshot.contains("Tag Vocabulary: (not set)"));
        assert!(snapshot.contains("Contradiction Handling: flag"));
        assert!(snapshot.contains("Confidence Threshold: low"));
    }

    #[tokio::test]
    async fn build_config_snapshot_with_upstream_context() {
        let ctx = make_ctx();
        let upstream_id = Uuid::new_v4();

        let step = make_step(ctx.step_id, ctx.workflow_id, "belief_capture");
        let mut upstream = make_step(upstream_id, ctx.workflow_id, "context");
        upstream.prompt_template = "Some context content".to_string();
        upstream.name = Some("Research Notes".to_string());
        upstream.description = "Gathered research".to_string();

        let step_id = ctx.step_id;
        let wf_id = ctx.workflow_id;
        let step_clone = step.clone();
        let upstream_clone = upstream.clone();

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_step().returning(move |id| {
            if id == step_id {
                Ok(Some(step_clone.clone()))
            } else if id == upstream_id {
                Ok(Some(upstream_clone.clone()))
            } else {
                Ok(None)
            }
        });
        repo.expect_get_extraction_plan().returning(|_| Ok(None));
        repo.expect_list_edges()
            .withf(move |wid| *wid == wf_id)
            .returning(move |_| {
                Ok(vec![WorkflowStepEdgeRow {
                    id: Uuid::new_v4(),
                    from_step_id: upstream_id,
                    to_step_id: step_id,
                    from_output_port: None,
                    to_input_port: None,
                    transform_jsonpath: None,
                    condition_type: None,
                    condition_value: None,
                    edge_label: None,
                    workflow_id: wf_id,
                }])
            });

        let snapshot = build_config_snapshot(&repo, &ctx).await.unwrap();

        assert!(snapshot.contains("Research Notes (context) — populated"));
        assert!(snapshot.contains("Gathered research"));
    }

    // =========================================================================
    // Unknown tool
    // =========================================================================

    #[tokio::test]
    async fn unknown_tool_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let result = execute_belief_capture_tool("nonexistent_tool", &json!({}), &repo, &ctx).await;

        assert!(result["error"]
            .as_str()
            .unwrap()
            .contains("Unknown belief capture tool"));
    }
}
