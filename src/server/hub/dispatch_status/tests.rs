#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::server::hub::dispatch_status::render;
    use crate::server::hub::dispatch_status::types::*;
    use crate::server::state::task_registry::TaskRegistry;

    // ========================================================================
    // Test helpers
    // ========================================================================

    fn make_snapshot(
        id: &str,
        instruction: &str,
        status: DispatchStatus,
        elapsed: &str,
        result: Option<&str>,
    ) -> DispatchSnapshot {
        DispatchSnapshot {
            id: id.to_string(),
            instruction: instruction.to_string(),
            status,
            elapsed: elapsed.to_string(),
            result: result.map(|s| s.to_string()),
        }
    }

    // ========================================================================
    // Render tests (pure, no TaskRegistry)
    // ========================================================================

    #[test]
    fn render_empty() {
        let result = render::render(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn render_single_running() {
        let snaps = vec![make_snapshot(
            "a1b2c3d4",
            "Set up pricing pipeline",
            DispatchStatus::InProgress,
            "45s ago",
            None,
        )];
        let xml = render::render(&snaps);

        assert!(xml.starts_with("<dispatch_status>\n"));
        assert!(xml.ends_with("</dispatch_status>\n"));
        assert!(xml.contains("id=\"a1b2c3d4\""));
        assert!(xml.contains("instruction=\"Set up pricing pipeline\""));
        assert!(xml.contains("status=\"in_progress\""));
        assert!(xml.contains("started=\"45s ago\""));
        // Self-closing — no children
        assert!(xml.contains("/>"));
        // Should NOT have "completed" attr
        assert!(!xml.contains("completed="));
        assert!(!xml.contains("result="));
    }

    #[test]
    fn render_completed_with_result() {
        let snaps = vec![make_snapshot(
            "x9y8z7w6",
            "Configure analysis team",
            DispatchStatus::Completed,
            "5m ago",
            Some("Added 3 agents"),
        )];
        let xml = render::render(&snaps);

        assert!(xml.contains("status=\"completed\""));
        assert!(xml.contains("completed=\"5m ago\""));
        assert!(xml.contains("result=\"Added 3 agents\""));
        // Should NOT have "started" attr
        assert!(!xml.contains("started="));
    }

    #[test]
    fn render_failed() {
        let snaps = vec![make_snapshot(
            "m5n6o7p8",
            "Migration task",
            DispatchStatus::Failed,
            "3m ago",
            Some("Database connection timeout"),
        )];
        let xml = render::render(&snaps);

        assert!(xml.contains("status=\"failed\""));
        assert!(xml.contains("completed=\"3m ago\""));
        assert!(xml.contains("result=\"Database connection timeout\""));
    }

    #[test]
    fn render_cancelled_no_result() {
        let snaps = vec![make_snapshot(
            "c4d5e6f7",
            "Cancelled job",
            DispatchStatus::Cancelled,
            "1m ago",
            None,
        )];
        let xml = render::render(&snaps);

        assert!(xml.contains("status=\"cancelled\""));
        assert!(!xml.contains("result="));
    }

    #[test]
    fn render_mixed_running_and_completed() {
        let snaps = vec![
            make_snapshot(
                "a1b2c3d4",
                "Current task",
                DispatchStatus::InProgress,
                "30s ago",
                None,
            ),
            make_snapshot(
                "x9y8z7w6",
                "Previous task",
                DispatchStatus::Completed,
                "5m ago",
                Some("Done"),
            ),
        ];
        let xml = render::render(&snaps);

        // Both dispatches present
        assert!(xml.contains("id=\"a1b2c3d4\""));
        assert!(xml.contains("id=\"x9y8z7w6\""));
        // Running one uses "started", completed uses "completed"
        assert!(xml.contains("started=\"30s ago\""));
        assert!(xml.contains("completed=\"5m ago\""));
    }

    #[test]
    fn render_escapes_xml_in_instruction() {
        let snaps = vec![make_snapshot(
            "e1f2g3h4",
            "Research A & B <Corp>",
            DispatchStatus::InProgress,
            "10s ago",
            None,
        )];
        let xml = render::render(&snaps);

        assert!(xml.contains("instruction=\"Research A &amp; B &lt;Corp&gt;\""));
    }

    // ========================================================================
    // DispatchStatus methods
    // ========================================================================

    #[test]
    fn status_as_str() {
        assert_eq!(DispatchStatus::InProgress.as_str(), "in_progress");
        assert_eq!(DispatchStatus::Completed.as_str(), "completed");
        assert_eq!(DispatchStatus::Failed.as_str(), "failed");
        assert_eq!(DispatchStatus::Cancelled.as_str(), "cancelled");
    }

    #[test]
    fn status_is_terminal() {
        assert!(!DispatchStatus::InProgress.is_terminal());
        assert!(DispatchStatus::Completed.is_terminal());
        assert!(DispatchStatus::Failed.is_terminal());
        assert!(DispatchStatus::Cancelled.is_terminal());
    }

    // ========================================================================
    // Integration tests (with TaskRegistry)
    // ========================================================================

    #[test]
    fn build_empty_registry() {
        let registry = TaskRegistry::new();
        let result = crate::server::hub::dispatch_status::build(&registry, Uuid::new_v4());
        assert!(result.is_empty());
    }

    #[test]
    fn build_running_task() {
        let registry = TaskRegistry::new();
        let step_id = Uuid::new_v4();

        registry.spawn_task(
            step_id,
            Uuid::new_v4(),
            Uuid::new_v4(),
            "Add a researcher agent".to_string(),
        );

        let result = crate::server::hub::dispatch_status::build(&registry, step_id);
        assert!(result.contains("<dispatch_status>"));
        assert!(result.contains("status=\"in_progress\""));
        assert!(result.contains("Add a researcher agent"));
    }

    #[test]
    fn build_completed_task() {
        let registry = TaskRegistry::new();
        let step_id = Uuid::new_v4();

        let (exec_id, _) = registry.spawn_task(
            step_id,
            Uuid::new_v4(),
            Uuid::new_v4(),
            "Set up the team".to_string(),
        );
        registry.mark_completed(exec_id, Some("Added 3 agents".to_string()));

        let result = crate::server::hub::dispatch_status::build(&registry, step_id);
        assert!(result.contains("status=\"completed\""));
        assert!(result.contains("result=\"Added 3 agents\""));
    }

    #[test]
    fn build_ignores_other_steps() {
        let registry = TaskRegistry::new();
        let step_a = Uuid::new_v4();
        let step_b = Uuid::new_v4();

        registry.spawn_task(
            step_a,
            Uuid::new_v4(),
            Uuid::new_v4(),
            "task for step A".to_string(),
        );

        let result = crate::server::hub::dispatch_status::build(&registry, step_b);
        assert!(result.is_empty());
    }
}
