#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::db::traits::{MockSessionRepo, MockWorkflowRepo};
    use crate::db::{
        CanvasElementMapRow, CanvasSnapshotRow, WorkflowRow, WorkflowStepEdgeRow, WorkflowStepRow,
    };
    use crate::server::hub::board_serializer::ExcalidrawElement;
    use crate::server::services::board::{submit_board, BoardSubmitInput};

    fn make_rect(id: &str, x: f64, y: f64, w: f64, h: f64, text_id: &str) -> serde_json::Value {
        serde_json::json!({
            "type": "rectangle",
            "id": id,
            "x": x, "y": y, "width": w, "height": h,
            "isDeleted": false,
            "boundElements": [{"id": text_id, "type": "text"}]
        })
    }

    fn make_text(id: &str, text: &str, container_id: &str) -> serde_json::Value {
        serde_json::json!({
            "type": "text",
            "id": id,
            "x": 10.0, "y": 10.0, "width": 100.0, "height": 20.0,
            "isDeleted": false,
            "text": text,
            "containerId": container_id
        })
    }

    fn make_arrow(id: &str, start_id: &str, end_id: &str) -> serde_json::Value {
        serde_json::json!({
            "type": "arrow",
            "id": id,
            "x": 0.0, "y": 0.0, "width": 100.0, "height": 0.0,
            "isDeleted": false,
            "startBinding": {"elementId": start_id},
            "endBinding": {"elementId": end_id}
        })
    }

    fn parse_elements(values: &[serde_json::Value]) -> Vec<ExcalidrawElement> {
        values
            .iter()
            .map(|v| serde_json::from_value(v.clone()).unwrap())
            .collect()
    }

    fn workflow_row(user_id: Uuid, workflow_id: Uuid) -> WorkflowRow {
        WorkflowRow {
            id: workflow_id,
            user_id,
            ..Default::default()
        }
    }

    /// Configure MockWorkflowRepo with Phase 0 expectations for step/edge creation.
    fn expect_phase_zero_creates(mock: &mut MockWorkflowRepo, workflow_id: Uuid) {
        // list_element_maps — empty on first submit
        mock.expect_list_element_maps().returning(|_| Ok(vec![]));

        // list_steps — needed by create_step for ref_id generation
        mock.expect_list_steps().returning(|_| Ok(vec![]));

        // create_step — return a step with the given workflow_id
        mock.expect_create_step().returning(move |step| {
            Ok(WorkflowStepRow {
                workflow_id,
                ..step
            })
        });

        // update_step — for board_context_cache
        mock.expect_update_step().returning(|step| Ok(step));

        // upsert_element_map — store the mapping
        mock.expect_upsert_element_map().returning(|row| Ok(row));

        // add_edge — for new edges
        mock.expect_add_edge().returning(move |wf_id, from, to| {
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
    }

    fn empty_session_repo() -> MockSessionRepo {
        MockSessionRepo::new()
    }

    #[tokio::test]
    async fn first_submit_all_nodes_are_new() {
        let user_id = Uuid::new_v4();
        let workflow_id = Uuid::new_v4();

        let elements_json_vals = vec![
            make_rect("r1", 0.0, 0.0, 200.0, 100.0, "t1"),
            make_text("t1", "Research competitors", "r1"),
            make_rect("r2", 300.0, 0.0, 200.0, 100.0, "t2"),
            make_text("t2", "Write report", "r2"),
            make_arrow("a1", "r1", "r2"),
        ];
        let elements = parse_elements(&elements_json_vals);
        let elements_json = serde_json::to_string(&elements_json_vals).unwrap();

        let mut mock = MockWorkflowRepo::new();

        mock.expect_get_workflow()
            .returning(move |_| Ok(Some(workflow_row(user_id, workflow_id))));

        mock.expect_get_canvas_snapshot().returning(|_| Ok(None));

        mock.expect_upsert_canvas_snapshot()
            .returning(|row| Ok(row));

        expect_phase_zero_creates(&mut mock, workflow_id);
        let session_mock = empty_session_repo();

        let result = submit_board(
            &mock,
            &session_mock,
            BoardSubmitInput {
                workflow_id,
                user_id,
                elements,
                elements_json,
            },
        )
        .await
        .unwrap();

        assert!(result.is_first_submit);
        assert_eq!(result.snapshot.nodes.len(), 2);
        assert_eq!(result.snapshot.edges.len(), 1);

        // All nodes should be new (meaningful)
        assert!(result.changeset.should_dispatch);
        assert_eq!(result.changeset.meaningful.len(), 3); // 2 nodes + 1 edge

        // Phase 0 should have created 2 steps and 1 edge
        assert_eq!(result.phase_zero.created_steps.len(), 2);
        assert_eq!(result.phase_zero.created_edges.len(), 1);
    }

    #[tokio::test]
    async fn second_submit_detects_changes() {
        let user_id = Uuid::new_v4();
        let workflow_id = Uuid::new_v4();

        // First snapshot: one node "Research"
        let first_elements = vec![
            make_rect("r1", 0.0, 0.0, 200.0, 100.0, "t1"),
            make_text("t1", "Research competitors", "r1"),
        ];
        let first_snapshot =
            crate::server::hub::board_serializer::classify_board(&parse_elements(&first_elements));
        let snapshot_json = serde_json::to_string(&first_snapshot).unwrap();

        // Second submit: same node with updated text + a new node
        let second_elements = vec![
            make_rect("r1", 0.0, 0.0, 200.0, 100.0, "t1"),
            make_text("t1", "Research competitors and pricing", "r1"),
            make_rect("r2", 300.0, 0.0, 200.0, 100.0, "t2"),
            make_text("t2", "Write report", "r2"),
        ];
        let elements = parse_elements(&second_elements);
        let elements_json = serde_json::to_string(&second_elements).unwrap();

        let mut mock = MockWorkflowRepo::new();

        mock.expect_get_workflow()
            .returning(move |_| Ok(Some(workflow_row(user_id, workflow_id))));

        let snapshot_json_clone = snapshot_json.clone();
        mock.expect_get_canvas_snapshot().returning(move |_| {
            Ok(Some(CanvasSnapshotRow {
                workflow_id,
                snapshot_json: snapshot_json_clone.clone(),
                elements_json: String::new(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            }))
        });

        mock.expect_upsert_canvas_snapshot()
            .returning(|row| Ok(row));

        // r1 already exists from first submit — has an element map entry.
        let existing_step_id = Uuid::new_v4();
        let map = CanvasElementMapRow {
            workflow_id,
            element_id: "r1".to_string(),
            step_id: Some(existing_step_id),
            edge_id: None,
            created_at: chrono::Utc::now(),
        };
        mock.expect_list_element_maps()
            .returning(move |_| Ok(vec![map.clone()]));

        // UpdatedNode processing fetches the existing step.
        mock.expect_get_step().returning(move |_| {
            Ok(Some(WorkflowStepRow {
                id: existing_step_id,
                workflow_id,
                name: Some("Research competitors".to_string()),
                prompt_template: "Research competitors".to_string(),
                ..Default::default()
            }))
        });

        // Step creation (for new node r2) + step update (for updated r1).
        mock.expect_list_steps().returning(|_| Ok(vec![]));
        mock.expect_create_step().returning(move |step| {
            Ok(WorkflowStepRow {
                workflow_id,
                ..step
            })
        });
        mock.expect_update_step().returning(|step| Ok(step));
        mock.expect_upsert_element_map().returning(|row| Ok(row));

        let session_mock = empty_session_repo();

        let result = submit_board(
            &mock,
            &session_mock,
            BoardSubmitInput {
                workflow_id,
                user_id,
                elements,
                elements_json,
            },
        )
        .await
        .unwrap();

        assert!(!result.is_first_submit);
        assert_eq!(result.snapshot.nodes.len(), 2);

        // Should have meaningful changes: 1 updated node + 1 new node
        assert!(result.changeset.meaningful.len() >= 2);
        assert!(result.changeset.should_dispatch);

        // Phase 0 should have created the new node (r2) and updated existing (r1).
        assert_eq!(result.phase_zero.created_steps.len(), 1);
        assert_eq!(result.phase_zero.updated_steps.len(), 1);
        assert_eq!(result.phase_zero.updated_steps[0].0, "r1");
        assert_eq!(result.phase_zero.updated_steps[0].1, existing_step_id);
    }

    #[tokio::test]
    async fn wrong_user_gets_not_found() {
        let owner_id = Uuid::new_v4();
        let caller_id = Uuid::new_v4();
        let workflow_id = Uuid::new_v4();

        let mut mock = MockWorkflowRepo::new();

        mock.expect_get_workflow()
            .returning(move |_| Ok(Some(workflow_row(owner_id, workflow_id))));

        let session_mock = empty_session_repo();

        let result = submit_board(
            &mock,
            &session_mock,
            BoardSubmitInput {
                workflow_id,
                user_id: caller_id,
                elements: vec![],
                elements_json: "[]".to_string(),
            },
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            crate::server::services::ServiceError::NotFound(_)
        ));
    }

    #[tokio::test]
    async fn missing_workflow_gets_not_found() {
        let user_id = Uuid::new_v4();
        let workflow_id = Uuid::new_v4();

        let mut mock = MockWorkflowRepo::new();

        mock.expect_get_workflow().returning(|_| Ok(None));

        let session_mock = empty_session_repo();

        let result = submit_board(
            &mock,
            &session_mock,
            BoardSubmitInput {
                workflow_id,
                user_id,
                elements: vec![],
                elements_json: "[]".to_string(),
            },
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            crate::server::services::ServiceError::NotFound(_)
        ));
    }

    #[tokio::test]
    async fn empty_board_submit_succeeds() {
        let user_id = Uuid::new_v4();
        let workflow_id = Uuid::new_v4();

        let mut mock = MockWorkflowRepo::new();

        mock.expect_get_workflow()
            .returning(move |_| Ok(Some(workflow_row(user_id, workflow_id))));

        mock.expect_get_canvas_snapshot().returning(|_| Ok(None));

        mock.expect_upsert_canvas_snapshot()
            .returning(|row| Ok(row));

        // Empty board — Phase 0 has nothing to do, but still loads maps
        mock.expect_list_element_maps().returning(|_| Ok(vec![]));

        let session_mock = empty_session_repo();

        let result = submit_board(
            &mock,
            &session_mock,
            BoardSubmitInput {
                workflow_id,
                user_id,
                elements: vec![],
                elements_json: "[]".to_string(),
            },
        )
        .await
        .unwrap();

        assert!(result.is_first_submit);
        assert_eq!(result.snapshot.nodes.len(), 0);
        assert_eq!(result.snapshot.edges.len(), 0);
        assert!(!result.changeset.should_dispatch);
        assert!(result.phase_zero.created_steps.is_empty());
    }
}
