#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use crate::db::traits::MockWorkflowRepo;
    use crate::db::{ProtocolDocumentDefRow, WorkflowRow, WorkflowStepRow};
    use crate::server::services::document_defs::*;
    use crate::server::services::ServiceError;

    fn make_workflow(user_id: Uuid) -> WorkflowRow {
        WorkflowRow {
            id: Uuid::new_v4(),
            user_id,
            name: "wf".to_string(),
            description: String::new(),
            execution_mode: "dag".to_string(),
            version: 1,
            container_enabled: false,
            target_repo_url: None,
            target_branch: None,
            vpn_enabled: false,
            created_at: Utc::now(),
            board_overview_summary: String::new(),
        }
    }

    fn make_step(workflow_id: Uuid) -> WorkflowStepRow {
        WorkflowStepRow {
            id: Uuid::new_v4(),
            workflow_id,
            agent_id: None,
            execution_mode: "single".to_string(),
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
            description: String::new(),
            board_context_cache: String::new(),
            board_context_updated_at: None,
            goal_summary: String::new(),
            goal_summary_updated_at: None,
            sub_workflow_template_id: None,
            child_workflow_id: None,
            is_designer_step: false,
        }
    }

    fn make_def(step_id: Uuid) -> ProtocolDocumentDefRow {
        ProtocolDocumentDefRow {
            id: Uuid::new_v4(),
            step_id: Some(step_id),
            name: "Design Doc".to_string(),
            description: "Architecture document".to_string(),
            target_length: 2000,
            display_order: 0,
            created_at: Utc::now(),
            protocol_id: None,
            document_id: None,
            agent_roster_entry_id: None,
        }
    }

    #[tokio::test]
    async fn list_rejects_wrong_owner() {
        let owner = Uuid::new_v4();
        let attacker = Uuid::new_v4();
        let wf = make_workflow(owner);
        let wf_id = wf.id;

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(wf.clone())));

        let result = list_document_defs(&repo, attacker, wf_id, Uuid::new_v4()).await;
        assert!(matches!(result, Err(ServiceError::NotFound(_))));
    }

    #[tokio::test]
    async fn create_succeeds_for_owner() {
        let owner = Uuid::new_v4();
        let wf = make_workflow(owner);
        let wf_id = wf.id;
        let step = make_step(wf_id);
        let step_id = step.id;

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(wf.clone())));
        repo.expect_get_step()
            .returning(move |_| Ok(Some(step.clone())));
        repo.expect_create_document_def().returning(|def| Ok(def));

        let result = create_document_def(
            &repo,
            CreateDocumentDefInput {
                user_id: owner,
                workflow_id: wf_id,
                step_id,
                name: "Design Doc".to_string(),
                description: "Architecture".to_string(),
                target_length: 2000,
                display_order: 0,
            },
        )
        .await;
        let row = result.unwrap();
        assert_eq!(row.name, "Design Doc");
        assert_eq!(row.target_length, 2000);
    }

    #[tokio::test]
    async fn update_merges_partial_fields() {
        let owner = Uuid::new_v4();
        let wf = make_workflow(owner);
        let wf_id = wf.id;
        let step = make_step(wf_id);
        let step_id = step.id;
        let def = make_def(step_id);
        let def_id = def.id;

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(wf.clone())));
        repo.expect_get_step()
            .returning(move |_| Ok(Some(step.clone())));
        repo.expect_list_document_defs()
            .returning(move |_| Ok(vec![def.clone()]));
        repo.expect_update_document_def()
            .returning(|id, name, desc, len| {
                Ok(ProtocolDocumentDefRow {
                    id,
                    step_id: None,
                    name,
                    description: desc,
                    target_length: len,
                    display_order: 0,
                    created_at: Utc::now(),
                    protocol_id: None,
                    document_id: None,
                    agent_roster_entry_id: None,
                })
            });

        let result = update_document_def(
            &repo,
            UpdateDocumentDefInput {
                user_id: owner,
                workflow_id: wf_id,
                step_id,
                def_id,
                name: Some("Updated Name".to_string()),
                description: None,   // keeps existing
                target_length: None, // keeps existing
            },
        )
        .await;
        let row = result.unwrap();
        assert_eq!(row.name, "Updated Name");
        assert_eq!(row.description, "Architecture document"); // preserved
        assert_eq!(row.target_length, 2000); // preserved
    }

    #[tokio::test]
    async fn delete_returns_info_for_scanner() {
        let owner = Uuid::new_v4();
        let wf = make_workflow(owner);
        let wf_id = wf.id;
        let step = make_step(wf_id);
        let step_id = step.id;
        let def = make_def(step_id);
        let def_id = def.id;

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(wf.clone())));
        repo.expect_get_step()
            .times(2) // verify_step_access + get name
            .returning(move |_| Ok(Some(step.clone())));
        repo.expect_list_document_defs()
            .returning(move |_| Ok(vec![def.clone()]));
        repo.expect_delete_document_def().returning(|_| Ok(()));

        let info = delete_document_def(&repo, owner, wf_id, step_id, def_id)
            .await
            .unwrap();
        assert_eq!(info.def_name, "Design Doc");
        assert_eq!(info.step_name, "Test Step");
    }
}
