#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::server::state::task_registry::{TaskRegistry, TaskStatus};

    #[test]
    fn spawn_task_creates_running_entry() {
        let registry = TaskRegistry::new();
        let step_id = Uuid::new_v4();
        let workflow_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();

        let (exec_id, _token) = registry.spawn_task(
            step_id,
            workflow_id,
            session_id,
            "Add a researcher agent".into(),
        );

        let entry = registry.get_task(exec_id).expect("task should exist");
        assert_eq!(entry.status, TaskStatus::Running);
        assert_eq!(entry.step_id, step_id);
        assert_eq!(entry.workflow_id, workflow_id);
        assert_eq!(entry.session_id, session_id);
        assert_eq!(entry.instruction, "Add a researcher agent");
        assert!(entry.result.is_none());
    }

    #[test]
    fn cancel_task_sets_cancelled_and_triggers_token() {
        let registry = TaskRegistry::new();
        let (exec_id, token) = registry.spawn_task(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            "test".into(),
        );

        assert!(!token.is_cancelled());
        let cancelled = registry.cancel_task(exec_id);
        assert!(cancelled);
        assert!(token.is_cancelled());

        let entry = registry.get_task(exec_id).unwrap();
        assert_eq!(entry.status, TaskStatus::Cancelled);
    }

    #[test]
    fn cancel_nonexistent_task_returns_false() {
        let registry = TaskRegistry::new();
        assert!(!registry.cancel_task(Uuid::new_v4()));
    }

    #[test]
    fn cancel_already_completed_task_returns_false() {
        let registry = TaskRegistry::new();
        let (exec_id, _token) = registry.spawn_task(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            "test".into(),
        );
        registry.mark_completed(exec_id, Some("done".into()));

        assert!(!registry.cancel_task(exec_id));
    }

    #[test]
    fn mark_completed_sets_status_and_summary() {
        let registry = TaskRegistry::new();
        let (exec_id, _) = registry.spawn_task(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            "test".into(),
        );

        registry.mark_completed(exec_id, Some("Added 2 agents".into()));

        let entry = registry.get_task(exec_id).unwrap();
        assert_eq!(entry.status, TaskStatus::Completed);
        assert_eq!(entry.result.as_deref(), Some("Added 2 agents"));
    }

    #[test]
    fn mark_failed_sets_status_and_error() {
        let registry = TaskRegistry::new();
        let (exec_id, _) = registry.spawn_task(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            "test".into(),
        );

        registry.mark_failed(exec_id, "DB error".into());

        let entry = registry.get_task(exec_id).unwrap();
        assert_eq!(entry.status, TaskStatus::Failed);
        assert_eq!(entry.result.as_deref(), Some("DB error"));
    }

    #[test]
    fn list_tasks_for_step_filters_and_sorts() {
        let registry = TaskRegistry::new();
        let step_a = Uuid::new_v4();
        let step_b = Uuid::new_v4();
        let wf = Uuid::new_v4();
        let sess = Uuid::new_v4();

        let (id1, _) = registry.spawn_task(step_a, wf, sess, "first".into());
        let (id2, _) = registry.spawn_task(step_a, wf, sess, "second".into());
        let (_id3, _) = registry.spawn_task(step_b, wf, sess, "other step".into());

        let tasks = registry.list_tasks_for_step(step_a);
        assert_eq!(tasks.len(), 2);
        // Newest first
        assert_eq!(tasks[0].execution_id, id2);
        assert_eq!(tasks[1].execution_id, id1);
    }

    #[test]
    fn cancel_all_cancels_only_running() {
        let registry = TaskRegistry::new();
        let wf = Uuid::new_v4();
        let sess = Uuid::new_v4();

        let (id1, token1) = registry.spawn_task(Uuid::new_v4(), wf, sess, "first".into());
        let (_id2, token2) = registry.spawn_task(Uuid::new_v4(), wf, sess, "second".into());

        // Complete the first one
        registry.mark_completed(id1, None);

        let cancelled = registry.cancel_all();
        assert_eq!(cancelled, 1);

        // First was already completed, token should not have been cancelled by cancel_all
        assert!(!token1.is_cancelled());
        // Second was running, should be cancelled
        assert!(token2.is_cancelled());
    }

    #[test]
    fn active_count_tracks_running_tasks() {
        let registry = TaskRegistry::new();
        let wf = Uuid::new_v4();
        let sess = Uuid::new_v4();

        assert_eq!(registry.active_count(), 0);

        let (id1, _) = registry.spawn_task(Uuid::new_v4(), wf, sess, "a".into());
        let (_id2, _) = registry.spawn_task(Uuid::new_v4(), wf, sess, "b".into());
        assert_eq!(registry.active_count(), 2);

        registry.mark_completed(id1, None);
        assert_eq!(registry.active_count(), 1);
    }

    #[test]
    fn cleanup_removes_old_non_running_entries() {
        let registry = TaskRegistry::new();
        let wf = Uuid::new_v4();
        let sess = Uuid::new_v4();

        let (id1, _) = registry.spawn_task(Uuid::new_v4(), wf, sess, "old".into());
        registry.mark_completed(id1, None);

        // The task was created "now", so a future cutoff should remove it
        let future = chrono::Utc::now() + chrono::Duration::hours(1);
        registry.cleanup_before(future);

        assert!(registry.get_task(id1).is_none());
    }

    #[test]
    fn cleanup_preserves_running_entries() {
        let registry = TaskRegistry::new();
        let wf = Uuid::new_v4();
        let sess = Uuid::new_v4();

        let (id1, _) = registry.spawn_task(Uuid::new_v4(), wf, sess, "running".into());

        let future = chrono::Utc::now() + chrono::Duration::hours(1);
        registry.cleanup_before(future);

        // Running tasks are preserved regardless of age
        assert!(registry.get_task(id1).is_some());
    }

    #[test]
    fn list_tasks_for_workflow_filters_and_sorts() {
        let registry = TaskRegistry::new();
        let wf = Uuid::new_v4();
        let other_wf = Uuid::new_v4();
        let sess = Uuid::new_v4();

        let (first, _) = registry.spawn_task(Uuid::new_v4(), wf, sess, "first".into());
        let (second, _) = registry.spawn_task(Uuid::new_v4(), wf, sess, "second".into());
        registry.spawn_task(Uuid::new_v4(), other_wf, sess, "elsewhere".into());

        let tasks = registry.list_tasks_for_workflow(wf);

        assert_eq!(tasks.len(), 2, "other workflows must be excluded");
        // Newest first — the frontend takes [0] as the current dispatch.
        assert_eq!(tasks[0].execution_id, second);
        assert_eq!(tasks[1].execution_id, first);
    }

    #[test]
    fn list_tasks_for_workflow_includes_terminal_entries() {
        let registry = TaskRegistry::new();
        let wf = Uuid::new_v4();
        let sess = Uuid::new_v4();

        let (done, _) = registry.spawn_task(Uuid::new_v4(), wf, sess, "done".into());
        registry.mark_completed(done, Some("summary".into()));

        let tasks = registry.list_tasks_for_workflow(wf);

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].status, TaskStatus::Completed);
        assert_eq!(tasks[0].result.as_deref(), Some("summary"));
    }

    #[test]
    fn list_tasks_for_workflow_empty_for_unknown_workflow() {
        let registry = TaskRegistry::new();
        assert!(registry.list_tasks_for_workflow(Uuid::new_v4()).is_empty());
    }
}
