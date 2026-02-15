#[cfg(test)]
mod tests {
    use chrono::Utc;
    use serde_json::json;
    use uuid::Uuid;

    use crate::db::traits::MockWorkflowRepo;
    use crate::db::{ProtocolDocumentDefRow, WorkflowStepEdgeRow, WorkflowStepRow};

    use super::super::{build_config_snapshot, execute_documenter_tool, DocumenterToolContext};

    fn make_ctx() -> DocumenterToolContext {
        DocumenterToolContext {
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

    fn make_doc_def(step_id: Uuid) -> ProtocolDocumentDefRow {
        ProtocolDocumentDefRow {
            id: Uuid::new_v4(),
            step_id: Some(step_id),
            name: "API Reference".to_string(),
            description: "API docs".to_string(),
            target_length: 3000,
            display_order: 0,
            created_at: Utc::now(),
            protocol_id: None,
            document_id: None,
        }
    }

    // =========================================================================
    // create_doc_def
    // =========================================================================

    #[tokio::test]
    async fn create_doc_def_returns_created_def() {
        let ctx = make_ctx();
        let step_id = ctx.step_id;

        let mut repo = MockWorkflowRepo::new();
        repo.expect_create_document_def().returning(move |def| {
            assert_eq!(def.step_id, Some(step_id));
            assert_eq!(def.name, "Migration Guide");
            assert_eq!(def.target_length, 2000);
            Ok(def)
        });

        let input = json!({
            "name": "Migration Guide",
            "description": "How to migrate",
            "target_length": 2000
        });

        let result = execute_documenter_tool("create_doc_def", &input, &repo, &ctx).await;

        assert!(result["id"].is_string());
        assert_eq!(result["name"], "Migration Guide");
        assert_eq!(result["description"], "How to migrate");
        assert_eq!(result["target_length"], 2000);
    }

    #[tokio::test]
    async fn create_doc_def_uses_defaults_for_optional_fields() {
        let ctx = make_ctx();

        let mut repo = MockWorkflowRepo::new();
        repo.expect_create_document_def().returning(move |def| {
            assert_eq!(def.description, "");
            assert_eq!(def.target_length, 1500);
            Ok(def)
        });

        let input = json!({ "name": "Quick Start" });
        let result = execute_documenter_tool("create_doc_def", &input, &repo, &ctx).await;

        assert!(result["error"].is_null());
        assert_eq!(result["name"], "Quick Start");
    }

    #[tokio::test]
    async fn create_doc_def_missing_name_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let input = json!({});
        let result = execute_documenter_tool("create_doc_def", &input, &repo, &ctx).await;

        assert_eq!(
            result["error"].as_str().unwrap(),
            "Missing required parameter: name"
        );
    }

    // =========================================================================
    // update_doc_def
    // =========================================================================

    #[tokio::test]
    async fn update_doc_def_merges_partial_fields() {
        let ctx = make_ctx();
        let existing = make_doc_def(ctx.step_id);
        let def_id = existing.id;

        let mut repo = MockWorkflowRepo::new();
        let existing_clone = existing.clone();
        repo.expect_list_document_defs()
            .returning(move |_| Ok(vec![existing_clone.clone()]));

        repo.expect_update_document_def()
            .returning(move |id, name, desc, tl| {
                assert_eq!(id, def_id);
                assert_eq!(name, "Updated Name");
                // description and target_length preserved from existing
                assert_eq!(desc, "API docs");
                assert_eq!(tl, 3000);
                Ok(ProtocolDocumentDefRow {
                    id,
                    step_id: Some(Uuid::new_v4()),
                    name,
                    description: desc,
                    target_length: tl,
                    display_order: 0,
                    created_at: Utc::now(),
                    protocol_id: None,
                    document_id: None,
                })
            });

        let input = json!({
            "doc_def_id": def_id.to_string(),
            "name": "Updated Name"
        });

        let result = execute_documenter_tool("update_doc_def", &input, &repo, &ctx).await;

        assert_eq!(result["name"], "Updated Name");
        assert_eq!(result["description"], "API docs");
        assert_eq!(result["target_length"], 3000);
    }

    #[tokio::test]
    async fn update_doc_def_not_found_returns_error() {
        let ctx = make_ctx();

        let mut repo = MockWorkflowRepo::new();
        repo.expect_list_document_defs().returning(|_| Ok(vec![]));

        let input = json!({ "doc_def_id": Uuid::new_v4().to_string() });
        let result = execute_documenter_tool("update_doc_def", &input, &repo, &ctx).await;

        assert_eq!(
            result["error"].as_str().unwrap(),
            "Document definition not found"
        );
    }

    #[tokio::test]
    async fn update_doc_def_missing_id_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let input = json!({ "name": "Test" });
        let result = execute_documenter_tool("update_doc_def", &input, &repo, &ctx).await;

        assert_eq!(
            result["error"].as_str().unwrap(),
            "Missing required parameter: doc_def_id"
        );
    }

    // =========================================================================
    // delete_doc_def
    // =========================================================================

    #[tokio::test]
    async fn delete_doc_def_returns_deleted_true() {
        let ctx = make_ctx();
        let def_id = Uuid::new_v4();

        let mut repo = MockWorkflowRepo::new();
        repo.expect_delete_document_def()
            .withf(move |id| *id == def_id)
            .returning(|_| Ok(()));

        let input = json!({ "doc_def_id": def_id.to_string() });
        let result = execute_documenter_tool("delete_doc_def", &input, &repo, &ctx).await;

        assert_eq!(result["deleted"], true);
    }

    #[tokio::test]
    async fn delete_doc_def_missing_id_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let input = json!({});
        let result = execute_documenter_tool("delete_doc_def", &input, &repo, &ctx).await;

        assert_eq!(
            result["error"].as_str().unwrap(),
            "Missing required parameter: doc_def_id"
        );
    }

    // =========================================================================
    // update_config
    // =========================================================================

    #[tokio::test]
    async fn update_config_updates_all_fields() {
        let ctx = make_ctx();
        let step = make_step(ctx.step_id, ctx.workflow_id, "documenter");

        let step_id = ctx.step_id;
        let step_clone = step.clone();

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_step()
            .withf(move |id| *id == step_id)
            .returning(move |_| Ok(Some(step_clone.clone())));

        repo.expect_update_step().returning(|step| {
            assert_eq!(step.name, Some("New Name".to_string()));
            assert_eq!(step.description, "New desc");
            assert_eq!(step.prompt_template, "New prompt");
            Ok(step)
        });

        let input = json!({
            "name": "New Name",
            "description": "New desc",
            "prompt_template": "New prompt"
        });
        let result = execute_documenter_tool("update_config", &input, &repo, &ctx).await;

        assert_eq!(result["updated"], true);
        assert_eq!(result["name"], "New Name");
        assert_eq!(result["description"], "New desc");
        assert_eq!(result["prompt_template"], "New prompt");
    }

    #[tokio::test]
    async fn update_config_partial_update_preserves_existing() {
        let ctx = make_ctx();
        let mut step = make_step(ctx.step_id, ctx.workflow_id, "documenter");
        step.name = Some("Original Name".to_string());
        step.description = "Original desc".to_string();
        step.prompt_template = "Original prompt".to_string();

        let step_id = ctx.step_id;
        let step_clone = step.clone();

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_step()
            .withf(move |id| *id == step_id)
            .returning(move |_| Ok(Some(step_clone.clone())));

        repo.expect_update_step().returning(|step| {
            // Only description changed
            assert_eq!(step.name, Some("Original Name".to_string()));
            assert_eq!(step.description, "Updated desc");
            assert_eq!(step.prompt_template, "Original prompt");
            Ok(step)
        });

        let input = json!({ "description": "Updated desc" });
        let result = execute_documenter_tool("update_config", &input, &repo, &ctx).await;

        assert_eq!(result["updated"], true);
    }

    #[tokio::test]
    async fn update_config_no_fields_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let input = json!({});
        let result = execute_documenter_tool("update_config", &input, &repo, &ctx).await;

        assert!(result["error"]
            .as_str()
            .unwrap()
            .contains("At least one field"));
    }

    // =========================================================================
    // build_config_snapshot
    // =========================================================================

    #[tokio::test]
    async fn build_config_snapshot_assembles_full_state() {
        let ctx = make_ctx();
        let upstream_context_id = Uuid::new_v4();
        let upstream_single_id = Uuid::new_v4();

        // Documenter step
        let mut documenter_step = make_step(ctx.step_id, ctx.workflow_id, "documenter");
        documenter_step.prompt_template = "Generate docs".to_string();
        documenter_step.name = Some("API Documenter".to_string());
        documenter_step.description = "Generates API docs".to_string();

        // Upstream context step (populated)
        let mut context_step = make_step(upstream_context_id, ctx.workflow_id, "context");
        context_step.prompt_template = "Some API spec content here".to_string();
        context_step.name = Some("API Spec".to_string());
        context_step.description = "OpenAPI specification".to_string();

        // Upstream single step (pending)
        let mut single_step = make_step(upstream_single_id, ctx.workflow_id, "single");
        single_step.name = Some("Researcher".to_string());
        single_step.description = "Researches topics".to_string();

        let doc_def = make_doc_def(ctx.step_id);

        let step_id = ctx.step_id;
        let wf_id = ctx.workflow_id;
        let documenter_step_clone = documenter_step.clone();
        let context_step_clone = context_step.clone();
        let single_step_clone = single_step.clone();

        let mut repo = MockWorkflowRepo::new();

        repo.expect_get_step().returning(move |id| {
            if id == documenter_step_clone.id {
                Ok(Some(documenter_step_clone.clone()))
            } else if id == context_step_clone.id {
                Ok(Some(context_step_clone.clone()))
            } else if id == single_step_clone.id {
                Ok(Some(single_step_clone.clone()))
            } else {
                Ok(None)
            }
        });

        repo.expect_list_document_defs()
            .withf(move |sid| *sid == step_id)
            .returning(move |_| Ok(vec![doc_def.clone()]));

        repo.expect_list_edges()
            .withf(move |wid| *wid == wf_id)
            .returning(move |_| {
                Ok(vec![
                    WorkflowStepEdgeRow {
                        id: Uuid::new_v4(),
                        from_step_id: upstream_context_id,
                        to_step_id: step_id,
                        from_output_port: None,
                        to_input_port: None,
                        transform_jsonpath: None,
                        condition_type: None,
                        condition_value: None,
                        edge_label: None,
                        workflow_id: wf_id,
                    },
                    WorkflowStepEdgeRow {
                        id: Uuid::new_v4(),
                        from_step_id: upstream_single_id,
                        to_step_id: step_id,
                        from_output_port: None,
                        to_input_port: None,
                        transform_jsonpath: None,
                        condition_type: None,
                        condition_value: None,
                        edge_label: None,
                        workflow_id: wf_id,
                    },
                ])
            });

        let snapshot = build_config_snapshot(&repo, &ctx).await.unwrap();

        assert!(snapshot.contains("Name: API Documenter"));
        assert!(snapshot.contains("Description: Generates API docs"));
        assert!(snapshot.contains("Prompt: Generate docs"));
        assert!(snapshot.contains("API Reference"));
        assert!(snapshot.contains("API Spec (context) — populated"));
        assert!(snapshot.contains("Researcher (single) — pending"));
        assert!(snapshot.contains("OpenAPI specification"));
    }

    // =========================================================================
    // Unknown tool
    // =========================================================================

    #[tokio::test]
    async fn unknown_tool_returns_error() {
        let ctx = make_ctx();
        let repo = MockWorkflowRepo::new();

        let result = execute_documenter_tool("nonexistent_tool", &json!({}), &repo, &ctx).await;

        assert!(result["error"]
            .as_str()
            .unwrap()
            .contains("Unknown documenter tool"));
    }
}
