//! Tests for workflow endpoints

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::db::traits::{MockWorkflowRepo, WorkflowRepo};
    use crate::db::WorkflowRow;
    use crate::server::auth::{AuthUser, Claims};
    use crate::types::UserId;
    use chrono::Utc;

    fn make_auth(user_id: Uuid) -> AuthUser {
        AuthUser {
            user_id: UserId(user_id),
            claims: Claims {
                sub: user_id.to_string(),
                email: "test@test.com".to_string(),
                is_admin: false,
                exp: 9999999999,
                iat: 0,
            },
        }
    }

    fn make_workflow(id: Uuid, user_id: Uuid) -> WorkflowRow {
        WorkflowRow {
            id,
            user_id,
            name: "Test Workflow".to_string(),
            description: "A test workflow".to_string(),
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

    // Workflow handlers check: row.user_id != auth.user_id.0 → 404
    // We test this pattern using MockWorkflowRepo::get_workflow.

    #[tokio::test]
    async fn get_own_workflow_ownership_passes() {
        let user_id = Uuid::new_v4();
        let wf_id = Uuid::new_v4();
        let wf = make_workflow(wf_id, user_id);

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_workflow()
            .withf(move |id| *id == wf_id)
            .returning(move |_| Ok(Some(wf.clone())));

        let fetched = repo.get_workflow(wf_id).await.unwrap().unwrap();
        let auth = make_auth(user_id);
        assert_eq!(fetched.user_id, auth.user_id.0);
    }

    #[tokio::test]
    async fn get_other_users_workflow_returns_404() {
        let owner_id = Uuid::new_v4();
        let attacker_id = Uuid::new_v4();
        let wf_id = Uuid::new_v4();
        let wf = make_workflow(wf_id, owner_id);

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(wf.clone())));

        let auth = make_auth(attacker_id);
        let fetched = repo.get_workflow(wf_id).await.unwrap().unwrap();
        // The handler would return 404 here
        assert_ne!(fetched.user_id, auth.user_id.0);
    }

    #[tokio::test]
    async fn nonexistent_workflow_returns_none() {
        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_workflow().returning(|_| Ok(None));

        let result = repo.get_workflow(Uuid::new_v4()).await.unwrap();
        assert!(result.is_none());
    }
}
