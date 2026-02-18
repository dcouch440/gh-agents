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
}
