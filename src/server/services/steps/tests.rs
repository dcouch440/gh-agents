#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::db::fixtures::fixtures::*;
    use crate::db::traits::MockWorkflowRepo;
    use crate::server::services::steps::*;
    use crate::server::services::ServiceError;

    // ── verify_step_access ────────────────────────────────────────────

    #[tokio::test]
    async fn verify_access_rejects_wrong_owner() {
        let owner = Uuid::new_v4();
        let attacker = Uuid::new_v4();
        let wf = workflow(owner);
        let wf_id = wf.id;
        let step = step_in(wf_id);
        let step_id = step.id;

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(wf.clone())));

        let result = verify_step_access(&repo, attacker, wf_id, step_id).await;
        assert!(matches!(result, Err(ServiceError::NotFound(_))));
    }

    #[tokio::test]
    async fn verify_access_rejects_step_not_in_workflow() {
        let owner = Uuid::new_v4();
        let wf = workflow(owner);
        let wf_id = wf.id;
        let other_workflow_id = Uuid::new_v4();
        let step = step_in(other_workflow_id); // step belongs to a different workflow
        let step_id = step.id;

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(wf.clone())));
        repo.expect_get_step()
            .returning(move |_| Ok(Some(step.clone())));

        let result = verify_step_access(&repo, owner, wf_id, step_id).await;
        assert!(matches!(result, Err(ServiceError::NotFound(_))));
    }

    // ── create_step ───────────────────────────────────────────────────

    #[tokio::test]
    async fn create_applies_defaults() {
        let owner = Uuid::new_v4();
        let wf = workflow(owner);
        let wf_id = wf.id;

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(wf.clone())));
        repo.expect_list_steps().returning(|_| Ok(vec![]));
        repo.expect_create_step().returning(|step| Ok(step));

        let result = create_step(
            &repo,
            CreateStepInput {
                workflow_id: wf_id,
                user_id: owner,
                payload: StepPayload {
                    agent_id: None,
                    execution_mode: None,
                    for_each_ref: None,
                    prompt_template_id: None,
                    prompt_template: None,
                    output_schema_id: None,
                    output_variable_name: None,
                    interactive_agent_id: None,
                    for_each_label_field: None,
                    display_order: None,
                    reasoning_trace: None,
                    verification_agent_ids: None,
                    position_x: None,
                    position_y: None,
                    width: None,
                    height: None,
                    name: None,
                    system_prompt_suffix: None,
                    description: None,
                    sub_workflow_template_id: None,
                },
            },
        )
        .await;
        let step = result.unwrap();
        assert_eq!(step.execution_mode, "single");
        assert_eq!(step.agent_id, Some(crate::constants::DEFAULT_AGENT_ID));
        assert!(!step.reasoning_trace);
        assert_eq!(step.display_order, 0);
    }

    #[tokio::test]
    async fn create_context_step_clears_agent_id() {
        let owner = Uuid::new_v4();
        let wf = workflow(owner);
        let wf_id = wf.id;
        let explicit_agent = Uuid::new_v4();

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(wf.clone())));
        repo.expect_list_steps().returning(|_| Ok(vec![]));
        repo.expect_create_step().returning(|step| Ok(step));

        let result = create_step(
            &repo,
            CreateStepInput {
                workflow_id: wf_id,
                user_id: owner,
                payload: StepPayload {
                    agent_id: Some(explicit_agent),
                    execution_mode: Some("context".to_string()),
                    for_each_ref: None,
                    prompt_template_id: None,
                    prompt_template: None,
                    output_schema_id: None,
                    output_variable_name: None,
                    interactive_agent_id: None,
                    for_each_label_field: None,
                    display_order: None,
                    reasoning_trace: None,
                    verification_agent_ids: None,
                    position_x: None,
                    position_y: None,
                    width: None,
                    height: None,
                    name: None,
                    system_prompt_suffix: None,
                    description: None,
                    sub_workflow_template_id: None,
                },
            },
        )
        .await;
        let step = result.unwrap();
        assert_eq!(step.execution_mode, "context");
        assert_eq!(step.agent_id, None);
    }

    // ── generate_ref_id ────────────────────────────────────────────────

    #[test]
    fn generate_ref_id_first_step() {
        let ref_id = generate_ref_id(&[], "workforce");
        assert_eq!(ref_id, "workforce-1");
    }

    #[test]
    fn generate_ref_id_increments() {
        let mut s1 = step();
        s1.ref_id = Some("workforce-1".to_string());
        let mut s2 = step();
        s2.ref_id = Some("workforce-2".to_string());

        let ref_id = generate_ref_id(&[s1, s2], "workforce");
        assert_eq!(ref_id, "workforce-3");
    }

    #[test]
    fn generate_ref_id_different_modes() {
        let mut s1 = step();
        s1.ref_id = Some("workforce-1".to_string());
        let mut s2 = step();
        s2.ref_id = Some("context-1".to_string());

        // New context step should be context-2
        let ref_id = generate_ref_id(&[s1.clone(), s2], "context");
        assert_eq!(ref_id, "context-2");

        // New workforce step should be workforce-2
        let ref_id = generate_ref_id(&[s1], "workforce");
        assert_eq!(ref_id, "workforce-2");
    }

    #[test]
    fn generate_ref_id_gaps_in_sequence() {
        // If workforce-1 exists but workforce-2 was deleted, next is workforce-2 (max+1)
        let mut s1 = step();
        s1.ref_id = Some("workforce-1".to_string());
        let mut s3 = step();
        s3.ref_id = Some("workforce-3".to_string());

        let ref_id = generate_ref_id(&[s1, s3], "workforce");
        assert_eq!(ref_id, "workforce-4");
    }

    #[tokio::test]
    async fn create_step_generates_ref_id() {
        let owner = Uuid::new_v4();
        let wf = workflow(owner);
        let wf_id = wf.id;

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(wf.clone())));
        repo.expect_list_steps().returning(|_| Ok(vec![]));
        repo.expect_create_step().returning(|step| Ok(step));

        let result = create_step(
            &repo,
            CreateStepInput {
                workflow_id: wf_id,
                user_id: owner,
                payload: StepPayload {
                    execution_mode: Some("workforce".to_string()),
                    ..Default::default()
                },
            },
        )
        .await;
        let step = result.unwrap();
        assert_eq!(step.ref_id, Some("workforce-1".to_string()));
    }

    // ── get_step ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn get_rejects_wrong_owner() {
        let owner = Uuid::new_v4();
        let attacker = Uuid::new_v4();
        let wf = workflow(owner);
        let wf_id = wf.id;
        let step = step_in(wf_id);
        let step_id = step.id;

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(wf.clone())));

        let result = get_step(&repo, attacker, wf_id, step_id).await;
        assert!(matches!(result, Err(ServiceError::NotFound(_))));
    }
}
