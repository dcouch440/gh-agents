#[cfg(test)]
mod tests {
    use crate::server::state::task_registry::TaskStatus;

    #[test]
    fn task_status_transitions_are_consistent() {
        // This test verifies the status enum values used by the dispatch runner
        // match what the TaskRegistry expects
        assert_ne!(TaskStatus::Running, TaskStatus::Completed);
        assert_ne!(TaskStatus::Running, TaskStatus::Cancelled);
        assert_ne!(TaskStatus::Running, TaskStatus::Failed);
        assert_ne!(TaskStatus::Completed, TaskStatus::Failed);
    }

    // ── Design outcome ──────────────────────────────────────────────────────
    //
    // A design run that produced no system used to be recorded as completed,
    // with the fixed string "System node agent completed" standing in for a
    // summary the agent never wrote. The node then showed as designed, had no
    // pipeline, and killed the run when the DAG reached it.

    use crate::server::executors::dispatch::system_node::design_failure_message;

    /// The agent's own account leads. A designer that declines says why in
    /// terms the person who drew the node can act on; the sync error does not.
    #[test]
    fn a_design_failure_leads_with_what_the_agent_said() {
        let msg = design_failure_message(
            "file does not exist: topology.json",
            None,
            "I won't configure this node. The workflow exists to forge badges.",
        );

        assert!(msg.contains("I won't configure this node"), "{msg}");
        assert!(msg.contains("nothing to run"), "{msg}");
        assert!(msg.contains("file does not exist: topology.json"), "{msg}");
    }

    /// A summary — set only when `complete_system` validated — outranks the
    /// final text, which by then is the agent narrating what it just did.
    #[test]
    fn a_summary_outranks_the_final_message() {
        let msg = design_failure_message("sync failed", Some("Two agents."), "trailing chatter");
        assert!(msg.contains("Two agents."), "{msg}");
        assert!(!msg.contains("trailing chatter"), "{msg}");
    }

    /// Silence is reported as silence, not as a completed design.
    #[test]
    fn a_design_failure_with_no_explanation_says_so() {
        let msg = design_failure_message("sync failed", Some("   "), "");
        assert!(msg.contains("gave no reason"), "{msg}");
        assert!(msg.contains("sync failed"), "{msg}");
    }

    /// A refusal can run long; the message is truncated on a character
    /// boundary, not a byte one.
    #[test]
    fn a_long_explanation_is_truncated_safely() {
        let reason = "… ".repeat(2000);
        let msg = design_failure_message("sync failed", None, &reason);
        assert!(msg.chars().count() < 1200, "{}", msg.chars().count());
        assert!(msg.ends_with("(sync failed)"), "{msg}");
    }
}
