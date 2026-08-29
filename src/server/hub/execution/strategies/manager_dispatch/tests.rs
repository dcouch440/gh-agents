#[cfg(test)]
mod tests {
    //! Most ManagerDispatchStrategy behaviour needs a full AppState with
    //! repos and providers, and is covered at the executor layer. What is
    //! testable here is the wiring the token ledger depends on.

    use std::sync::Arc;

    use crate::db::fixtures::fixtures::*;
    use crate::db::traits::{MockSessionRepo, MockWorkflowRepo};
    use crate::server::hub::strategy::ExecutionStrategy;
    use crate::server::state::test_helpers::MockReposBuilder;
    use crate::server::state::AppState;
    use crate::types::{AppConfig, UserId};

    /// The dispatcher held a `user_id` but never exposed it through the
    /// trait, so `on_complete` skipped the ledger and its spend went unbilled
    /// alongside the designers'.
    #[tokio::test]
    async fn the_strategy_exposes_its_user_and_execution_to_the_ledger() {
        let user_id = UserId::new();
        let wf = workflow(user_id.0);
        let workflow_id = wf.id;

        // Enough of the board for the constructor's `board_state::build`.
        let mut wf_repo = MockWorkflowRepo::new();
        wf_repo
            .expect_get_workflow()
            .returning(move |_| Ok(Some(wf.clone())));
        wf_repo.expect_list_steps().returning(|_| Ok(vec![]));
        wf_repo.expect_list_edges().returning(|_| Ok(vec![]));
        wf_repo
            .expect_get_step_question_states()
            .returning(|_| Ok(Default::default()));

        let mut session_repo = MockSessionRepo::new();
        session_repo
            .expect_check_initial_instructions_sent()
            .returning(|_| Ok(Default::default()));

        let repos = MockReposBuilder::new()
            .with_workflows(Arc::new(wf_repo))
            .with_sessions(Arc::new(session_repo))
            .build();
        let (state, _rx) = AppState::with_repos(None, repos, AppConfig::default());

        let mut strategy = super::super::ManagerDispatchStrategy::new(
            state,
            workflow_id,
            user_id,
            "dispatch it".into(),
            None,
        )
        .await
        .expect("strategy construction failed");

        assert_eq!(strategy.user_id(), Some(user_id.0));
        assert_eq!(strategy.agent_execution_id(), None);

        let ae_id = uuid::Uuid::new_v4();
        strategy.set_agent_execution_id(Some(ae_id));
        assert_eq!(strategy.agent_execution_id(), Some(ae_id));
    }
}
