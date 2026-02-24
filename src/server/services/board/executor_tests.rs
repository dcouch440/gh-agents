#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use uuid::Uuid;

    use crate::db::traits::{MockSessionRepo, MockWorkflowRepo};
    use crate::db::{CanvasElementMapRow, WorkflowStepEdgeRow, WorkflowStepRow};
    use crate::server::hub::board_serializer::{
        AgentlessChanges, CanvasBounds, CanvasEdge, CanvasNode, ChangeSignificance, EdgeRewire,
        FilteredChangeset, NodeMove, ScoredChange,
    };
    use crate::server::services::board::executor::execute_phase_zero;

    fn empty_changeset() -> FilteredChangeset {
        FilteredChangeset {
            agentless: AgentlessChanges {
                deleted_node_ids: vec![],
                deleted_edge_ids: vec![],
                rewired_edges: vec![],
                moved_nodes: vec![],
            },
            noise: vec![],
            meaningful: vec![],
            aggregate_score: 0.0,
            should_dispatch: false,
        }
    }

    fn make_canvas_node(element_id: &str, text: &str, x: f64, y: f64) -> CanvasNode {
        CanvasNode {
            element_id: element_id.to_string(),
            raw_text: text.to_string(),
            bounds: CanvasBounds {
                x,
                y,
                width: 200.0,
                height: 100.0,
            },
            annotations: vec![],
            sketch: None,
        }
    }

    fn make_canvas_edge(element_id: &str, source: &str, target: &str) -> CanvasEdge {
        CanvasEdge {
            element_id: element_id.to_string(),
            source_node_id: source.to_string(),
            target_node_id: target.to_string(),
        }
    }

    /// Set up MockWorkflowRepo with standard expectations for step creation.
    /// create_step calls verify_workflow_ownership internally, which needs get_workflow.
    fn setup_create_expectations(
        mock: &mut MockWorkflowRepo,
        workflow_id: Uuid,
        user_id: Uuid,
    ) {
        mock.expect_get_workflow().returning(move |_| {
            Ok(Some(crate::db::WorkflowRow {
                id: workflow_id,
                user_id,
                ..Default::default()
            }))
        });

        mock.expect_list_steps().returning(|_| Ok(vec![]));

        mock.expect_create_step().returning(move |step| {
            Ok(WorkflowStepRow {
                workflow_id,
                ..step
            })
        });

        mock.expect_update_step().returning(|step| Ok(step));

        mock.expect_upsert_element_map().returning(|row| Ok(row));
    }

    #[tokio::test]
    async fn create_new_nodes_from_canvas() {
        let workflow_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let mut changeset = empty_changeset();
        changeset.meaningful = vec![
            ScoredChange::NewNode {
                node: CanvasNode {
                    element_id: "r1".to_string(),
                    raw_text: "Research competitors\nLook at Q3 and Q4 data".to_string(),
                    bounds: CanvasBounds {
                        x: 0.0,
                        y: 0.0,
                        width: 200.0,
                        height: 100.0,
                    },
                    annotations: vec!["Focus on pricing".to_string()],
                    sketch: None,
                },
                significance: ChangeSignificance::High,
            },
            ScoredChange::NewNode {
                node: make_canvas_node("r2", "Write report", 300.0, 0.0),
                significance: ChangeSignificance::High,
            },
        ];

        // Capture created steps to verify content mapping.
        let created_steps: Arc<Mutex<Vec<WorkflowStepRow>>> = Arc::new(Mutex::new(vec![]));
        let created_steps_clone = created_steps.clone();

        let mut mock = MockWorkflowRepo::new();
        mock.expect_list_element_maps().returning(|_| Ok(vec![]));

        // create_step needs get_workflow for ownership check
        mock.expect_get_workflow().returning(move |_| {
            Ok(Some(crate::db::WorkflowRow {
                id: workflow_id,
                user_id,
                ..Default::default()
            }))
        });

        mock.expect_list_steps().returning(|_| Ok(vec![]));

        mock.expect_create_step().returning(move |step| {
            let row = WorkflowStepRow {
                workflow_id,
                ..step
            };
            created_steps_clone.lock().unwrap().push(row.clone());
            Ok(row)
        });

        mock.expect_update_step().returning(|step| Ok(step));
        mock.expect_upsert_element_map().returning(|row| Ok(row));

        let session_mock = MockSessionRepo::new();

        let result = execute_phase_zero(&mock, &session_mock, workflow_id, user_id, &changeset)
            .await
            .unwrap();

        assert_eq!(result.created_steps.len(), 2);
        assert_eq!(result.created_steps[0].0, "r1");
        assert_eq!(result.created_steps[1].0, "r2");

        let steps = created_steps.lock().unwrap();

        // First node: name should be first line, prompt_template should be full text.
        assert_eq!(steps[0].name, Some("Research competitors".to_string()));
        assert_eq!(
            steps[0].prompt_template,
            "Research competitors\nLook at Q3 and Q4 data"
        );
        assert_eq!(steps[0].position_x, Some(0.0));
        assert_eq!(steps[0].position_y, Some(0.0));

        // Second node: simple single-line text.
        assert_eq!(steps[1].name, Some("Write report".to_string()));
        assert_eq!(steps[1].prompt_template, "Write report");
    }

    #[tokio::test]
    async fn create_new_edges_between_mapped_nodes() {
        let workflow_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let step_a_id = Uuid::new_v4();
        let step_b_id = Uuid::new_v4();

        let mut changeset = empty_changeset();
        changeset.meaningful = vec![ScoredChange::NewEdge {
            edge: make_canvas_edge("arrow1", "r1", "r2"),
            significance: ChangeSignificance::Medium,
        }];

        let mut mock = MockWorkflowRepo::new();

        // Pre-existing element maps for nodes.
        let map_a = CanvasElementMapRow {
            workflow_id,
            element_id: "r1".to_string(),
            step_id: Some(step_a_id),
            edge_id: None,
            created_at: chrono::Utc::now(),
        };
        let map_b = CanvasElementMapRow {
            workflow_id,
            element_id: "r2".to_string(),
            step_id: Some(step_b_id),
            edge_id: None,
            created_at: chrono::Utc::now(),
        };
        let maps = vec![map_a, map_b];
        mock.expect_list_element_maps()
            .returning(move |_| Ok(maps.clone()));

        mock.expect_add_edge()
            .returning(move |wf_id, from, to| {
                assert_eq!(from, step_a_id);
                assert_eq!(to, step_b_id);
                Ok(WorkflowStepEdgeRow {
                    id: Uuid::new_v4(),
                    workflow_id: wf_id,
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

        mock.expect_upsert_element_map().returning(|row| Ok(row));

        let session_mock = MockSessionRepo::new();

        let result = execute_phase_zero(&mock, &session_mock, workflow_id, user_id, &changeset)
            .await
            .unwrap();

        assert_eq!(result.created_edges.len(), 1);
        assert_eq!(result.created_edges[0].0, "arrow1");
    }

    #[tokio::test]
    async fn delete_nodes_removes_step_and_mapping() {
        let workflow_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let step_id = Uuid::new_v4();

        let mut changeset = empty_changeset();
        changeset.agentless.deleted_node_ids = vec!["r1".to_string()];

        let mut mock = MockWorkflowRepo::new();

        let map = CanvasElementMapRow {
            workflow_id,
            element_id: "r1".to_string(),
            step_id: Some(step_id),
            edge_id: None,
            created_at: chrono::Utc::now(),
        };
        mock.expect_list_element_maps()
            .returning(move |_| Ok(vec![map.clone()]));

        // delete_step requires ownership check + step lookup
        mock.expect_get_workflow().returning(move |_| {
            Ok(Some(crate::db::WorkflowRow {
                id: workflow_id,
                user_id,
                ..Default::default()
            }))
        });

        mock.expect_get_step().returning(move |_| {
            Ok(Some(WorkflowStepRow {
                id: step_id,
                workflow_id,
                ..Default::default()
            }))
        });

        mock.expect_delete_step().returning(|_| Ok(()));
        mock.expect_delete_element_map().returning(|_, _| Ok(()));

        let mut session_mock = MockSessionRepo::new();
        session_mock
            .expect_find_session_by_step_id()
            .returning(|_| Ok(None));
        session_mock
            .expect_find_builder_session_by_step_id()
            .returning(|_| Ok(None));
        session_mock
            .expect_find_manager_builder_session()
            .returning(|_| Ok(None));

        let result = execute_phase_zero(&mock, &session_mock, workflow_id, user_id, &changeset)
            .await
            .unwrap();

        assert_eq!(result.deleted_steps, vec!["r1"]);
    }

    #[tokio::test]
    async fn delete_edges_removes_edge_and_mapping() {
        let workflow_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let edge_id = Uuid::new_v4();

        let mut changeset = empty_changeset();
        changeset.agentless.deleted_edge_ids = vec!["arrow1".to_string()];

        let mut mock = MockWorkflowRepo::new();

        let map = CanvasElementMapRow {
            workflow_id,
            element_id: "arrow1".to_string(),
            step_id: None,
            edge_id: Some(edge_id),
            created_at: chrono::Utc::now(),
        };
        mock.expect_list_element_maps()
            .returning(move |_| Ok(vec![map.clone()]));

        mock.expect_delete_edge_by_id()
            .returning(move |id| {
                assert_eq!(id, edge_id);
                Ok(WorkflowStepEdgeRow {
                    id: edge_id,
                    workflow_id: Uuid::new_v4(),
                    from_step_id: Uuid::new_v4(),
                    to_step_id: Uuid::new_v4(),
                    from_output_port: None,
                    to_input_port: None,
                    transform_jsonpath: None,
                    condition_type: None,
                    condition_value: None,
                    edge_label: None,
                })
            });

        mock.expect_delete_element_map().returning(|_, _| Ok(()));

        let session_mock = MockSessionRepo::new();

        let result = execute_phase_zero(&mock, &session_mock, workflow_id, user_id, &changeset)
            .await
            .unwrap();

        assert_eq!(result.deleted_edges, vec!["arrow1"]);
    }

    #[tokio::test]
    async fn rewire_edge_updates_endpoints() {
        let workflow_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let old_edge_id = Uuid::new_v4();
        let step_a = Uuid::new_v4();
        let step_b = Uuid::new_v4();
        let step_c = Uuid::new_v4();

        let mut changeset = empty_changeset();
        changeset.agentless.rewired_edges = vec![EdgeRewire {
            element_id: "arrow1".to_string(),
            old_source: "r1".to_string(),
            old_target: "r2".to_string(),
            new_source: "r1".to_string(),
            new_target: "r3".to_string(),
        }];

        let mut mock = MockWorkflowRepo::new();

        let maps = vec![
            CanvasElementMapRow {
                workflow_id,
                element_id: "arrow1".to_string(),
                step_id: None,
                edge_id: Some(old_edge_id),
                created_at: chrono::Utc::now(),
            },
            CanvasElementMapRow {
                workflow_id,
                element_id: "r1".to_string(),
                step_id: Some(step_a),
                edge_id: None,
                created_at: chrono::Utc::now(),
            },
            CanvasElementMapRow {
                workflow_id,
                element_id: "r2".to_string(),
                step_id: Some(step_b),
                edge_id: None,
                created_at: chrono::Utc::now(),
            },
            CanvasElementMapRow {
                workflow_id,
                element_id: "r3".to_string(),
                step_id: Some(step_c),
                edge_id: None,
                created_at: chrono::Utc::now(),
            },
        ];
        mock.expect_list_element_maps()
            .returning(move |_| Ok(maps.clone()));

        mock.expect_delete_edge_by_id()
            .returning(move |id| {
                assert_eq!(id, old_edge_id);
                Ok(WorkflowStepEdgeRow {
                    id: old_edge_id,
                    workflow_id: Uuid::new_v4(),
                    from_step_id: Uuid::new_v4(),
                    to_step_id: Uuid::new_v4(),
                    from_output_port: None,
                    to_input_port: None,
                    transform_jsonpath: None,
                    condition_type: None,
                    condition_value: None,
                    edge_label: None,
                })
            });

        mock.expect_add_edge()
            .returning(move |wf_id, from, to| {
                assert_eq!(from, step_a);
                assert_eq!(to, step_c);
                Ok(WorkflowStepEdgeRow {
                    id: Uuid::new_v4(),
                    workflow_id: wf_id,
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

        mock.expect_upsert_element_map().returning(|row| Ok(row));

        let session_mock = MockSessionRepo::new();

        let result = execute_phase_zero(&mock, &session_mock, workflow_id, user_id, &changeset)
            .await
            .unwrap();

        assert_eq!(result.rewired_edges, vec!["arrow1"]);
    }

    #[tokio::test]
    async fn move_nodes_updates_position() {
        let workflow_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let step_id = Uuid::new_v4();

        let mut changeset = empty_changeset();
        changeset.agentless.moved_nodes = vec![NodeMove {
            element_id: "r1".to_string(),
            old_bounds: CanvasBounds {
                x: 0.0,
                y: 0.0,
                width: 200.0,
                height: 100.0,
            },
            new_bounds: CanvasBounds {
                x: 400.0,
                y: 200.0,
                width: 200.0,
                height: 100.0,
            },
        }];

        let mut mock = MockWorkflowRepo::new();

        let map = CanvasElementMapRow {
            workflow_id,
            element_id: "r1".to_string(),
            step_id: Some(step_id),
            edge_id: None,
            created_at: chrono::Utc::now(),
        };
        mock.expect_list_element_maps()
            .returning(move |_| Ok(vec![map.clone()]));

        mock.expect_get_step().returning(move |_| {
            Ok(Some(WorkflowStepRow {
                id: step_id,
                workflow_id,
                position_x: Some(0.0),
                position_y: Some(0.0),
                width: Some(200.0),
                height: Some(100.0),
                ..Default::default()
            }))
        });

        // Verify the update contains the new position.
        mock.expect_update_step().returning(|step| {
            assert_eq!(step.position_x, Some(400.0));
            assert_eq!(step.position_y, Some(200.0));
            Ok(step)
        });

        let session_mock = MockSessionRepo::new();

        let result = execute_phase_zero(&mock, &session_mock, workflow_id, user_id, &changeset)
            .await
            .unwrap();

        assert_eq!(result.moved_steps, vec!["r1"]);
    }

    #[tokio::test]
    async fn execution_order_creates_nodes_before_edges() {
        let workflow_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        // New node and new edge referencing it — both in the same changeset.
        let mut changeset = empty_changeset();
        changeset.meaningful = vec![
            ScoredChange::NewNode {
                node: make_canvas_node("r1", "Node A", 0.0, 0.0),
                significance: ChangeSignificance::High,
            },
            ScoredChange::NewNode {
                node: make_canvas_node("r2", "Node B", 300.0, 0.0),
                significance: ChangeSignificance::High,
            },
            ScoredChange::NewEdge {
                edge: make_canvas_edge("arrow1", "r1", "r2"),
                significance: ChangeSignificance::Medium,
            },
        ];

        let mut mock = MockWorkflowRepo::new();
        mock.expect_list_element_maps().returning(|_| Ok(vec![]));

        mock.expect_get_workflow().returning(move |_| {
            Ok(Some(crate::db::WorkflowRow {
                id: workflow_id,
                user_id,
                ..Default::default()
            }))
        });

        mock.expect_list_steps().returning(|_| Ok(vec![]));

        mock.expect_create_step().returning(move |step| {
            Ok(WorkflowStepRow {
                workflow_id,
                ..step
            })
        });

        mock.expect_update_step().returning(|step| Ok(step));

        mock.expect_upsert_element_map().returning(|row| Ok(row));

        mock.expect_add_edge()
            .returning(move |wf_id, _from, _to| {
                Ok(WorkflowStepEdgeRow {
                    id: Uuid::new_v4(),
                    workflow_id: wf_id,
                    from_step_id: _from,
                    to_step_id: _to,
                    from_output_port: None,
                    to_input_port: None,
                    transform_jsonpath: None,
                    condition_type: None,
                    condition_value: None,
                    edge_label: None,
                })
            });

        let session_mock = MockSessionRepo::new();

        let result = execute_phase_zero(&mock, &session_mock, workflow_id, user_id, &changeset)
            .await
            .unwrap();

        // Both nodes created, then edge created — no errors from missing mappings.
        assert_eq!(result.created_steps.len(), 2);
        assert_eq!(result.created_edges.len(), 1);
    }

    #[tokio::test]
    async fn missing_element_map_returns_error() {
        let workflow_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let mut changeset = empty_changeset();
        changeset.agentless.deleted_node_ids = vec!["nonexistent".to_string()];

        let mut mock = MockWorkflowRepo::new();
        mock.expect_list_element_maps().returning(|_| Ok(vec![]));

        let session_mock = MockSessionRepo::new();

        let result =
            execute_phase_zero(&mock, &session_mock, workflow_id, user_id, &changeset).await;

        assert!(result.is_err());
        let err = format!("{}", result.unwrap_err());
        assert!(err.contains("nonexistent"));
    }
}
