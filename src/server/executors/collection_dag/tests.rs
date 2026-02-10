use super::*;

#[test]
fn test_topological_sort_simple() {
    // A -> B -> C
    let workflow_ids = vec![
        Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
        Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap(),
        Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap(),
    ];

    let collection_workflows = vec![
        CollectionWorkflowRow {
            collection_id: Uuid::new_v4(),
            workflow_id: workflow_ids[0],
            display_order: 0,
            execution_mode: None,
        },
        CollectionWorkflowRow {
            collection_id: Uuid::new_v4(),
            workflow_id: workflow_ids[1],
            display_order: 1,
            execution_mode: None,
        },
        CollectionWorkflowRow {
            collection_id: Uuid::new_v4(),
            workflow_id: workflow_ids[2],
            display_order: 2,
            execution_mode: None,
        },
    ];

    let edges = vec![
        CollectionWorkflowEdgeRow {
            from_workflow_id: workflow_ids[0],
            to_workflow_id: workflow_ids[1],
            collection_id: Uuid::new_v4(),
        },
        CollectionWorkflowEdgeRow {
            from_workflow_id: workflow_ids[1],
            to_workflow_id: workflow_ids[2],
            collection_id: Uuid::new_v4(),
        },
    ];

    let sorted = topological_sort_workflows(&workflow_ids, &collection_workflows, &edges).unwrap();
    assert_eq!(sorted, workflow_ids);
}

#[test]
fn test_topological_sort_diamond() {
    // A -> B -> D
    // A -> C -> D
    let a = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
    let b = Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap();
    let c = Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap();
    let d = Uuid::parse_str("00000000-0000-0000-0000-000000000004").unwrap();

    let workflow_ids = vec![a, b, c, d];

    let collection_workflows = vec![
        CollectionWorkflowRow {
            collection_id: Uuid::new_v4(),
            workflow_id: a,
            display_order: 0,
            execution_mode: None,
        },
        CollectionWorkflowRow {
            collection_id: Uuid::new_v4(),
            workflow_id: b,
            display_order: 1,
            execution_mode: None,
        },
        CollectionWorkflowRow {
            collection_id: Uuid::new_v4(),
            workflow_id: c,
            display_order: 2,
            execution_mode: None,
        },
        CollectionWorkflowRow {
            collection_id: Uuid::new_v4(),
            workflow_id: d,
            display_order: 3,
            execution_mode: None,
        },
    ];

    let edges = vec![
        CollectionWorkflowEdgeRow {
            from_workflow_id: a,
            to_workflow_id: b,
            collection_id: Uuid::new_v4(),
        },
        CollectionWorkflowEdgeRow {
            from_workflow_id: a,
            to_workflow_id: c,
            collection_id: Uuid::new_v4(),
        },
        CollectionWorkflowEdgeRow {
            from_workflow_id: b,
            to_workflow_id: d,
            collection_id: Uuid::new_v4(),
        },
        CollectionWorkflowEdgeRow {
            from_workflow_id: c,
            to_workflow_id: d,
            collection_id: Uuid::new_v4(),
        },
    ];

    let sorted = topological_sort_workflows(&workflow_ids, &collection_workflows, &edges).unwrap();

    // A must come first, D must come last
    assert_eq!(sorted[0], a);
    assert_eq!(sorted[3], d);
    // B and C can be in any order
    assert!(sorted.contains(&b));
    assert!(sorted.contains(&c));
}

#[tokio::test]
async fn test_variable_resolution() {
    use crate::db::{WorkflowExecutionRow, WorkflowRow};
    use chrono::Utc;

    // Mock workflow repo
    struct MockWorkflowRepo;

    #[async_trait::async_trait]
    impl crate::db::traits::WorkflowRepo for MockWorkflowRepo {
        async fn get_workflow(&self, id: Uuid) -> Result<Option<WorkflowRow>> {
            Ok(Some(WorkflowRow {
                id,
                user_id: Uuid::new_v4(),
                name: "test_workflow".to_string(),
                description: String::new(),
                execution_mode: "parallel".to_string(),
                created_at: Utc::now(),
                version: 1,
                container_enabled: false,
                target_repo_url: None,
                target_branch: None,
                vpn_enabled: false,
            }))
        }

        // Stub other methods (not used in test)
        async fn create_workflow(
            &self,
            _: Uuid,
            _: String,
            _: String,
            _: bool,
            _: Option<String>,
            _: Option<String>,
            _: bool,
        ) -> Result<WorkflowRow> {
            unimplemented!()
        }
        async fn list_workflows(&self, _: Uuid) -> Result<Vec<WorkflowRow>> {
            unimplemented!()
        }
        async fn update_workflow(
            &self,
            _: Uuid,
            _: Option<String>,
            _: Option<String>,
            _: Option<bool>,
            _: Option<Option<String>>,
            _: Option<Option<String>>,
            _: Option<bool>,
        ) -> Result<WorkflowRow> {
            unimplemented!()
        }
        async fn delete_workflow(&self, _: Uuid) -> Result<()> {
            unimplemented!()
        }
        async fn create_step(
            &self,
            _: crate::db::WorkflowStepRow,
        ) -> Result<crate::db::WorkflowStepRow> {
            unimplemented!()
        }
        async fn get_step(&self, _: Uuid) -> Result<Option<crate::db::WorkflowStepRow>> {
            unimplemented!()
        }
        async fn list_steps(&self, _: Uuid) -> Result<Vec<crate::db::WorkflowStepRow>> {
            unimplemented!()
        }
        async fn update_step(
            &self,
            _: crate::db::WorkflowStepRow,
        ) -> Result<crate::db::WorkflowStepRow> {
            unimplemented!()
        }
        async fn delete_step(&self, _: Uuid) -> Result<()> {
            unimplemented!()
        }
        async fn set_edges(&self, _: Uuid, _: Vec<crate::db::WorkflowStepEdgeRow>) -> Result<()> {
            unimplemented!()
        }
        async fn list_edges(&self, _: Uuid) -> Result<Vec<crate::db::WorkflowStepEdgeRow>> {
            unimplemented!()
        }
        async fn add_edge(
            &self,
            _: Uuid,
            _: Uuid,
            _: Uuid,
        ) -> Result<crate::db::WorkflowStepEdgeRow> {
            unimplemented!()
        }
        async fn remove_edge(&self, _: Uuid, _: Uuid) -> Result<()> {
            unimplemented!()
        }
        async fn delete_edge_by_id(&self, _: Uuid) -> Result<()> {
            unimplemented!()
        }
        async fn list_step_documents(&self, _: Uuid) -> Result<Vec<crate::db::StepDocumentRow>> {
            unimplemented!()
        }
        async fn add_step_document(&self, _: Uuid, _: Uuid) -> Result<()> {
            unimplemented!()
        }
        async fn remove_step_document(&self, _: Uuid, _: Uuid) -> Result<()> {
            unimplemented!()
        }
        // Phase 3 methods (stubs for tests)
        async fn get_step_inputs(&self, _: Uuid) -> Result<Vec<crate::db::StepInputRow>> {
            unimplemented!()
        }
        async fn get_step_outputs(&self, _: Uuid) -> Result<Vec<crate::db::StepOutputRow>> {
            unimplemented!()
        }
        async fn create_step_input(
            &self,
            _: Uuid,
            _: &str,
            _: &str,
            _: bool,
            _: Option<serde_json::Value>,
            _: Option<String>,
            _: Option<serde_json::Value>,
        ) -> Result<crate::db::StepInputRow> {
            unimplemented!()
        }
        async fn create_step_output(
            &self,
            _: Uuid,
            _: &str,
            _: &str,
            _: &str,
            _: Option<String>,
            _: Option<serde_json::Value>,
        ) -> Result<crate::db::StepOutputRow> {
            unimplemented!()
        }
        async fn delete_step_input(&self, _: Uuid) -> Result<()> {
            unimplemented!()
        }
        async fn delete_step_output(&self, _: Uuid) -> Result<()> {
            unimplemented!()
        }
        async fn get_step_routing_rules(
            &self,
            _: Uuid,
        ) -> Result<Vec<crate::db::StepRoutingRuleRow>> {
            unimplemented!()
        }
        async fn create_routing_rule(
            &self,
            _: Uuid,
            _: &str,
            _: Uuid,
            _: Option<String>,
            _: i32,
        ) -> Result<crate::db::StepRoutingRuleRow> {
            unimplemented!()
        }
        async fn update_routing_rule(
            &self,
            _: Uuid,
            _: Option<Uuid>,
            _: Option<String>,
            _: Option<i32>,
        ) -> Result<crate::db::StepRoutingRuleRow> {
            unimplemented!()
        }
        async fn delete_routing_rule(&self, _: Uuid) -> Result<()> {
            unimplemented!()
        }
        async fn find_step_by_room_id(
            &self,
            _: Uuid,
        ) -> Result<Option<crate::db::WorkflowStepRow>> {
            unimplemented!()
        }
        async fn list_document_defs(
            &self,
            _: Uuid,
        ) -> Result<Vec<crate::db::ProtocolDocumentDefRow>> {
            unimplemented!()
        }
        async fn create_document_def(
            &self,
            _: crate::db::ProtocolDocumentDefRow,
        ) -> Result<crate::db::ProtocolDocumentDefRow> {
            unimplemented!()
        }
        async fn update_document_def(
            &self,
            _: Uuid,
            _: String,
            _: String,
            _: i32,
        ) -> Result<crate::db::ProtocolDocumentDefRow> {
            unimplemented!()
        }
        async fn delete_document_def(&self, _: Uuid) -> Result<()> {
            unimplemented!()
        }
    }

    // Create test data
    let workflow_id = Uuid::new_v4();
    let mut completed_workflows = HashMap::new();

    let workflow_exec = WorkflowExecutionRow {
        id: Uuid::new_v4(),
        collection_run_id: Some(Uuid::new_v4()),
        workflow_id,
        user_id: Uuid::new_v4(),
        status: "completed".to_string(),
        started_at: Some(Utc::now()),
        completed_at: Some(Utc::now()),
        outputs: Some(serde_json::json!({
            "analysis": {"findings": ["issue1", "issue2"]},
            "summary": "Test complete"
        })),
        error: None,
    };

    completed_workflows.insert(workflow_id, workflow_exec);

    // Test variable collection
    let repo = MockWorkflowRepo;
    let outputs = collect_workflow_outputs(&completed_workflows, &repo)
        .await
        .unwrap();

    // Verify structure: $workflow_test_workflow -> { analysis: ..., summary: ... }
    assert!(outputs.contains_key("$workflow_test_workflow"));

    let workflow_outputs = outputs.get("$workflow_test_workflow").unwrap();
    assert!(workflow_outputs.get("analysis").is_some());
    assert!(workflow_outputs.get("summary").is_some());

    // Verify it would work with resolve_variables
    let template = "Found: {$workflow_test_workflow.analysis.findings.0}";
    let resolved = crate::server::hub::dag::resolve_variables(template, &HashMap::new(), &outputs);
    // Note: JSON strings are resolved with quotes (this is correct behavior)
    assert_eq!(resolved, "Found: issue1");
}

#[test]
fn test_topological_sort_cycle() {
    // A -> B -> C -> A (cycle!)
    let workflow_ids = vec![
        Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
        Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap(),
        Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap(),
    ];

    let collection_workflows = vec![
        CollectionWorkflowRow {
            collection_id: Uuid::new_v4(),
            workflow_id: workflow_ids[0],
            display_order: 0,
            execution_mode: None,
        },
        CollectionWorkflowRow {
            collection_id: Uuid::new_v4(),
            workflow_id: workflow_ids[1],
            display_order: 1,
            execution_mode: None,
        },
        CollectionWorkflowRow {
            collection_id: Uuid::new_v4(),
            workflow_id: workflow_ids[2],
            display_order: 2,
            execution_mode: None,
        },
    ];

    let edges = vec![
        CollectionWorkflowEdgeRow {
            from_workflow_id: workflow_ids[0],
            to_workflow_id: workflow_ids[1],
            collection_id: Uuid::new_v4(),
        },
        CollectionWorkflowEdgeRow {
            from_workflow_id: workflow_ids[1],
            to_workflow_id: workflow_ids[2],
            collection_id: Uuid::new_v4(),
        },
        CollectionWorkflowEdgeRow {
            from_workflow_id: workflow_ids[2],
            to_workflow_id: workflow_ids[0],
            collection_id: Uuid::new_v4(),
        },
    ];

    let result = topological_sort_workflows(&workflow_ids, &collection_workflows, &edges);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Cycle detected"));
}
