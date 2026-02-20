#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::db::fixtures::fixtures::*;
    use crate::db::traits::MockWorkflowRepo;
    use crate::server::services::edges::*;
    use crate::server::services::ServiceError;

    #[tokio::test]
    async fn add_edge_rejects_context_node_target() {
        let owner = Uuid::new_v4();
        let wf = workflow(owner);
        let wf_id = wf.id;
        let from_step = step_in(wf_id);
        let mut to_step = step_in(wf_id);
        to_step.execution_mode = "context".into();
        let from_id = from_step.id;
        let to_id = to_step.id;

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(wf.clone())));
        repo.expect_get_step().returning(move |id| {
            if id == to_id {
                Ok(Some(to_step.clone()))
            } else {
                Ok(Some(from_step.clone()))
            }
        });

        let result = add_edge(&repo, owner, wf_id, from_id, to_id).await;
        assert!(matches!(result, Err(ServiceError::Validation(_))));
    }

    #[tokio::test]
    async fn add_edge_succeeds_for_non_context_target() {
        let owner = Uuid::new_v4();
        let wf = workflow(owner);
        let wf_id = wf.id;
        let to_step = step_in(wf_id);
        let from_id = Uuid::new_v4();
        let to_id = to_step.id;

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(wf.clone())));
        repo.expect_get_step()
            .returning(move |_| Ok(Some(to_step.clone())));
        repo.expect_add_edge()
            .returning(move |wid, fid, tid| Ok(edge_in(wid, fid, tid)));

        let result = add_edge(&repo, owner, wf_id, from_id, to_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn list_edges_rejects_wrong_owner() {
        let owner = Uuid::new_v4();
        let attacker = Uuid::new_v4();
        let wf = workflow(owner);
        let wf_id = wf.id;

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(wf.clone())));

        let result = list_edges(&repo, attacker, wf_id).await;
        assert!(matches!(result, Err(ServiceError::NotFound(_))));
    }

    #[tokio::test]
    async fn list_edges_succeeds_for_owner() {
        let owner = Uuid::new_v4();
        let wf = workflow(owner);
        let wf_id = wf.id;

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(wf.clone())));
        repo.expect_list_edges().returning(|_| Ok(vec![]));

        let result = list_edges(&repo, owner, wf_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn delete_edge_by_id_rejects_wrong_owner() {
        let owner = Uuid::new_v4();
        let attacker = Uuid::new_v4();
        let wf = workflow(owner);
        let wf_id = wf.id;

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(wf.clone())));

        let result = delete_edge_by_id(&repo, attacker, wf_id, Uuid::new_v4()).await;
        assert!(matches!(result, Err(ServiceError::NotFound(_))));
    }
}
