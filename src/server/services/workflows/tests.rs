#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use crate::db::traits::{CreateWorkflowInput, MockWorkflowRepo, UpdateWorkflowInput};
    use crate::db::WorkflowRow;
    use crate::server::services::workflows::*;
    use crate::server::services::ServiceError;

    fn make_workflow(user_id: Uuid) -> WorkflowRow {
        WorkflowRow {
            id: Uuid::new_v4(),
            user_id,
            name: "test-workflow".to_string(),
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

    // ── create_workflow ───────────────────────────────────────────────

    #[tokio::test]
    async fn create_rejects_empty_name() {
        let repo = MockWorkflowRepo::new();
        let result = create_workflow(
            &repo,
            Uuid::new_v4(),
            "   ".to_string(),
            None,
            None,
            None,
            None,
            None,
        )
        .await;
        assert!(matches!(result, Err(ServiceError::Validation(_))));
    }

    #[tokio::test]
    async fn create_succeeds_with_valid_name() {
        let user_id = Uuid::new_v4();
        let mut repo = MockWorkflowRepo::new();
        repo.expect_create_workflow()
            .returning(move |input: CreateWorkflowInput| {
                Ok(WorkflowRow {
                    id: Uuid::new_v4(),
                    user_id: input.user_id,
                    name: input.name,
                    description: input.description,
                    execution_mode: "dag".to_string(),
                    version: 1,
                    container_enabled: input.container_enabled,
                    target_repo_url: input.target_repo_url,
                    target_branch: input.target_branch,
                    vpn_enabled: input.vpn_enabled,
                    created_at: Utc::now(),
                    board_overview_summary: String::new(),
                })
            });

        let result = create_workflow(
            &repo,
            user_id,
            "My Workflow".to_string(),
            None,
            None,
            None,
            None,
            None,
        )
        .await;
        let wf = result.unwrap();
        assert_eq!(wf.name, "My Workflow");
        assert_eq!(wf.user_id, user_id);
        assert!(!wf.container_enabled);
    }

    // ── get_workflow ──────────────────────────────────────────────────

    #[tokio::test]
    async fn get_rejects_wrong_owner() {
        let owner = Uuid::new_v4();
        let attacker = Uuid::new_v4();
        let wf = make_workflow(owner);
        let wf_id = wf.id;

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(wf.clone())));

        let result = get_workflow(&repo, attacker, wf_id).await;
        assert!(matches!(result, Err(ServiceError::NotFound(_))));
    }

    // ── update_workflow ───────────────────────────────────────────────

    #[tokio::test]
    async fn update_merges_partial_fields() {
        let owner = Uuid::new_v4();
        let wf = make_workflow(owner);
        let wf_id = wf.id;
        let wf_clone = wf.clone();

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(wf.clone())));
        repo.expect_update_workflow()
            .returning(move |input: UpdateWorkflowInput| {
                Ok(WorkflowRow {
                    id: input.id,
                    user_id: wf_clone.user_id,
                    name: input.name.unwrap_or_else(|| wf_clone.name.clone()),
                    description: input
                        .description
                        .unwrap_or_else(|| wf_clone.description.clone()),
                    execution_mode: wf_clone.execution_mode.clone(),
                    version: wf_clone.version,
                    container_enabled: input
                        .container_enabled
                        .unwrap_or(wf_clone.container_enabled),
                    target_repo_url: input
                        .target_repo_url
                        .unwrap_or_else(|| wf_clone.target_repo_url.clone()),
                    target_branch: input
                        .target_branch
                        .unwrap_or_else(|| wf_clone.target_branch.clone()),
                    vpn_enabled: input.vpn_enabled.unwrap_or(wf_clone.vpn_enabled),
                    created_at: wf_clone.created_at,
                    board_overview_summary: wf_clone.board_overview_summary.clone(),
                })
            });

        let result = update_workflow(
            &repo,
            owner,
            wf_id,
            Some("Renamed".to_string()),
            None,
            None,
            None,
            None,
            None,
        )
        .await;
        let updated = result.unwrap();
        assert_eq!(updated.name, "Renamed");
        assert!(updated.description.is_empty()); // unchanged
    }

    // ── delete_workflow ───────────────────────────────────────────────

    #[tokio::test]
    async fn delete_rejects_wrong_owner() {
        let owner = Uuid::new_v4();
        let attacker = Uuid::new_v4();
        let wf = make_workflow(owner);
        let wf_id = wf.id;

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(wf.clone())));

        let result = delete_workflow(&repo, attacker, wf_id).await;
        assert!(matches!(result, Err(ServiceError::NotFound(_))));
    }
}
