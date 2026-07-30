#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::db::traits::{
        CreateDesignerOutputGenericInput, CreateDesignerOutputInput, CreateStepInputPort,
        CreateWorkflowInput, UpdateWorkflowInput,
    };

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

        let sorted =
            topological_sort_workflows(&workflow_ids, &collection_workflows, &edges).unwrap();
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

        let sorted =
            topological_sort_workflows(&workflow_ids, &collection_workflows, &edges).unwrap();

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
                    execution_mode: "parallel".to_string(),
                    ..Default::default()
                }))
            }

            // Stub other methods (not used in test)
            async fn create_workflow(&self, _: CreateWorkflowInput) -> Result<WorkflowRow> {
                unimplemented!()
            }
            async fn list_workflows(&self, _: Uuid) -> Result<Vec<WorkflowRow>> {
                unimplemented!()
            }
            async fn update_workflow(&self, _: UpdateWorkflowInput) -> Result<WorkflowRow> {
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
            async fn find_step_by_ref_id(
                &self,
                _: Uuid,
                _: &str,
            ) -> Result<Option<crate::db::WorkflowStepRow>> {
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
            async fn replace_chat_beliefs(
                &self,
                _: Uuid,
                _: &[crate::db::BeliefRow],
            ) -> Result<Vec<crate::db::BeliefRow>> {
                unimplemented!()
            }
            async fn get_beliefs_for_connected_steps(
                &self,
                _: Uuid,
                _: Uuid,
            ) -> Result<Vec<crate::db::BeliefRow>> {
                unimplemented!()
            }
            async fn set_edges(
                &self,
                _: Uuid,
                _: Vec<crate::db::WorkflowStepEdgeRow>,
            ) -> Result<()> {
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
            async fn remove_edge(
                &self,
                _: Uuid,
                _: Uuid,
            ) -> Result<crate::db::WorkflowStepEdgeRow> {
                unimplemented!()
            }
            async fn delete_edge_by_id(&self, _: Uuid) -> Result<crate::db::WorkflowStepEdgeRow> {
                unimplemented!()
            }
            async fn list_step_documents(
                &self,
                _: Uuid,
            ) -> Result<Vec<crate::db::StepDocumentRow>> {
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
                _: CreateStepInputPort,
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
            async fn get_document_def(
                &self,
                _: Uuid,
            ) -> Result<Option<crate::db::ProtocolDocumentDefRow>> {
                Ok(None)
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
            async fn link_document_to_def(&self, _: Uuid, _: Uuid) -> Result<()> {
                unimplemented!()
            }
            async fn delete_document_def(&self, _: Uuid) -> Result<()> {
                unimplemented!()
            }
            // Workforce stubs
            async fn get_mission_brief(
                &self,
                _: Uuid,
            ) -> Result<Option<crate::db::TaskMissionBriefRow>> {
                unimplemented!()
            }
            async fn upsert_mission_brief(
                &self,
                _: Uuid,
                _: &str,
                _: &[String],
                _: &str,
                _: Option<String>,
            ) -> Result<crate::db::TaskMissionBriefRow> {
                unimplemented!()
            }
            async fn list_agent_roster(
                &self,
                _: Uuid,
            ) -> Result<Vec<crate::db::TaskAgentRosterRow>> {
                unimplemented!()
            }
            async fn add_roster_agent(
                &self,
                _: Uuid,
                _: &str,
                _: &str,
                _: &[String],
                _: i32,
            ) -> Result<crate::db::TaskAgentRosterRow> {
                unimplemented!()
            }
            async fn update_roster_agent(
                &self,
                _: Uuid,
                _: Option<String>,
                _: Option<String>,
                _: Option<Vec<String>>,
            ) -> Result<crate::db::TaskAgentRosterRow> {
                unimplemented!()
            }
            async fn remove_roster_agent(&self, _: Uuid) -> Result<()> {
                unimplemented!()
            }
            async fn update_roster_agent_order(&self, _: Uuid, _: i32) -> Result<()> {
                unimplemented!()
            }
            async fn link_roster_agent_to_child_step(
                &self,
                _: Uuid,
                _: Option<Uuid>,
            ) -> Result<()> {
                unimplemented!()
            }
            async fn get_extraction_plan(
                &self,
                _: Uuid,
            ) -> Result<Option<crate::db::BeliefExtractionPlanRow>> {
                unimplemented!()
            }
            async fn upsert_extraction_plan(
                &self,
                _: Uuid,
                _: &str,
                _: &[String],
                _: &str,
                _: &str,
            ) -> Result<crate::db::BeliefExtractionPlanRow> {
                unimplemented!()
            }
            async fn insert_belief(
                &self,
                _: &crate::db::BeliefRow,
            ) -> Result<crate::db::BeliefRow> {
                unimplemented!()
            }
            async fn list_beliefs_for_execution(
                &self,
                _: Uuid,
            ) -> Result<Vec<crate::db::BeliefRow>> {
                unimplemented!()
            }
            // Room step config stubs
            async fn get_room_step_config(
                &self,
                _: Uuid,
            ) -> Result<Option<crate::db::RoomStepConfigRow>> {
                unimplemented!()
            }
            async fn upsert_room_step_config(
                &self,
                _: Uuid,
                _: &str,
                _: i32,
                _: &str,
                _: bool,
            ) -> Result<crate::db::RoomStepConfigRow> {
                unimplemented!()
            }
            async fn list_room_step_members(
                &self,
                _: Uuid,
            ) -> Result<Vec<crate::db::RoomStepMemberRow>> {
                unimplemented!()
            }
            async fn add_room_step_member(
                &self,
                _: Uuid,
                _: &str,
                _: &str,
                _: &str,
                _: i32,
            ) -> Result<crate::db::RoomStepMemberRow> {
                unimplemented!()
            }
            async fn update_room_step_member(
                &self,
                _: Uuid,
                _: Option<String>,
                _: Option<String>,
                _: Option<String>,
            ) -> Result<crate::db::RoomStepMemberRow> {
                unimplemented!()
            }
            async fn remove_room_step_member(&self, _: Uuid) -> Result<()> {
                unimplemented!()
            }
            // Agent Designer stubs
            async fn create_designer_run(
                &self,
                _: Uuid,
                _: Uuid,
                _: Uuid,
                _: Uuid,
                _: &str,
            ) -> Result<crate::db::AgentDesignerRunRow> {
                unimplemented!()
            }
            async fn update_designer_run_tokens(
                &self,
                _: Uuid,
                _: i64,
                _: i64,
                _: f32,
            ) -> Result<()> {
                unimplemented!()
            }
            async fn create_designer_output(
                &self,
                _: CreateDesignerOutputInput,
            ) -> Result<crate::db::AgentDesignerOutputRow> {
                unimplemented!()
            }
            async fn create_designer_run_generic(
                &self,
                _: Uuid,
                _: Uuid,
                _: Uuid,
                _: &str,
                _: &str,
                _: &str,
            ) -> Result<crate::db::AgentDesignerRunRow> {
                unimplemented!()
            }
            async fn create_designer_output_generic(
                &self,
                _: CreateDesignerOutputGenericInput,
            ) -> Result<crate::db::AgentDesignerOutputRow> {
                unimplemented!()
            }
            async fn list_designer_outputs(
                &self,
                _: Uuid,
            ) -> Result<Vec<crate::db::AgentDesignerOutputRow>> {
                unimplemented!()
            }
            async fn list_designer_outputs_by_protocol_execution(
                &self,
                _: Uuid,
            ) -> Result<Vec<crate::db::AgentDesignerOutputRow>> {
                unimplemented!()
            }
            async fn list_designer_runs_for_step(
                &self,
                _: Uuid,
                _: Uuid,
            ) -> Result<Vec<crate::db::AgentDesignerRunRow>> {
                unimplemented!()
            }
            // Assistant Notes stubs
            async fn get_plan(&self, _: Uuid) -> Result<Option<String>> {
                unimplemented!()
            }
            async fn upsert_plan(&self, _: Uuid, _: &str) -> Result<()> {
                unimplemented!()
            }
            async fn get_all_plans_for_workflow(
                &self,
                _: Uuid,
            ) -> Result<Vec<(Uuid, Option<String>, String, String)>> {
                unimplemented!()
            }
            async fn get_board_overview_summary(&self, _: Uuid) -> Result<String> {
                unimplemented!()
            }
            async fn update_board_overview_summary(&self, _: Uuid, _: &str) -> Result<()> {
                unimplemented!()
            }
            async fn update_designer_handoff(&self, _: Uuid, _: &str) -> Result<()> {
                unimplemented!()
            }
            // Step question state stubs
            async fn get_step_question_state(
                &self,
                _: Uuid,
            ) -> Result<Option<crate::db::StepQuestionStateRow>> {
                unimplemented!()
            }
            async fn get_step_question_states(
                &self,
                _: &[Uuid],
            ) -> Result<Vec<crate::db::StepQuestionStateRow>> {
                unimplemented!()
            }
            async fn upsert_step_question_state(
                &self,
                _: Uuid,
                _: &str,
                _: Option<String>,
            ) -> Result<()> {
                unimplemented!()
            }
            // Run template stubs
            async fn create_template(
                &self,
                _: Uuid,
                _: Uuid,
                _: &str,
                _: Option<String>,
                _: serde_json::Value,
            ) -> Result<crate::db::RunTemplateRow> {
                unimplemented!()
            }
            async fn get_template(&self, _: Uuid) -> Result<Option<crate::db::RunTemplateRow>> {
                unimplemented!()
            }
            async fn list_templates(&self, _: Uuid) -> Result<Vec<crate::db::RunTemplateRow>> {
                unimplemented!()
            }
            async fn delete_template(&self, _: Uuid) -> Result<()> {
                unimplemented!()
            }
            async fn set_step_pinned(&self, _: Uuid, _: bool) -> Result<()> {
                unimplemented!()
            }
            async fn update_run_results_summary(&self, _: Uuid, _: &str) -> Result<()> {
                unimplemented!()
            }
            async fn get_run_context_for_step(
                &self,
                _: Uuid,
                _: Uuid,
            ) -> Result<Vec<(String, String, bool)>> {
                unimplemented!()
            }
            async fn get_canvas_snapshot(
                &self,
                _: Uuid,
            ) -> Result<Option<crate::db::CanvasSnapshotRow>> {
                unimplemented!()
            }
            async fn upsert_canvas_snapshot(
                &self,
                _: crate::db::CanvasSnapshotRow,
            ) -> Result<crate::db::CanvasSnapshotRow> {
                unimplemented!()
            }
            async fn list_element_maps(
                &self,
                _: Uuid,
            ) -> Result<Vec<crate::db::CanvasElementMapRow>> {
                unimplemented!()
            }
            async fn upsert_element_map(
                &self,
                _: crate::db::CanvasElementMapRow,
            ) -> Result<crate::db::CanvasElementMapRow> {
                unimplemented!()
            }
            async fn delete_element_map(&self, _: Uuid, _: &str) -> Result<()> {
                unimplemented!()
            }
            async fn update_canvas_snapshot_response(&self, _: Uuid, _: String) -> Result<()> {
                unimplemented!()
            }
            async fn upsert_step_image(&self, _: Uuid, _: &str) -> Result<()> {
                unimplemented!()
            }
            async fn get_step_stroke_image(&self, _: Uuid) -> Result<Option<String>> {
                unimplemented!()
            }
            async fn create_workflow_version(
                &self,
                _: crate::db::WorkflowVersionRow,
            ) -> Result<crate::db::WorkflowVersionRow> {
                unimplemented!()
            }
            async fn list_workflow_versions(
                &self,
                _: Uuid,
            ) -> Result<Vec<crate::db::WorkflowVersionRow>> {
                unimplemented!()
            }
            async fn get_workflow_version(
                &self,
                _: Uuid,
            ) -> Result<Option<crate::db::WorkflowVersionRow>> {
                unimplemented!()
            }
            async fn get_latest_version_number(&self, _: Uuid) -> Result<i32> {
                unimplemented!()
            }
            async fn delete_workflow_version(&self, _: Uuid) -> Result<()> {
                unimplemented!()
            }
            async fn get_active_run_for_workflow(&self, _: Uuid) -> Result<Option<Uuid>> {
                Ok(None)
            }
        }

        // Create test data
        let workflow_id = Uuid::new_v4();
        let mut completed_workflows = HashMap::new();

        let exec_id = Uuid::new_v4();
        let workflow_exec = WorkflowExecutionRow {
            id: exec_id,
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
            execution_mode: "full".to_string(),
            root_execution_id: Some(exec_id),
            ..Default::default()
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
        let resolved =
            crate::server::hub::dag::resolve_variables(template, &HashMap::new(), &outputs);
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
}
