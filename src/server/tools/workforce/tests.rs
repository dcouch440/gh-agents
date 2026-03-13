#[cfg(test)]
mod tests {
    use serde_json::json;
    use uuid::Uuid;

    use crate::db::fixtures::fixtures::*;
    use crate::db::traits::MockWorkflowRepo;
    use crate::db::{
        TaskAgentRosterRow, TaskMissionBriefRow, WorkflowRow, WorkflowStepEdgeRow, WorkflowStepRow,
    };

    use super::super::{execute_workforce_tool, WorkforceToolContext};

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
            execution_mode: mode.to_string(),
            name: Some("Test Workforce".to_string()),
            description: "Test description".to_string(),
            ..step_in(workflow_id)
        }
    }

    fn make_workflow(id: Uuid, user_id: Uuid) -> WorkflowRow {
        WorkflowRow {
            id,
            execution_mode: "sequential".to_string(),
            name: "Parent Workflow".to_string(),
            ..workflow(user_id)
        }
    }

    fn make_brief(step_id: Uuid) -> TaskMissionBriefRow {
        TaskMissionBriefRow {
            task_description: "Build the system".to_string(),
            available_capabilities: vec!["code_gen".to_string()],
            ..brief(step_id)
        }
    }

    fn make_roster_agent(brief_id: Uuid, name: &str, order: i32) -> TaskAgentRosterRow {
        TaskAgentRosterRow {
            role_description: "Worker".to_string(),
            capabilities: vec!["code_gen".to_string()],
            ..roster_agent(brief_id, name, order)
        }
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
                    ..Default::default()
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
    // add_agent (workforce-specific: creates pipeline + child step)
    // =========================================================================

    #[tokio::test]
    async fn add_agent_first_agent_creates_pipeline_and_child_step() {
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

        // create_pipeline: get_step returns step without child_workflow_id
        let step_clone = step.clone();
        repo.expect_get_step()
            .returning(move |_| Ok(Some(step_clone.clone())));
        let wf_clone = parent_wf.clone();
        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(wf_clone.clone())));
        repo.expect_create_workflow().returning(move |input| {
            assert_eq!(input.user_id, user_id);
            assert!(input.name.contains("child"));
            Ok(WorkflowRow {
                id: child_wf_id,
                user_id: input.user_id,
                name: input.name,
                execution_mode: "sequential".to_string(),
                ..Default::default()
            })
        });
        repo.expect_update_step().returning(|s| Ok(s));

        // list_agent_roster: empty (first agent)
        repo.expect_list_agent_roster().returning(|_| Ok(vec![]));

        // create agent step (single step, no designer)
        repo.expect_create_step().times(1).returning(|s| Ok(s));

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
                    ..Default::default()
                })
            });

        // link_roster_agent_to_child_step
        repo.expect_link_roster_agent_to_child_step()
            .returning(|_, _| Ok(()));

        // recompute_execution_order: list_edges + list_steps for child workflow
        repo.expect_list_edges().returning(|_| Ok(vec![]));
        repo.expect_list_steps().returning(|_| Ok(vec![]));

        let input = json!({
            "name": "Scanner",
            "role": "Scan codebase",
            "capabilities": ["file_read"]
        });
        let result = execute_workforce_tool("add_agent", &input, &repo, &ctx).await;

        assert_eq!(result["name"], "Scanner");
        assert_eq!(result["role"], "Scan codebase");
        assert!(result["execution_order"].is_array());
    }

    #[tokio::test]
    async fn add_agent_subsequent_agent_appends_to_pipeline() {
        let ctx = make_ctx();
        let brief = make_brief(ctx.step_id);
        let brief_id = brief.id;
        let child_wf_id = Uuid::new_v4();
        let prev_child_step_id = Uuid::new_v4();
        let step_id = ctx.step_id;
        let wf_id = ctx.workflow_id;

        // Step already has child_workflow_id (pipeline exists)
        let mut step = make_step(step_id, wf_id, "workforce");
        step.child_workflow_id = Some(child_wf_id);

        // Existing roster has one agent with a child_step_id
        let mut existing_agent = make_roster_agent(brief_id, "Agent1", 0);
        existing_agent.child_step_id = Some(prev_child_step_id);

        let mut repo = MockWorkflowRepo::new();

        let brief_clone = brief.clone();
        repo.expect_get_mission_brief()
            .returning(move |_| Ok(Some(brief_clone.clone())));

        // create_pipeline: get_step returns step with child_workflow_id (returns existing)
        let step_clone = step.clone();
        repo.expect_get_step()
            .returning(move |_| Ok(Some(step_clone.clone())));

        // resolve_user_id
        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(make_workflow(wf_id, Uuid::new_v4()))));

        let existing_agent_clone = existing_agent.clone();
        repo.expect_list_agent_roster()
            .returning(move |_| Ok(vec![existing_agent_clone.clone()]));

        // create agent step (pipeline already exists, just adds step)
        repo.expect_create_step().times(1).returning(|s| Ok(s));

        // auto-create sequential edge from Agent1 → Agent2
        repo.expect_add_edge()
            .withf(move |_, from, _to| *from == prev_child_step_id)
            .returning(|wid, from, to| {
                Ok(WorkflowStepEdgeRow {
                    id: Uuid::new_v4(),
                    from_step_id: from,
                    to_step_id: to,
                    workflow_id: wid,
                    ..Default::default()
                })
            });

        // recompute_execution_order: list_steps + list_edges
        repo.expect_list_steps().returning(|_| Ok(vec![]));
        repo.expect_list_edges().returning(|_| Ok(vec![]));

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
                    ..Default::default()
                })
            });

        repo.expect_link_roster_agent_to_child_step()
            .returning(|_, _| Ok(()));

        let input = json!({ "name": "Agent2", "role": "Writer" });
        let result = execute_workforce_tool("add_agent", &input, &repo, &ctx).await;

        assert_eq!(result["name"], "Agent2");
        assert!(result["execution_order"].is_array());
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
                    child_step_id: Some(child_step_id),
                    ..Default::default()
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
    async fn update_agent_missing_both_id_and_name_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let input = json!({ "role": "New role" });
        let result = execute_workforce_tool("update_agent", &input, &repo, &ctx).await;

        assert!(result["error"]
            .as_str()
            .unwrap()
            .contains("Provide either agent_id or name"));
    }

    #[tokio::test]
    async fn update_agent_by_name() {
        let ctx = make_ctx();
        let brief = make_brief(ctx.step_id);
        let brief_id = brief.id;
        let agent_id = Uuid::new_v4();

        let mut repo = MockWorkflowRepo::new();

        // resolve_agent_id loads brief + roster for name lookup
        repo.expect_get_mission_brief().returning(move |_| {
            Ok(Some(TaskMissionBriefRow {
                id: brief_id,
                ..brief.clone()
            }))
        });

        let agent = make_roster_agent(brief_id, "Scanner", 0);
        let agent_with_id = TaskAgentRosterRow {
            id: agent_id,
            ..agent
        };
        repo.expect_list_agent_roster()
            .returning(move |_| Ok(vec![agent_with_id.clone()]));

        repo.expect_update_roster_agent()
            .returning(move |id, _name, role, _caps| {
                assert_eq!(id, agent_id);
                Ok(TaskAgentRosterRow {
                    id,
                    mission_brief_id: brief_id,
                    name: "Scanner".to_string(),
                    role_description: role.unwrap_or_default(),
                    ..Default::default()
                })
            });

        let input = json!({ "name": "Scanner", "role": "Updated scanner role" });
        let result = execute_workforce_tool("update_agent", &input, &repo, &ctx).await;

        assert!(result.get("error").is_none(), "got error: {:?}", result);
        assert_eq!(result["name"], "Scanner");
        assert_eq!(result["role"], "Updated scanner role");
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
                workflow_id: child_wf_id,
                ..Default::default()
            }])
        });

        // Pipeline service: remove_step checks remaining steps + recompute
        let remaining = make_step(Uuid::new_v4(), child_wf_id, "single");
        repo.expect_list_steps()
            .returning(move |_| Ok(vec![remaining.clone()]));

        // Remove edges only — no add_edge (no bridging)
        repo.expect_remove_edge().returning(|from, to| {
            Ok(WorkflowStepEdgeRow {
                id: Uuid::new_v4(),
                workflow_id: Uuid::new_v4(),
                from_step_id: from,
                to_step_id: to,
                from_output_port: None,
                to_input_port: None,
                transform_jsonpath: None,
                condition_type: None,
                condition_value: None,
                edge_label: None,
            })
        });
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
    async fn remove_agent_missing_both_id_and_name_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let result = execute_workforce_tool("remove_agent", &json!({}), &repo, &ctx).await;

        assert!(result["error"]
            .as_str()
            .unwrap()
            .contains("Provide either agent_id or name"));
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

        // Pipeline service: recompute_execution_order calls list_steps
        repo.expect_list_steps().returning(|_| Ok(vec![]));

        // Expect edge creation: Scanner → Analyzer
        repo.expect_add_edge()
            .withf(move |_, from, to| *from == scanner_child && *to == analyzer_child)
            .returning(|wid, from, to| {
                Ok(WorkflowStepEdgeRow {
                    id: Uuid::new_v4(),
                    from_step_id: from,
                    to_step_id: to,
                    workflow_id: wid,
                    ..Default::default()
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
                workflow_id: child_wf_id,
                ..Default::default()
            }])
        });

        let input = json!({ "from_agent": "Scanner", "to_agent": "Analyzer" });
        let result = execute_workforce_tool("set_dependency", &input, &repo, &ctx).await;

        assert_eq!(result["already_exists"], true);
    }

    #[tokio::test]
    async fn set_dependency_detects_two_node_cycle() {
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

        // Scanner → Analyzer already exists; trying Analyzer → Scanner
        repo.expect_list_edges().returning(move |_| {
            Ok(vec![WorkflowStepEdgeRow {
                id: Uuid::new_v4(),
                from_step_id: scanner_child,
                to_step_id: analyzer_child,
                workflow_id: child_wf_id,
                ..Default::default()
            }])
        });

        let input = json!({ "from_agent": "Analyzer", "to_agent": "Scanner" });
        let result = execute_workforce_tool("set_dependency", &input, &repo, &ctx).await;

        assert!(result["error"].as_str().unwrap().contains("cycle"));
    }

    #[tokio::test]
    async fn set_dependency_detects_three_node_cycle() {
        let ctx = make_ctx();
        let brief = make_brief(ctx.step_id);
        let brief_id = brief.id;
        let child_wf_id = Uuid::new_v4();
        let a_child = Uuid::new_v4();
        let b_child = Uuid::new_v4();
        let c_child = Uuid::new_v4();

        let mut agent_a = make_roster_agent(brief_id, "A", 0);
        agent_a.child_step_id = Some(a_child);
        let mut agent_b = make_roster_agent(brief_id, "B", 1);
        agent_b.child_step_id = Some(b_child);
        let mut agent_c = make_roster_agent(brief_id, "C", 2);
        agent_c.child_step_id = Some(c_child);

        let mut parent_step = make_step(ctx.step_id, ctx.workflow_id, "workforce");
        parent_step.child_workflow_id = Some(child_wf_id);

        let mut repo = MockWorkflowRepo::new();

        let brief_clone = brief.clone();
        repo.expect_get_mission_brief()
            .returning(move |_| Ok(Some(brief_clone.clone())));

        let a_clone = agent_a.clone();
        let b_clone = agent_b.clone();
        let c_clone = agent_c.clone();
        repo.expect_list_agent_roster()
            .returning(move |_| Ok(vec![a_clone.clone(), b_clone.clone(), c_clone.clone()]));

        let parent_step_clone = parent_step.clone();
        repo.expect_get_step()
            .returning(move |_| Ok(Some(parent_step_clone.clone())));

        // A → B, B → C exist; trying C → A
        repo.expect_list_edges().returning(move |_| {
            Ok(vec![
                WorkflowStepEdgeRow {
                    id: Uuid::new_v4(),
                    from_step_id: a_child,
                    to_step_id: b_child,
                    workflow_id: child_wf_id,
                    ..Default::default()
                },
                WorkflowStepEdgeRow {
                    id: Uuid::new_v4(),
                    from_step_id: b_child,
                    to_step_id: c_child,
                    workflow_id: child_wf_id,
                    ..Default::default()
                },
            ])
        });

        let input = json!({ "from_agent": "C", "to_agent": "A" });
        let result = execute_workforce_tool("set_dependency", &input, &repo, &ctx).await;

        assert!(result["error"].as_str().unwrap().contains("cycle"));
    }

    #[tokio::test]
    async fn set_dependency_allows_non_cycle() {
        let ctx = make_ctx();
        let brief = make_brief(ctx.step_id);
        let brief_id = brief.id;
        let child_wf_id = Uuid::new_v4();
        let a_child = Uuid::new_v4();
        let b_child = Uuid::new_v4();
        let c_child = Uuid::new_v4();

        let mut agent_a = make_roster_agent(brief_id, "A", 0);
        agent_a.child_step_id = Some(a_child);
        let mut agent_b = make_roster_agent(brief_id, "B", 1);
        agent_b.child_step_id = Some(b_child);
        let mut agent_c = make_roster_agent(brief_id, "C", 2);
        agent_c.child_step_id = Some(c_child);

        let mut parent_step = make_step(ctx.step_id, ctx.workflow_id, "workforce");
        parent_step.child_workflow_id = Some(child_wf_id);

        let mut repo = MockWorkflowRepo::new();

        let brief_clone = brief.clone();
        repo.expect_get_mission_brief()
            .returning(move |_| Ok(Some(brief_clone.clone())));

        let a_clone = agent_a.clone();
        let b_clone = agent_b.clone();
        let c_clone = agent_c.clone();
        repo.expect_list_agent_roster()
            .returning(move |_| Ok(vec![a_clone.clone(), b_clone.clone(), c_clone.clone()]));

        let parent_step_clone = parent_step.clone();
        repo.expect_get_step()
            .returning(move |_| Ok(Some(parent_step_clone.clone())));

        // A → B exists; adding A → C (no cycle)
        repo.expect_list_edges().returning(move |_| {
            Ok(vec![WorkflowStepEdgeRow {
                id: Uuid::new_v4(),
                from_step_id: a_child,
                to_step_id: b_child,
                workflow_id: child_wf_id,
                ..Default::default()
            }])
        });

        // Pipeline service: recompute_execution_order calls list_steps
        repo.expect_list_steps().returning(|_| Ok(vec![]));

        repo.expect_add_edge()
            .withf(move |_, from, to| *from == a_child && *to == c_child)
            .returning(|wid, from, to| {
                Ok(WorkflowStepEdgeRow {
                    id: Uuid::new_v4(),
                    from_step_id: from,
                    to_step_id: to,
                    workflow_id: wid,
                    ..Default::default()
                })
            });

        let input = json!({ "from_agent": "A", "to_agent": "C" });
        let result = execute_workforce_tool("set_dependency", &input, &repo, &ctx).await;

        assert_eq!(result["created"], true);
    }

    // =========================================================================
    // remove_dependency
    // =========================================================================

    #[tokio::test]
    async fn remove_dependency_removes_edge() {
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

        // Step with child_workflow_id (needed by recompute_execution_order)
        let mut step = make_step(ctx.step_id, ctx.workflow_id, "workforce");
        step.child_workflow_id = Some(child_wf_id);

        let mut repo = MockWorkflowRepo::new();

        let brief_clone = brief.clone();
        repo.expect_get_mission_brief()
            .returning(move |_| Ok(Some(brief_clone.clone())));

        let scanner_clone = scanner.clone();
        let analyzer_clone = analyzer.clone();
        repo.expect_list_agent_roster()
            .returning(move |_| Ok(vec![scanner_clone.clone(), analyzer_clone.clone()]));

        let step_clone = step.clone();
        repo.expect_get_step()
            .returning(move |_| Ok(Some(step_clone.clone())));

        repo.expect_remove_edge()
            .withf(move |from, to| *from == scanner_child && *to == analyzer_child)
            .returning(|from, to| {
                Ok(WorkflowStepEdgeRow {
                    id: Uuid::new_v4(),
                    workflow_id: Uuid::new_v4(),
                    from_step_id: from,
                    to_step_id: to,
                    from_output_port: None,
                    to_input_port: None,
                    transform_jsonpath: None,
                    condition_type: None,
                    condition_value: None,
                    edge_label: None,
                })
            });

        // recompute_execution_order: list_edges (edge was just removed)
        repo.expect_list_edges().returning(|_| Ok(vec![]));

        // Pipeline service: recompute_execution_order calls list_steps
        repo.expect_list_steps().returning(|_| Ok(vec![]));

        let input = json!({ "from_agent": "Scanner", "to_agent": "Analyzer" });
        let result = execute_workforce_tool("remove_dependency", &input, &repo, &ctx).await;

        assert_eq!(result["removed"], true);
        assert_eq!(result["from"], "Scanner");
        assert_eq!(result["to"], "Analyzer");
        assert!(result["execution_order"].is_array());
    }

    #[tokio::test]
    async fn remove_dependency_missing_params_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let result = execute_workforce_tool("remove_dependency", &json!({}), &repo, &ctx).await;

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
                    available_capabilities: caps.to_vec(),
                    ..Default::default()
                })
            });

        let input = json!({ "capabilities": ["read_file", "write_file"] });
        let result = execute_workforce_tool("set_capabilities", &input, &repo, &ctx).await;

        let caps = result["capabilities"].as_array().unwrap();
        assert_eq!(caps.len(), 2);
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

    // =========================================================================
    // configure_team — input validation
    // =========================================================================

    #[tokio::test]
    async fn configure_team_missing_task_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let input = json!({ "agents": [] });
        let result = execute_workforce_tool("configure_team", &input, &repo, &ctx).await;

        assert!(result["error"]
            .as_str()
            .unwrap()
            .contains("Missing required parameter: task"));
    }

    #[tokio::test]
    async fn configure_team_missing_agents_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let input = json!({ "task": "Do stuff" });
        let result = execute_workforce_tool("configure_team", &input, &repo, &ctx).await;

        assert!(result["error"]
            .as_str()
            .unwrap()
            .contains("Missing required parameter: agents"));
    }

    #[tokio::test]
    async fn configure_team_deduplicates_agents_by_name() {
        let ctx = make_ctx();
        let brief = make_brief(ctx.step_id);
        let mut repo = MockWorkflowRepo::new();

        let step_id = ctx.step_id;
        let wf_id = ctx.workflow_id;
        let user_id = Uuid::new_v4();
        let child_wf_id = Uuid::new_v4();

        let mut parent_step = make_step(step_id, wf_id, "workforce");
        parent_step.child_workflow_id = Some(child_wf_id);

        // ensure_mission_brief — brief exists
        let brief_clone = brief.clone();
        repo.expect_get_mission_brief()
            .returning(move |_| Ok(Some(brief_clone.clone())));
        repo.expect_upsert_mission_brief()
            .returning(|sid, desc, _, _, _| {
                Ok(TaskMissionBriefRow {
                    id: Uuid::new_v4(),
                    step_id: sid,
                    task_description: desc.to_string(),
                    ..Default::default()
                })
            });
        repo.expect_list_agent_roster().returning(|_| Ok(vec![]));
        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(make_workflow(wf_id, user_id))));
        let parent_step_clone = parent_step.clone();
        repo.expect_get_step()
            .returning(move |_| Ok(Some(parent_step_clone.clone())));
        repo.expect_create_step().returning(|s| Ok(s));
        repo.expect_add_roster_agent()
            .returning(|bid, name, role, caps, order| {
                Ok(TaskAgentRosterRow {
                    id: Uuid::new_v4(),
                    mission_brief_id: bid,
                    name: name.to_string(),
                    role_description: role.to_string(),
                    capabilities: caps.to_vec(),
                    execution_order: order,
                    ..Default::default()
                })
            });
        repo.expect_link_roster_agent_to_child_step()
            .returning(|_, _| Ok(()));
        repo.expect_list_edges().returning(|_| Ok(vec![]));
        repo.expect_list_steps().returning(|_| Ok(vec![]));
        repo.expect_update_roster_agent_order()
            .returning(|_, _| Ok(()));

        // Duplicate names should be deduped (last wins), not rejected
        let input = json!({
            "task": "Scan repos",
            "agents": [
                { "name": "Scanner", "role_description": "Scans code" },
                { "name": "scanner", "role_description": "Also scans code" }
            ]
        });
        let result = execute_workforce_tool("configure_team", &input, &repo, &ctx).await;

        assert!(
            result.get("error").is_none(),
            "Expected success, got: {result}"
        );
        let agents = result["agents"].as_array().unwrap();
        // Only one agent (deduped from two "Scanner"/"scanner" entries)
        assert_eq!(agents.len(), 1);
    }

    #[tokio::test]
    async fn configure_team_dependency_unknown_agent_returns_error() {
        let ctx = make_ctx();
        let brief = make_brief(ctx.step_id);
        let mut repo = MockWorkflowRepo::new();

        // ensure_mission_brief runs before dependency validation now
        let brief_clone = brief.clone();
        repo.expect_get_mission_brief()
            .returning(move |_| Ok(Some(brief_clone.clone())));

        let input = json!({
            "task": "Scan repos",
            "agents": [
                { "name": "Scanner", "role_description": "Scans code" }
            ],
            "dependencies": [
                { "from": "Scanner", "to": "Ghost" }
            ]
        });
        let result = execute_workforce_tool("configure_team", &input, &repo, &ctx).await;

        assert!(result["error"].as_str().unwrap().contains("unknown agent"));
    }

    #[tokio::test]
    async fn configure_team_self_dependency_returns_error() {
        let ctx = make_ctx();
        let brief = make_brief(ctx.step_id);
        let mut repo = MockWorkflowRepo::new();

        // ensure_mission_brief runs before dependency validation now
        let brief_clone = brief.clone();
        repo.expect_get_mission_brief()
            .returning(move |_| Ok(Some(brief_clone.clone())));

        let input = json!({
            "task": "Scan repos",
            "agents": [
                { "name": "Scanner", "role_description": "Scans code" }
            ],
            "dependencies": [
                { "from": "Scanner", "to": "Scanner" }
            ]
        });
        let result = execute_workforce_tool("configure_team", &input, &repo, &ctx).await;

        assert!(result["error"]
            .as_str()
            .unwrap()
            .contains("Self-dependency not allowed"));
    }

    // =========================================================================
    // configure_team — fresh team (empty node)
    // =========================================================================

    #[tokio::test]
    async fn configure_team_fresh_creates_all_agents() {
        let ctx = make_ctx();
        // Empty brief — simulates a freshly created node
        let mut brief = make_brief(ctx.step_id);
        brief.task_description = String::new();
        brief.available_capabilities = vec![];
        let step_id = ctx.step_id;
        let wf_id = ctx.workflow_id;
        let user_id = Uuid::new_v4();
        let child_wf_id = Uuid::new_v4();

        let mut parent_step = make_step(step_id, wf_id, "workforce");
        parent_step.child_workflow_id = Some(child_wf_id);

        let mut repo = MockWorkflowRepo::new();

        // ensure_mission_brief — brief exists but is empty
        let brief_clone = brief.clone();
        repo.expect_get_mission_brief()
            .returning(move |_| Ok(Some(brief_clone.clone())));

        // Task diff — empty task means "created"
        repo.expect_upsert_mission_brief()
            .returning(|sid, desc, _, _, _| {
                Ok(TaskMissionBriefRow {
                    id: Uuid::new_v4(),
                    step_id: sid,
                    task_description: desc.to_string(),
                    ..Default::default()
                })
            });

        // Empty current roster
        repo.expect_list_agent_roster().returning(|_| Ok(vec![]));

        // resolve_user_id
        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(make_workflow(wf_id, user_id))));

        // Pipeline: get_step returns step with child_workflow_id
        let parent_step_clone = parent_step.clone();
        repo.expect_get_step()
            .returning(move |_| Ok(Some(parent_step_clone.clone())));

        // Pipeline: create_step for each agent
        repo.expect_create_step().returning(|s| Ok(s));

        // add_roster_agent called for each agent
        repo.expect_add_roster_agent()
            .returning(move |bid, name, role, caps, order| {
                Ok(TaskAgentRosterRow {
                    id: Uuid::new_v4(),
                    mission_brief_id: bid,
                    name: name.to_string(),
                    role_description: role.to_string(),
                    capabilities: caps.to_vec(),
                    execution_order: order,
                    child_step_id: Some(Uuid::new_v4()),
                    ..Default::default()
                })
            });

        repo.expect_link_roster_agent_to_child_step()
            .returning(|_, _| Ok(()));

        // No edges to diff (fresh team, no deps requested)
        repo.expect_list_edges().returning(|_| Ok(vec![]));
        repo.expect_list_steps().returning(|_| Ok(vec![]));

        // recompute: update_roster_agent_order
        repo.expect_update_roster_agent_order()
            .returning(|_, _| Ok(()));

        let input = json!({
            "task": "Scan repositories for vulnerabilities",
            "agents": [
                { "name": "Scanner", "role_description": "Scans code for vulns", "capabilities": ["file_read"] },
                { "name": "Analyzer", "role_description": "Analyzes scan results", "capabilities": ["file_read", "content_search"] }
            ]
        });
        let result = execute_workforce_tool("configure_team", &input, &repo, &ctx).await;

        // Task
        assert_eq!(result["task"]["status"], "created");
        assert_eq!(
            result["task"]["description"],
            "Scan repositories for vulnerabilities"
        );

        // Agents
        let agents = result["agents"].as_array().unwrap();
        assert_eq!(agents.len(), 2);
        assert_eq!(agents[0]["name"], "Scanner");
        assert_eq!(agents[0]["status"], "created");
        assert_eq!(agents[1]["name"], "Analyzer");
        assert_eq!(agents[1]["status"], "created");
    }

    // =========================================================================
    // configure_team — idempotent (same call twice)
    // =========================================================================

    #[tokio::test]
    async fn configure_team_idempotent_returns_unchanged() {
        let ctx = make_ctx();
        let brief = make_brief(ctx.step_id);
        let brief_id = brief.id;
        let step_id = ctx.step_id;
        let wf_id = ctx.workflow_id;
        let user_id = Uuid::new_v4();
        let child_wf_id = Uuid::new_v4();

        // Brief already has the same task
        let mut existing_brief = brief.clone();
        existing_brief.task_description = "Scan repos".to_string();

        // Roster already has the agent with same role + caps
        let mut scanner = make_roster_agent(brief_id, "Scanner", 0);
        scanner.role_description = "Scans code".to_string();
        scanner.capabilities = vec!["file_read".to_string()];
        scanner.child_step_id = Some(Uuid::new_v4());

        let mut parent_step = make_step(step_id, wf_id, "workforce");
        parent_step.child_workflow_id = Some(child_wf_id);

        let mut repo = MockWorkflowRepo::new();

        let existing_brief_clone = existing_brief.clone();
        repo.expect_get_mission_brief()
            .returning(move |_| Ok(Some(existing_brief_clone.clone())));

        let scanner_clone = scanner.clone();
        repo.expect_list_agent_roster()
            .returning(move |_| Ok(vec![scanner_clone.clone()]));

        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(make_workflow(wf_id, user_id))));

        let parent_step_clone = parent_step.clone();
        repo.expect_get_step()
            .returning(move |_| Ok(Some(parent_step_clone.clone())));

        // caps sync
        repo.expect_upsert_mission_brief()
            .returning(|sid, desc, _, _, _| {
                Ok(TaskMissionBriefRow {
                    step_id: sid,
                    task_description: desc.to_string(),
                    ..Default::default()
                })
            });

        // No edges (no deps in input or current)
        repo.expect_list_edges().returning(|_| Ok(vec![]));
        repo.expect_list_steps().returning(|_| Ok(vec![]));

        let input = json!({
            "task": "Scan repos",
            "agents": [
                { "name": "Scanner", "role_description": "Scans code", "capabilities": ["file_read"] }
            ]
        });
        let result = execute_workforce_tool("configure_team", &input, &repo, &ctx).await;

        assert!(result.get("error").is_none());
        assert_eq!(result["task"]["status"], "unchanged");

        let agents = result["agents"].as_array().unwrap();
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0]["status"], "unchanged");
    }

    // =========================================================================
    // configure_team — add agent to existing team
    // =========================================================================

    #[tokio::test]
    async fn configure_team_adds_new_agent_keeps_existing() {
        let ctx = make_ctx();
        let brief = make_brief(ctx.step_id);
        let brief_id = brief.id;
        let step_id = ctx.step_id;
        let wf_id = ctx.workflow_id;
        let user_id = Uuid::new_v4();
        let child_wf_id = Uuid::new_v4();

        let mut existing_brief = brief.clone();
        existing_brief.task_description = "Scan repos".to_string();

        // One existing agent
        let mut scanner = make_roster_agent(brief_id, "Scanner", 0);
        scanner.role_description = "Scans code".to_string();
        scanner.capabilities = vec!["file_read".to_string()];
        scanner.child_step_id = Some(Uuid::new_v4());

        let mut parent_step = make_step(step_id, wf_id, "workforce");
        parent_step.child_workflow_id = Some(child_wf_id);

        let mut repo = MockWorkflowRepo::new();

        let existing_brief_clone = existing_brief.clone();
        repo.expect_get_mission_brief()
            .returning(move |_| Ok(Some(existing_brief_clone.clone())));

        let scanner_clone = scanner.clone();
        repo.expect_list_agent_roster()
            .returning(move |_| Ok(vec![scanner_clone.clone()]));

        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(make_workflow(wf_id, user_id))));

        let parent_step_clone = parent_step.clone();
        repo.expect_get_step()
            .returning(move |_| Ok(Some(parent_step_clone.clone())));

        // Pipeline: create step for the new agent
        repo.expect_create_step().returning(|s| Ok(s));

        repo.expect_add_roster_agent()
            .returning(move |bid, name, role, caps, order| {
                assert_eq!(name, "Analyzer");
                Ok(TaskAgentRosterRow {
                    id: Uuid::new_v4(),
                    mission_brief_id: bid,
                    name: name.to_string(),
                    role_description: role.to_string(),
                    capabilities: caps.to_vec(),
                    execution_order: order,
                    child_step_id: Some(Uuid::new_v4()),
                    ..Default::default()
                })
            });

        repo.expect_link_roster_agent_to_child_step()
            .returning(|_, _| Ok(()));

        // caps sync
        repo.expect_upsert_mission_brief()
            .returning(|sid, desc, _, _, _| {
                Ok(TaskMissionBriefRow {
                    step_id: sid,
                    task_description: desc.to_string(),
                    ..Default::default()
                })
            });

        repo.expect_list_edges().returning(|_| Ok(vec![]));
        repo.expect_list_steps().returning(|_| Ok(vec![]));
        repo.expect_update_roster_agent_order()
            .returning(|_, _| Ok(()));

        let input = json!({
            "task": "Scan repos",
            "agents": [
                { "name": "Scanner", "role_description": "Scans code", "capabilities": ["file_read"] },
                { "name": "Analyzer", "role_description": "Analyzes results", "capabilities": ["content_search"] }
            ]
        });
        let result = execute_workforce_tool("configure_team", &input, &repo, &ctx).await;

        assert!(result.get("error").is_none());
        assert_eq!(result["task"]["status"], "unchanged");

        let agents = result["agents"].as_array().unwrap();
        assert_eq!(agents.len(), 2);
        assert_eq!(agents[0]["name"], "Scanner");
        assert_eq!(agents[0]["status"], "unchanged");
        assert_eq!(agents[1]["name"], "Analyzer");
        assert_eq!(agents[1]["status"], "created");
    }

    // =========================================================================
    // configure_team — remove agent from existing team
    // =========================================================================

    #[tokio::test]
    async fn configure_team_removes_agent_not_in_spec() {
        let ctx = make_ctx();
        let brief = make_brief(ctx.step_id);
        let brief_id = brief.id;
        let step_id = ctx.step_id;
        let wf_id = ctx.workflow_id;
        let user_id = Uuid::new_v4();
        let child_wf_id = Uuid::new_v4();
        let old_agent_child_step = Uuid::new_v4();

        let mut existing_brief = brief.clone();
        existing_brief.task_description = "Scan repos".to_string();

        // Two existing agents — we'll keep Scanner but remove OldAgent
        let mut scanner = make_roster_agent(brief_id, "Scanner", 0);
        scanner.role_description = "Scans code".to_string();
        scanner.capabilities = vec!["file_read".to_string()];
        scanner.child_step_id = Some(Uuid::new_v4());

        let mut old_agent = make_roster_agent(brief_id, "OldAgent", 1);
        old_agent.child_step_id = Some(old_agent_child_step);

        let mut parent_step = make_step(step_id, wf_id, "workforce");
        parent_step.child_workflow_id = Some(child_wf_id);

        let mut repo = MockWorkflowRepo::new();

        let existing_brief_clone = existing_brief.clone();
        repo.expect_get_mission_brief()
            .returning(move |_| Ok(Some(existing_brief_clone.clone())));

        let scanner_clone = scanner.clone();
        let old_agent_clone = old_agent.clone();
        repo.expect_list_agent_roster()
            .returning(move |_| Ok(vec![scanner_clone.clone(), old_agent_clone.clone()]));

        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(make_workflow(wf_id, user_id))));

        let parent_step_clone = parent_step.clone();
        repo.expect_get_step()
            .returning(move |_| Ok(Some(parent_step_clone.clone())));

        // Pipeline remove_step for OldAgent's child step
        repo.expect_list_edges().returning(|_| Ok(vec![]));
        repo.expect_list_steps().returning(|_| Ok(vec![]));
        repo.expect_remove_edge().returning(|from, to| {
            Ok(WorkflowStepEdgeRow {
                id: Uuid::new_v4(),
                workflow_id: Uuid::new_v4(),
                from_step_id: from,
                to_step_id: to,
                from_output_port: None,
                to_input_port: None,
                transform_jsonpath: None,
                condition_type: None,
                condition_value: None,
                edge_label: None,
            })
        });
        repo.expect_delete_step().returning(|_| Ok(()));
        repo.expect_update_step().returning(|s| Ok(s));

        repo.expect_remove_roster_agent().returning(|_| Ok(()));

        // caps sync
        repo.expect_upsert_mission_brief()
            .returning(|sid, desc, _, _, _| {
                Ok(TaskMissionBriefRow {
                    step_id: sid,
                    task_description: desc.to_string(),
                    ..Default::default()
                })
            });

        let input = json!({
            "task": "Scan repos",
            "agents": [
                { "name": "Scanner", "role_description": "Scans code", "capabilities": ["file_read"] }
            ]
        });
        let result = execute_workforce_tool("configure_team", &input, &repo, &ctx).await;

        assert!(result.get("error").is_none());

        let agents = result["agents"].as_array().unwrap();
        assert_eq!(agents.len(), 2);
        assert_eq!(agents[0]["name"], "Scanner");
        assert_eq!(agents[0]["status"], "unchanged");
        assert_eq!(agents[1]["name"], "OldAgent");
        assert_eq!(agents[1]["status"], "removed");
    }

    // =========================================================================
    // configure_team — update agent role/capabilities
    // =========================================================================

    #[tokio::test]
    async fn configure_team_updates_agent_role() {
        let ctx = make_ctx();
        let brief = make_brief(ctx.step_id);
        let brief_id = brief.id;
        let step_id = ctx.step_id;
        let wf_id = ctx.workflow_id;
        let user_id = Uuid::new_v4();
        let child_wf_id = Uuid::new_v4();
        let scanner_child = Uuid::new_v4();

        let mut existing_brief = brief.clone();
        existing_brief.task_description = "Scan repos".to_string();

        let mut scanner = make_roster_agent(brief_id, "Scanner", 0);
        scanner.role_description = "Old role".to_string();
        scanner.capabilities = vec!["file_read".to_string()];
        scanner.child_step_id = Some(scanner_child);

        let mut parent_step = make_step(step_id, wf_id, "workforce");
        parent_step.child_workflow_id = Some(child_wf_id);

        let mut repo = MockWorkflowRepo::new();

        let existing_brief_clone = existing_brief.clone();
        repo.expect_get_mission_brief()
            .returning(move |_| Ok(Some(existing_brief_clone.clone())));

        let scanner_clone = scanner.clone();
        repo.expect_list_agent_roster()
            .returning(move |_| Ok(vec![scanner_clone.clone()]));

        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(make_workflow(wf_id, user_id))));

        let parent_step_clone = parent_step.clone();
        repo.expect_get_step()
            .returning(move |_| Ok(Some(parent_step_clone.clone())));

        // update_roster_agent called with new role
        repo.expect_update_roster_agent()
            .returning(|id, name, role, caps| {
                assert!(role.is_some());
                assert_eq!(role.as_deref().unwrap(), "New deep scanner role");
                assert!(caps.is_none()); // caps unchanged
                Ok(TaskAgentRosterRow {
                    id,
                    name: name.unwrap_or_else(|| "Scanner".to_string()),
                    role_description: role.unwrap_or_default(),
                    ..Default::default()
                })
            });

        // update child step description
        repo.expect_update_step().returning(|s| Ok(s));

        // caps sync
        repo.expect_upsert_mission_brief()
            .returning(|sid, desc, _, _, _| {
                Ok(TaskMissionBriefRow {
                    step_id: sid,
                    task_description: desc.to_string(),
                    ..Default::default()
                })
            });

        repo.expect_list_edges().returning(|_| Ok(vec![]));
        repo.expect_list_steps().returning(|_| Ok(vec![]));

        let input = json!({
            "task": "Scan repos",
            "agents": [
                { "name": "Scanner", "role_description": "New deep scanner role", "capabilities": ["file_read"] }
            ]
        });
        let result = execute_workforce_tool("configure_team", &input, &repo, &ctx).await;

        assert!(result.get("error").is_none());

        let agents = result["agents"].as_array().unwrap();
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0]["name"], "Scanner");
        assert_eq!(agents[0]["status"], "updated");
    }

    // =========================================================================
    // configure_team — case-insensitive name matching
    // =========================================================================

    #[tokio::test]
    async fn configure_team_case_insensitive_name_matching() {
        let ctx = make_ctx();
        let brief = make_brief(ctx.step_id);
        let brief_id = brief.id;
        let step_id = ctx.step_id;
        let wf_id = ctx.workflow_id;
        let user_id = Uuid::new_v4();
        let child_wf_id = Uuid::new_v4();

        let mut existing_brief = brief.clone();
        existing_brief.task_description = "Scan repos".to_string();

        // Existing agent named "Scanner" (title case)
        let mut scanner = make_roster_agent(brief_id, "Scanner", 0);
        scanner.role_description = "Scans code".to_string();
        scanner.capabilities = vec!["file_read".to_string()];
        scanner.child_step_id = Some(Uuid::new_v4());

        let mut parent_step = make_step(step_id, wf_id, "workforce");
        parent_step.child_workflow_id = Some(child_wf_id);

        let mut repo = MockWorkflowRepo::new();

        let existing_brief_clone = existing_brief.clone();
        repo.expect_get_mission_brief()
            .returning(move |_| Ok(Some(existing_brief_clone.clone())));

        let scanner_clone = scanner.clone();
        repo.expect_list_agent_roster()
            .returning(move |_| Ok(vec![scanner_clone.clone()]));

        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(make_workflow(wf_id, user_id))));

        let parent_step_clone = parent_step.clone();
        repo.expect_get_step()
            .returning(move |_| Ok(Some(parent_step_clone.clone())));

        // caps sync
        repo.expect_upsert_mission_brief()
            .returning(|sid, desc, _, _, _| {
                Ok(TaskMissionBriefRow {
                    step_id: sid,
                    task_description: desc.to_string(),
                    ..Default::default()
                })
            });

        repo.expect_list_edges().returning(|_| Ok(vec![]));
        repo.expect_list_steps().returning(|_| Ok(vec![]));

        // Input uses "SCANNER" (all caps) — should match existing "Scanner"
        let input = json!({
            "task": "Scan repos",
            "agents": [
                { "name": "SCANNER", "role_description": "Scans code", "capabilities": ["file_read"] }
            ]
        });
        let result = execute_workforce_tool("configure_team", &input, &repo, &ctx).await;

        assert!(result.get("error").is_none());

        let agents = result["agents"].as_array().unwrap();
        assert_eq!(agents.len(), 1);
        // Matched by normalized name — not created as new
        assert_eq!(agents[0]["status"], "unchanged");
    }

    // =========================================================================
    // configure_team — dependency creation
    // =========================================================================

    #[tokio::test]
    async fn configure_team_creates_dependencies() {
        let ctx = make_ctx();
        let brief = make_brief(ctx.step_id);
        let brief_id = brief.id;
        let step_id = ctx.step_id;
        let wf_id = ctx.workflow_id;
        let user_id = Uuid::new_v4();
        let child_wf_id = Uuid::new_v4();
        let scanner_child = Uuid::new_v4();
        let analyzer_child = Uuid::new_v4();

        let mut existing_brief = brief.clone();
        existing_brief.task_description = "Scan repos".to_string();

        let mut scanner = make_roster_agent(brief_id, "Scanner", 0);
        scanner.role_description = "Scans code".to_string();
        scanner.capabilities = vec![];
        scanner.child_step_id = Some(scanner_child);

        let mut analyzer = make_roster_agent(brief_id, "Analyzer", 1);
        analyzer.role_description = "Analyzes results".to_string();
        analyzer.capabilities = vec![];
        analyzer.child_step_id = Some(analyzer_child);

        let mut parent_step = make_step(step_id, wf_id, "workforce");
        parent_step.child_workflow_id = Some(child_wf_id);

        let mut repo = MockWorkflowRepo::new();

        let existing_brief_clone = existing_brief.clone();
        repo.expect_get_mission_brief()
            .returning(move |_| Ok(Some(existing_brief_clone.clone())));

        let scanner_clone = scanner.clone();
        let analyzer_clone = analyzer.clone();
        repo.expect_list_agent_roster()
            .returning(move |_| Ok(vec![scanner_clone.clone(), analyzer_clone.clone()]));

        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(make_workflow(wf_id, user_id))));

        let parent_step_clone = parent_step.clone();
        repo.expect_get_step()
            .returning(move |_| Ok(Some(parent_step_clone.clone())));

        // caps sync
        repo.expect_upsert_mission_brief()
            .returning(|sid, desc, _, _, _| {
                Ok(TaskMissionBriefRow {
                    step_id: sid,
                    task_description: desc.to_string(),
                    ..Default::default()
                })
            });

        // No existing edges
        repo.expect_list_edges().returning(|_| Ok(vec![]));
        repo.expect_list_steps().returning(|_| Ok(vec![]));

        // Edge creation: Scanner → Analyzer
        repo.expect_add_edge().returning(|wid, from, to| {
            Ok(WorkflowStepEdgeRow {
                id: Uuid::new_v4(),
                from_step_id: from,
                to_step_id: to,
                workflow_id: wid,
                ..Default::default()
            })
        });

        let input = json!({
            "task": "Scan repos",
            "agents": [
                { "name": "Scanner", "role_description": "Scans code" },
                { "name": "Analyzer", "role_description": "Analyzes results" }
            ],
            "dependencies": [
                { "from": "Scanner", "to": "Analyzer" }
            ]
        });
        let result = execute_workforce_tool("configure_team", &input, &repo, &ctx).await;

        assert!(result.get("error").is_none());

        let deps = result["dependencies"].as_array().unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0]["from"], "Scanner");
        assert_eq!(deps[0]["to"], "Analyzer");
        assert_eq!(deps[0]["status"], "created");
    }

    // =========================================================================
    // configure_team — dependency removal
    // =========================================================================

    #[tokio::test]
    async fn configure_team_removes_extra_dependencies() {
        let ctx = make_ctx();
        let brief = make_brief(ctx.step_id);
        let brief_id = brief.id;
        let step_id = ctx.step_id;
        let wf_id = ctx.workflow_id;
        let user_id = Uuid::new_v4();
        let child_wf_id = Uuid::new_v4();
        let scanner_child = Uuid::new_v4();
        let analyzer_child = Uuid::new_v4();

        let mut existing_brief = brief.clone();
        existing_brief.task_description = "Scan repos".to_string();

        let mut scanner = make_roster_agent(brief_id, "Scanner", 0);
        scanner.role_description = "Scans code".to_string();
        scanner.capabilities = vec![];
        scanner.child_step_id = Some(scanner_child);

        let mut analyzer = make_roster_agent(brief_id, "Analyzer", 1);
        analyzer.role_description = "Analyzes results".to_string();
        analyzer.capabilities = vec![];
        analyzer.child_step_id = Some(analyzer_child);

        let mut parent_step = make_step(step_id, wf_id, "workforce");
        parent_step.child_workflow_id = Some(child_wf_id);

        let mut repo = MockWorkflowRepo::new();

        let existing_brief_clone = existing_brief.clone();
        repo.expect_get_mission_brief()
            .returning(move |_| Ok(Some(existing_brief_clone.clone())));

        let scanner_clone = scanner.clone();
        let analyzer_clone = analyzer.clone();
        repo.expect_list_agent_roster()
            .returning(move |_| Ok(vec![scanner_clone.clone(), analyzer_clone.clone()]));

        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(make_workflow(wf_id, user_id))));

        let parent_step_clone = parent_step.clone();
        repo.expect_get_step()
            .returning(move |_| Ok(Some(parent_step_clone.clone())));

        // caps sync
        repo.expect_upsert_mission_brief()
            .returning(|sid, desc, _, _, _| {
                Ok(TaskMissionBriefRow {
                    step_id: sid,
                    task_description: desc.to_string(),
                    ..Default::default()
                })
            });

        // Existing edge Scanner → Analyzer — but desired spec has NO dependencies
        repo.expect_list_edges().returning(move |_| {
            Ok(vec![WorkflowStepEdgeRow {
                id: Uuid::new_v4(),
                from_step_id: scanner_child,
                to_step_id: analyzer_child,
                workflow_id: child_wf_id,
                ..Default::default()
            }])
        });

        repo.expect_list_steps().returning(|_| Ok(vec![]));

        // Edge removal
        repo.expect_remove_edge()
            .withf(move |from, to| *from == scanner_child && *to == analyzer_child)
            .returning(|from, to| {
                Ok(WorkflowStepEdgeRow {
                    id: Uuid::new_v4(),
                    workflow_id: Uuid::new_v4(),
                    from_step_id: from,
                    to_step_id: to,
                    from_output_port: None,
                    to_input_port: None,
                    transform_jsonpath: None,
                    condition_type: None,
                    condition_value: None,
                    edge_label: None,
                })
            });

        let input = json!({
            "task": "Scan repos",
            "agents": [
                { "name": "Scanner", "role_description": "Scans code" },
                { "name": "Analyzer", "role_description": "Analyzes results" }
            ]
        });
        let result = execute_workforce_tool("configure_team", &input, &repo, &ctx).await;

        assert!(result.get("error").is_none());

        let deps = result["dependencies"].as_array().unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0]["status"], "removed");
    }

    // =========================================================================
    // configure_team — capabilities aggregation onto mission brief
    // =========================================================================

    #[tokio::test]
    async fn configure_team_aggregates_capabilities_to_brief() {
        let ctx = make_ctx();
        let mut brief = make_brief(ctx.step_id);
        brief.task_description = String::new();
        brief.available_capabilities = vec![];
        let step_id = ctx.step_id;
        let wf_id = ctx.workflow_id;
        let user_id = Uuid::new_v4();
        let child_wf_id = Uuid::new_v4();

        let mut parent_step = make_step(step_id, wf_id, "workforce");
        parent_step.child_workflow_id = Some(child_wf_id);

        let mut repo = MockWorkflowRepo::new();

        let brief_clone = brief.clone();
        repo.expect_get_mission_brief()
            .returning(move |_| Ok(Some(brief_clone.clone())));

        // Track the capabilities passed to upsert_mission_brief.
        // Called twice: once for task, once for caps sync.
        let caps_seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Vec<String>>::new()));
        let caps_seen_clone = caps_seen.clone();
        repo.expect_upsert_mission_brief()
            .returning(move |sid, desc, caps, _, _| {
                caps_seen_clone.lock().unwrap().push(caps.to_vec());
                Ok(TaskMissionBriefRow {
                    id: Uuid::new_v4(),
                    step_id: sid,
                    task_description: desc.to_string(),
                    available_capabilities: caps.to_vec(),
                    ..Default::default()
                })
            });

        repo.expect_list_agent_roster().returning(|_| Ok(vec![]));

        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(make_workflow(wf_id, user_id))));

        let parent_step_clone = parent_step.clone();
        repo.expect_get_step()
            .returning(move |_| Ok(Some(parent_step_clone.clone())));

        repo.expect_create_step().returning(|s| Ok(s));

        repo.expect_add_roster_agent()
            .returning(move |bid, name, role, caps, order| {
                Ok(TaskAgentRosterRow {
                    id: Uuid::new_v4(),
                    mission_brief_id: bid,
                    name: name.to_string(),
                    role_description: role.to_string(),
                    capabilities: caps.to_vec(),
                    execution_order: order,
                    child_step_id: Some(Uuid::new_v4()),
                    ..Default::default()
                })
            });

        repo.expect_link_roster_agent_to_child_step()
            .returning(|_, _| Ok(()));

        repo.expect_list_edges().returning(|_| Ok(vec![]));
        repo.expect_list_steps().returning(|_| Ok(vec![]));
        repo.expect_update_roster_agent_order()
            .returning(|_, _| Ok(()));

        // Two agents with overlapping capabilities
        let input = json!({
            "task": "Analyze codebase",
            "agents": [
                { "name": "Scanner", "role_description": "Scans code", "capabilities": ["file_read", "content_search"] },
                { "name": "Reporter", "role_description": "Writes reports", "capabilities": ["file_read", "file_write"] }
            ]
        });
        let result = execute_workforce_tool("configure_team", &input, &repo, &ctx).await;

        assert!(result.get("error").is_none());

        // The last upsert_mission_brief call should have the deduplicated, sorted union
        let all_caps = caps_seen.lock().unwrap();
        let caps_sync_call = all_caps.last().unwrap();
        assert_eq!(
            caps_sync_call,
            &vec![
                "content_search".to_string(),
                "file_read".to_string(),
                "file_write".to_string(),
            ],
            "Capabilities should be deduplicated and sorted (BTreeSet)"
        );
    }
}
