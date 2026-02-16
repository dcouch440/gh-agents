#[cfg(test)]
mod tests {
    use chrono::Utc;
    use serde_json::json;
    use uuid::Uuid;

    use crate::db::traits::MockWorkflowRepo;
    use crate::db::{
        ProtocolDocumentDefRow, TaskAgentRosterRow, TaskMissionBriefRow, WorkflowRow,
        WorkflowStepEdgeRow, WorkflowStepRow,
    };

    use super::super::{
        build_config_snapshot, execute_workforce_tool, WorkforceToolContext, VALID_FAILURE_MODES,
    };

    fn make_ctx() -> WorkforceToolContext {
        WorkforceToolContext {
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
            name: Some("Test Workforce".to_string()),
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

    fn make_workflow(id: Uuid, user_id: Uuid) -> WorkflowRow {
        WorkflowRow {
            id,
            user_id,
            name: "Parent Workflow".to_string(),
            description: String::new(),
            execution_mode: "sequential".to_string(),
            created_at: Utc::now(),
            version: 1,
            container_enabled: false,
            target_repo_url: None,
            target_branch: None,
            vpn_enabled: false,
            board_overview_summary: String::new(),
        }
    }

    fn make_brief(step_id: Uuid) -> TaskMissionBriefRow {
        TaskMissionBriefRow {
            id: Uuid::new_v4(),
            step_id,
            task_description: "Build the system".to_string(),
            available_capabilities: vec!["code_gen".to_string()],
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
            child_step_id: None,
        }
    }

    fn make_doc_def(step_id: Uuid, name: &str) -> ProtocolDocumentDefRow {
        ProtocolDocumentDefRow {
            id: Uuid::new_v4(),
            step_id: Some(step_id),
            name: name.to_string(),
            description: String::new(),
            target_length: 1500,
            display_order: 0,
            created_at: Utc::now(),
            protocol_id: None,
            document_id: None,
            agent_roster_entry_id: None,
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
        let result = execute_workforce_tool("set_task", &input, &repo, &ctx).await;

        assert_eq!(result["task_description"], "Build login page");
        assert_eq!(result["step_id"], step_id.to_string());
    }

    #[tokio::test]
    async fn set_task_missing_description_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let result = execute_workforce_tool("set_task", &json!({}), &repo, &ctx).await;

        assert_eq!(
            result["error"].as_str().unwrap(),
            "Missing required parameter: description"
        );
    }

    // =========================================================================
    // add_agent (workforce-specific: creates child step + Designer)
    // =========================================================================

    #[tokio::test]
    async fn add_agent_first_agent_creates_designer_and_child_step() {
        let ctx = make_ctx();
        let brief = make_brief(ctx.step_id);
        let brief_id = brief.id;
        let user_id = Uuid::new_v4();
        let child_wf_id = Uuid::new_v4();
        let step_id = ctx.step_id;
        let wf_id = ctx.workflow_id;

        let step = make_step(step_id, wf_id, "workforce");
        let parent_wf = make_workflow(wf_id, user_id);

        let mut repo = MockWorkflowRepo::new();

        // ensure_mission_brief
        let brief_clone = brief.clone();
        repo.expect_get_mission_brief()
            .returning(move |_| Ok(Some(brief_clone.clone())));

        // ensure_child_workflow: get_step returns step without child_workflow_id
        let step_clone = step.clone();
        repo.expect_get_step()
            .returning(move |_| Ok(Some(step_clone.clone())));
        let wf_clone = parent_wf.clone();
        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(wf_clone.clone())));
        repo.expect_create_workflow()
            .returning(move |uid, name, _, _, _, _, _| {
                assert_eq!(uid, user_id);
                assert!(name.contains("child"));
                Ok(WorkflowRow {
                    id: child_wf_id,
                    user_id: uid,
                    name,
                    description: String::new(),
                    execution_mode: "sequential".to_string(),
                    created_at: Utc::now(),
                    version: 1,
                    container_enabled: false,
                    target_repo_url: None,
                    target_branch: None,
                    vpn_enabled: false,
                    board_overview_summary: String::new(),
                })
            });
        repo.expect_update_step().returning(|s| Ok(s));

        // list_agent_roster: empty (first agent)
        repo.expect_list_agent_roster().returning(|_| Ok(vec![]));

        // create_designer_step + create agent step
        repo.expect_create_step().times(2).returning(|s| Ok(s));

        // Wire Designer → agent edge
        repo.expect_add_edge().returning(|wid, from, to| {
            Ok(WorkflowStepEdgeRow {
                id: Uuid::new_v4(),
                from_step_id: from,
                to_step_id: to,
                from_output_port: None,
                to_input_port: None,
                transform_jsonpath: None,
                condition_type: None,
                condition_value: None,
                edge_label: None,
                workflow_id: wid,
            })
        });

        // add_roster_agent
        repo.expect_add_roster_agent()
            .returning(move |bid, name, role, caps, order| {
                assert_eq!(bid, brief_id);
                assert_eq!(name, "Scanner");
                assert_eq!(order, 0);
                Ok(TaskAgentRosterRow {
                    id: Uuid::new_v4(),
                    mission_brief_id: bid,
                    name: name.to_string(),
                    role_description: role.to_string(),
                    capabilities: caps.to_vec(),
                    execution_order: order,
                    created_at: Utc::now(),
                    child_step_id: None,
                })
            });

        // link_roster_agent_to_child_step
        repo.expect_link_roster_agent_to_child_step()
            .returning(|_, _| Ok(()));

        let input = json!({
            "name": "Scanner",
            "role": "Scan codebase",
            "capabilities": ["file_read"]
        });
        let result = execute_workforce_tool("add_agent", &input, &repo, &ctx).await;

        assert_eq!(result["name"], "Scanner");
        assert_eq!(result["role"], "Scan codebase");
        assert_eq!(result["execution_order"], 0);
        assert!(result["child_step_id"].is_string());
    }

    #[tokio::test]
    async fn add_agent_subsequent_agent_fans_out_from_designer() {
        let ctx = make_ctx();
        let brief = make_brief(ctx.step_id);
        let brief_id = brief.id;
        let child_wf_id = Uuid::new_v4();
        let designer_step_id = Uuid::new_v4();
        let prev_child_step_id = Uuid::new_v4();
        let step_id = ctx.step_id;
        let wf_id = ctx.workflow_id;

        // Step already has child_workflow_id
        let mut step = make_step(step_id, wf_id, "workforce");
        step.child_workflow_id = Some(child_wf_id);

        // Existing roster has one agent with a child_step_id
        let mut existing_agent = make_roster_agent(brief_id, "Agent1", 0);
        existing_agent.child_step_id = Some(prev_child_step_id);

        // Designer step exists in child workflow
        let mut designer = make_step(designer_step_id, child_wf_id, "single");
        designer.is_designer_step = true;

        let mut repo = MockWorkflowRepo::new();

        let brief_clone = brief.clone();
        repo.expect_get_mission_brief()
            .returning(move |_| Ok(Some(brief_clone.clone())));

        let step_clone = step.clone();
        repo.expect_get_step()
            .returning(move |_| Ok(Some(step_clone.clone())));

        let existing_agent_clone = existing_agent.clone();
        repo.expect_list_agent_roster()
            .returning(move |_| Ok(vec![existing_agent_clone.clone()]));

        // list_steps returns Designer + existing agent step
        let designer_clone = designer.clone();
        repo.expect_list_steps()
            .returning(move |_| Ok(vec![designer_clone.clone()]));

        // create agent step (no Designer creation this time)
        repo.expect_create_step().times(1).returning(|s| Ok(s));

        // Wire from Designer (fan-out), NOT from previous agent
        repo.expect_add_edge()
            .withf(move |_, from, _| *from == designer_step_id)
            .returning(|wid, from, to| {
                Ok(WorkflowStepEdgeRow {
                    id: Uuid::new_v4(),
                    from_step_id: from,
                    to_step_id: to,
                    from_output_port: None,
                    to_input_port: None,
                    transform_jsonpath: None,
                    condition_type: None,
                    condition_value: None,
                    edge_label: None,
                    workflow_id: wid,
                })
            });

        repo.expect_add_roster_agent()
            .returning(move |bid, name, role, caps, order| {
                assert_eq!(order, 1);
                Ok(TaskAgentRosterRow {
                    id: Uuid::new_v4(),
                    mission_brief_id: bid,
                    name: name.to_string(),
                    role_description: role.to_string(),
                    capabilities: caps.to_vec(),
                    execution_order: order,
                    created_at: Utc::now(),
                    child_step_id: None,
                })
            });

        repo.expect_link_roster_agent_to_child_step()
            .returning(|_, _| Ok(()));

        let input = json!({ "name": "Agent2", "role": "Writer" });
        let result = execute_workforce_tool("add_agent", &input, &repo, &ctx).await;

        assert_eq!(result["name"], "Agent2");
        assert_eq!(result["execution_order"], 1);
    }

    #[tokio::test]
    async fn add_agent_missing_name_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let input = json!({ "role": "Worker" });
        let result = execute_workforce_tool("add_agent", &input, &repo, &ctx).await;

        assert_eq!(
            result["error"].as_str().unwrap(),
            "Missing required parameter: name"
        );
    }

    // =========================================================================
    // update_agent
    // =========================================================================

    #[tokio::test]
    async fn update_agent_updates_roster_and_child_step() {
        let ctx = make_ctx();
        let agent_id = Uuid::new_v4();
        let child_step_id = Uuid::new_v4();
        let wf_id = ctx.workflow_id;

        let mut repo = MockWorkflowRepo::new();

        let child_step = make_step(child_step_id, wf_id, "single");
        let child_step_clone = child_step.clone();

        repo.expect_update_roster_agent()
            .returning(move |id, name, role, _caps| {
                assert_eq!(id, agent_id);
                Ok(TaskAgentRosterRow {
                    id,
                    mission_brief_id: Uuid::new_v4(),
                    name: name.unwrap_or_else(|| "Coder".to_string()),
                    role_description: role.unwrap_or_else(|| "Worker".to_string()),
                    capabilities: vec![],
                    execution_order: 0,
                    created_at: Utc::now(),
                    child_step_id: Some(child_step_id),
                })
            });

        repo.expect_get_step()
            .returning(move |_| Ok(Some(child_step_clone.clone())));

        repo.expect_update_step().returning(|s| {
            assert_eq!(s.name, Some("Renamed Agent".to_string()));
            assert_eq!(s.description, "New role");
            Ok(s)
        });

        let input = json!({
            "agent_id": agent_id.to_string(),
            "name": "Renamed Agent",
            "role": "New role"
        });
        let result = execute_workforce_tool("update_agent", &input, &repo, &ctx).await;

        assert_eq!(result["name"], "Renamed Agent");
        assert_eq!(result["role"], "New role");
    }

    #[tokio::test]
    async fn update_agent_missing_id_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let input = json!({ "name": "Test" });
        let result = execute_workforce_tool("update_agent", &input, &repo, &ctx).await;

        assert_eq!(
            result["error"].as_str().unwrap(),
            "Missing required parameter: agent_id"
        );
    }

    // =========================================================================
    // remove_agent
    // =========================================================================

    #[tokio::test]
    async fn remove_agent_removes_edges_without_bridging() {
        let ctx = make_ctx();
        let brief = make_brief(ctx.step_id);
        let brief_id = brief.id;
        let child_wf_id = Uuid::new_v4();
        let step_id = ctx.step_id;
        let wf_id = ctx.workflow_id;

        let designer_id = Uuid::new_v4();
        let agent1_step = Uuid::new_v4();

        let mut agent1 = make_roster_agent(brief_id, "Agent1", 0);
        agent1.child_step_id = Some(agent1_step);
        let agent1_id = agent1.id;

        let agent2 = make_roster_agent(brief_id, "Agent2", 1);

        let mut parent_step = make_step(step_id, wf_id, "workforce");
        parent_step.child_workflow_id = Some(child_wf_id);

        let mut repo = MockWorkflowRepo::new();

        let brief_clone = brief.clone();
        repo.expect_get_mission_brief()
            .returning(move |_| Ok(Some(brief_clone.clone())));

        let agent1_clone = agent1.clone();
        let agent2_clone = agent2.clone();
        repo.expect_list_agent_roster()
            .returning(move |_| Ok(vec![agent1_clone.clone(), agent2_clone.clone()]));

        let parent_step_clone = parent_step.clone();
        repo.expect_get_step()
            .returning(move |_| Ok(Some(parent_step_clone.clone())));

        // Edges: Designer → Agent1 (fan-out)
        repo.expect_list_edges().returning(move |_| {
            Ok(vec![WorkflowStepEdgeRow {
                id: Uuid::new_v4(),
                from_step_id: designer_id,
                to_step_id: agent1_step,
                from_output_port: None,
                to_input_port: None,
                transform_jsonpath: None,
                condition_type: None,
                condition_value: None,
                edge_label: None,
                workflow_id: child_wf_id,
            }])
        });

        // Remove edges only — no add_edge (no bridging)
        repo.expect_remove_edge().returning(|_, _| Ok(()));
        repo.expect_delete_step().returning(|_| Ok(()));
        repo.expect_remove_roster_agent()
            .withf(move |id| *id == agent1_id)
            .returning(|_| Ok(()));

        let input = json!({ "agent_id": agent1_id.to_string() });
        let result = execute_workforce_tool("remove_agent", &input, &repo, &ctx).await;

        assert_eq!(result["deleted"], true);
        assert_eq!(result["name"], "Agent1");
    }

    #[tokio::test]
    async fn remove_agent_missing_id_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let result = execute_workforce_tool("remove_agent", &json!({}), &repo, &ctx).await;

        assert_eq!(
            result["error"].as_str().unwrap(),
            "Missing required parameter: agent_id"
        );
    }

    // =========================================================================
    // set_dependency
    // =========================================================================

    #[tokio::test]
    async fn set_dependency_creates_edge() {
        let ctx = make_ctx();
        let brief = make_brief(ctx.step_id);
        let brief_id = brief.id;
        let child_wf_id = Uuid::new_v4();
        let scanner_child = Uuid::new_v4();
        let analyzer_child = Uuid::new_v4();

        let mut scanner = make_roster_agent(brief_id, "Scanner", 0);
        scanner.child_step_id = Some(scanner_child);
        let mut analyzer = make_roster_agent(brief_id, "Analyzer", 1);
        analyzer.child_step_id = Some(analyzer_child);

        let mut parent_step = make_step(ctx.step_id, ctx.workflow_id, "workforce");
        parent_step.child_workflow_id = Some(child_wf_id);

        let mut repo = MockWorkflowRepo::new();

        let brief_clone = brief.clone();
        repo.expect_get_mission_brief()
            .returning(move |_| Ok(Some(brief_clone.clone())));

        let scanner_clone = scanner.clone();
        let analyzer_clone = analyzer.clone();
        repo.expect_list_agent_roster()
            .returning(move |_| Ok(vec![scanner_clone.clone(), analyzer_clone.clone()]));

        let parent_step_clone = parent_step.clone();
        repo.expect_get_step()
            .returning(move |_| Ok(Some(parent_step_clone.clone())));

        // No existing edges between agents
        repo.expect_list_edges().returning(|_| Ok(vec![]));

        // Expect edge creation: Scanner → Analyzer
        repo.expect_add_edge()
            .withf(move |_, from, to| *from == scanner_child && *to == analyzer_child)
            .returning(|wid, from, to| {
                Ok(WorkflowStepEdgeRow {
                    id: Uuid::new_v4(),
                    from_step_id: from,
                    to_step_id: to,
                    from_output_port: None,
                    to_input_port: None,
                    transform_jsonpath: None,
                    condition_type: None,
                    condition_value: None,
                    edge_label: None,
                    workflow_id: wid,
                })
            });

        let input = json!({ "from_agent": "Scanner", "to_agent": "Analyzer" });
        let result = execute_workforce_tool("set_dependency", &input, &repo, &ctx).await;

        assert_eq!(result["created"], true);
        assert_eq!(result["from"], "Scanner");
        assert_eq!(result["to"], "Analyzer");
    }

    #[tokio::test]
    async fn set_dependency_self_edge_rejected() {
        let ctx = make_ctx();
        let brief = make_brief(ctx.step_id);
        let brief_id = brief.id;

        let mut scanner = make_roster_agent(brief_id, "Scanner", 0);
        scanner.child_step_id = Some(Uuid::new_v4());

        let mut repo = MockWorkflowRepo::new();

        let brief_clone = brief.clone();
        repo.expect_get_mission_brief()
            .returning(move |_| Ok(Some(brief_clone.clone())));

        let scanner_clone = scanner.clone();
        repo.expect_list_agent_roster()
            .returning(move |_| Ok(vec![scanner_clone.clone()]));

        let input = json!({ "from_agent": "Scanner", "to_agent": "Scanner" });
        let result = execute_workforce_tool("set_dependency", &input, &repo, &ctx).await;

        assert!(result["error"]
            .as_str()
            .unwrap()
            .contains("dependency from an agent to itself"));
    }

    #[tokio::test]
    async fn set_dependency_unknown_agent_returns_error() {
        let ctx = make_ctx();
        let brief = make_brief(ctx.step_id);

        let mut repo = MockWorkflowRepo::new();

        let brief_clone = brief.clone();
        repo.expect_get_mission_brief()
            .returning(move |_| Ok(Some(brief_clone.clone())));
        repo.expect_list_agent_roster().returning(|_| Ok(vec![]));

        let input = json!({ "from_agent": "Ghost", "to_agent": "Phantom" });
        let result = execute_workforce_tool("set_dependency", &input, &repo, &ctx).await;

        assert!(result["error"].as_str().unwrap().contains("not found"));
    }

    #[tokio::test]
    async fn set_dependency_duplicate_returns_already_exists() {
        let ctx = make_ctx();
        let brief = make_brief(ctx.step_id);
        let brief_id = brief.id;
        let child_wf_id = Uuid::new_v4();
        let scanner_child = Uuid::new_v4();
        let analyzer_child = Uuid::new_v4();

        let mut scanner = make_roster_agent(brief_id, "Scanner", 0);
        scanner.child_step_id = Some(scanner_child);
        let mut analyzer = make_roster_agent(brief_id, "Analyzer", 1);
        analyzer.child_step_id = Some(analyzer_child);

        let mut parent_step = make_step(ctx.step_id, ctx.workflow_id, "workforce");
        parent_step.child_workflow_id = Some(child_wf_id);

        let mut repo = MockWorkflowRepo::new();

        let brief_clone = brief.clone();
        repo.expect_get_mission_brief()
            .returning(move |_| Ok(Some(brief_clone.clone())));

        let scanner_clone = scanner.clone();
        let analyzer_clone = analyzer.clone();
        repo.expect_list_agent_roster()
            .returning(move |_| Ok(vec![scanner_clone.clone(), analyzer_clone.clone()]));

        let parent_step_clone = parent_step.clone();
        repo.expect_get_step()
            .returning(move |_| Ok(Some(parent_step_clone.clone())));

        // Edge already exists
        repo.expect_list_edges().returning(move |_| {
            Ok(vec![WorkflowStepEdgeRow {
                id: Uuid::new_v4(),
                from_step_id: scanner_child,
                to_step_id: analyzer_child,
                from_output_port: None,
                to_input_port: None,
                transform_jsonpath: None,
                condition_type: None,
                condition_value: None,
                edge_label: None,
                workflow_id: child_wf_id,
            }])
        });

        let input = json!({ "from_agent": "Scanner", "to_agent": "Analyzer" });
        let result = execute_workforce_tool("set_dependency", &input, &repo, &ctx).await;

        assert_eq!(result["already_exists"], true);
    }

    // =========================================================================
    // remove_dependency
    // =========================================================================

    #[tokio::test]
    async fn remove_dependency_removes_edge() {
        let ctx = make_ctx();
        let brief = make_brief(ctx.step_id);
        let brief_id = brief.id;
        let scanner_child = Uuid::new_v4();
        let analyzer_child = Uuid::new_v4();

        let mut scanner = make_roster_agent(brief_id, "Scanner", 0);
        scanner.child_step_id = Some(scanner_child);
        let mut analyzer = make_roster_agent(brief_id, "Analyzer", 1);
        analyzer.child_step_id = Some(analyzer_child);

        let mut repo = MockWorkflowRepo::new();

        let brief_clone = brief.clone();
        repo.expect_get_mission_brief()
            .returning(move |_| Ok(Some(brief_clone.clone())));

        let scanner_clone = scanner.clone();
        let analyzer_clone = analyzer.clone();
        repo.expect_list_agent_roster()
            .returning(move |_| Ok(vec![scanner_clone.clone(), analyzer_clone.clone()]));

        repo.expect_remove_edge()
            .withf(move |from, to| *from == scanner_child && *to == analyzer_child)
            .returning(|_, _| Ok(()));

        let input = json!({ "from_agent": "Scanner", "to_agent": "Analyzer" });
        let result = execute_workforce_tool("remove_dependency", &input, &repo, &ctx).await;

        assert_eq!(result["removed"], true);
        assert_eq!(result["from"], "Scanner");
        assert_eq!(result["to"], "Analyzer");
    }

    #[tokio::test]
    async fn remove_dependency_missing_params_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let result =
            execute_workforce_tool("remove_dependency", &json!({}), &repo, &ctx).await;

        assert!(result["error"]
            .as_str()
            .unwrap()
            .contains("Missing required parameter"));
    }

    // =========================================================================
    // set_capabilities
    // =========================================================================

    #[tokio::test]
    async fn set_capabilities_updates_brief() {
        let ctx = make_ctx();

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_mission_brief().returning(|_| Ok(None));
        repo.expect_upsert_mission_brief()
            .returning(|_, _, caps, _, _| {
                assert_eq!(caps, &["read_file".to_string(), "write_file".to_string()]);
                Ok(TaskMissionBriefRow {
                    id: Uuid::new_v4(),
                    step_id: Uuid::new_v4(),
                    task_description: String::new(),
                    available_capabilities: caps.to_vec(),
                    failure_mode: "fail_fast".to_string(),
                    downstream_context: None,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                })
            });

        let input = json!({ "capabilities": ["read_file", "write_file"] });
        let result = execute_workforce_tool("set_capabilities", &input, &repo, &ctx).await;

        let caps = result["capabilities"].as_array().unwrap();
        assert_eq!(caps.len(), 2);
    }

    // =========================================================================
    // set_failure_mode
    // =========================================================================

    #[tokio::test]
    async fn set_failure_mode_invalid_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let input = json!({ "mode": "explode" });
        let result = execute_workforce_tool("set_failure_mode", &input, &repo, &ctx).await;

        assert!(result["error"]
            .as_str()
            .unwrap()
            .contains("Invalid failure mode"));
    }

    // =========================================================================
    // add_deliverable
    // =========================================================================

    #[tokio::test]
    async fn add_deliverable_creates_def() {
        let ctx = make_ctx();
        let step_id = ctx.step_id;
        let agent_id = Uuid::new_v4();

        let mut repo = MockWorkflowRepo::new();
        repo.expect_create_document_def().returning(move |def| {
            assert_eq!(def.name, "API Docs");
            assert_eq!(def.step_id, Some(step_id));
            assert_eq!(def.agent_roster_entry_id, Some(agent_id));
            assert_eq!(def.target_length, 2000);
            Ok(def)
        });

        let input = json!({
            "name": "API Docs",
            "description": "API documentation",
            "target_length": 2000,
            "agent_id": agent_id.to_string()
        });
        let result = execute_workforce_tool("add_deliverable", &input, &repo, &ctx).await;

        assert_eq!(result["name"], "API Docs");
        assert_eq!(result["target_length"], 2000);
        assert_eq!(result["agent_id"], agent_id.to_string());
    }

    #[tokio::test]
    async fn add_deliverable_without_agent() {
        let ctx = make_ctx();

        let mut repo = MockWorkflowRepo::new();
        repo.expect_create_document_def().returning(|def| {
            assert!(def.agent_roster_entry_id.is_none());
            Ok(def)
        });

        let input = json!({ "name": "Overview" });
        let result = execute_workforce_tool("add_deliverable", &input, &repo, &ctx).await;

        assert_eq!(result["name"], "Overview");
        assert!(result["agent_id"].is_null());
    }

    #[tokio::test]
    async fn add_deliverable_missing_name_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let result = execute_workforce_tool("add_deliverable", &json!({}), &repo, &ctx).await;

        assert_eq!(
            result["error"].as_str().unwrap(),
            "Missing required parameter: name"
        );
    }

    // =========================================================================
    // update_deliverable
    // =========================================================================

    #[tokio::test]
    async fn update_deliverable_merges_fields() {
        let ctx = make_ctx();
        let def = make_doc_def(ctx.step_id, "Old Name");
        let def_id = def.id;

        let mut repo = MockWorkflowRepo::new();
        let def_clone = def.clone();
        repo.expect_list_document_defs()
            .returning(move |_| Ok(vec![def_clone.clone()]));
        repo.expect_update_document_def()
            .returning(move |id, name, desc, tl| {
                assert_eq!(id, def_id);
                assert_eq!(name, "New Name");
                assert_eq!(desc, ""); // preserved from existing (empty)
                assert_eq!(tl, 1500); // preserved from existing
                Ok(ProtocolDocumentDefRow {
                    id,
                    step_id: None,
                    name,
                    description: desc,
                    target_length: tl,
                    display_order: 0,
                    created_at: Utc::now(),
                    protocol_id: None,
                    document_id: None,
                    agent_roster_entry_id: None,
                })
            });

        let input = json!({
            "deliverable_id": def_id.to_string(),
            "name": "New Name"
        });
        let result = execute_workforce_tool("update_deliverable", &input, &repo, &ctx).await;

        assert_eq!(result["name"], "New Name");
    }

    #[tokio::test]
    async fn update_deliverable_not_found_returns_error() {
        let ctx = make_ctx();

        let mut repo = MockWorkflowRepo::new();
        repo.expect_list_document_defs().returning(|_| Ok(vec![]));

        let input = json!({ "deliverable_id": Uuid::new_v4().to_string() });
        let result = execute_workforce_tool("update_deliverable", &input, &repo, &ctx).await;

        assert_eq!(result["error"].as_str().unwrap(), "Deliverable not found");
    }

    // =========================================================================
    // remove_deliverable
    // =========================================================================

    #[tokio::test]
    async fn remove_deliverable_deletes_def() {
        let ctx = make_ctx();
        let def = make_doc_def(ctx.step_id, "To Delete");
        let def_id = def.id;

        let mut repo = MockWorkflowRepo::new();
        let def_clone = def.clone();
        repo.expect_list_document_defs()
            .returning(move |_| Ok(vec![def_clone.clone()]));
        repo.expect_delete_document_def()
            .withf(move |id| *id == def_id)
            .returning(|_| Ok(()));

        let input = json!({ "deliverable_id": def_id.to_string() });
        let result = execute_workforce_tool("remove_deliverable", &input, &repo, &ctx).await;

        assert_eq!(result["deleted"], true);
        assert_eq!(result["name"], "To Delete");
    }

    #[tokio::test]
    async fn remove_deliverable_missing_id_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let result = execute_workforce_tool("remove_deliverable", &json!({}), &repo, &ctx).await;

        assert_eq!(
            result["error"].as_str().unwrap(),
            "Missing required parameter: deliverable_id"
        );
    }

    // =========================================================================
    // build_config_snapshot
    // =========================================================================

    #[tokio::test]
    async fn config_snapshot_shows_agents_and_deliverables() {
        let ctx = make_ctx();
        let step = make_step(ctx.step_id, ctx.workflow_id, "workforce");
        let brief = make_brief(ctx.step_id);
        let brief_id = brief.id;
        let agent = make_roster_agent(brief_id, "Scanner", 0);
        let agent_id = agent.id;

        let mut def = make_doc_def(ctx.step_id, "Analysis Report");
        def.agent_roster_entry_id = Some(agent_id);
        def.target_length = 2000;

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
        repo.expect_list_document_defs()
            .returning(move |_| Ok(vec![def.clone()]));
        repo.expect_list_edges()
            .withf(move |wid| *wid == wf_id)
            .returning(|_| Ok(vec![]));

        let snapshot = build_config_snapshot(&repo, &ctx).await.unwrap();

        assert!(snapshot.contains("Name: Test Workforce"));
        assert!(snapshot.contains("Task: Build the system"));
        assert!(snapshot.contains("1. Scanner"));
        assert!(snapshot.contains("Analysis Report"));
        assert!(snapshot.contains("~2000 words"));
    }

    #[tokio::test]
    async fn config_snapshot_without_brief() {
        let ctx = make_ctx();
        let step = make_step(ctx.step_id, ctx.workflow_id, "workforce");

        let step_id = ctx.step_id;
        let wf_id = ctx.workflow_id;
        let step_clone = step.clone();

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_step()
            .withf(move |id| *id == step_id)
            .returning(move |_| Ok(Some(step_clone.clone())));
        repo.expect_get_mission_brief().returning(|_| Ok(None));
        repo.expect_list_document_defs().returning(|_| Ok(vec![]));
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

        let result = execute_workforce_tool("nonexistent", &json!({}), &repo, &ctx).await;

        assert!(result["error"]
            .as_str()
            .unwrap()
            .contains("Unknown workforce tool"));
    }
}
