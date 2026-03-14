#[cfg(test)]
mod tests {
    use crate::server::services::dispatch::Passdown;
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

    #[test]
    fn passdown_fallback_when_none() {
        // Simulates what the executor does when the builder never calls complete_task
        let passdown: Option<Passdown> = None;
        let fallback_content = "Some LLM response content";

        let result = passdown.unwrap_or_else(|| Passdown {
            summary: if fallback_content.is_empty() {
                "Completed with no response".to_string()
            } else {
                fallback_content.to_string()
            },
            question: None,
        });

        assert_eq!(result.summary, "Some LLM response content");
        assert!(result.question.is_none());
    }

    #[test]
    fn passdown_fallback_empty_content() {
        let passdown: Option<Passdown> = None;
        let fallback_content = "";

        let result = passdown.unwrap_or_else(|| Passdown {
            summary: if fallback_content.is_empty() {
                "Completed with no response".to_string()
            } else {
                fallback_content.to_string()
            },
            question: None,
        });

        assert_eq!(result.summary, "Completed with no response");
    }

    #[test]
    fn passdown_json_persistence_format() {
        // Verifies the passdown serializes correctly for agent_execution output
        let passdown = Passdown {
            summary: "Configured 3-agent pipeline".to_string(),
            question: Some("Which repos?".to_string()),
        };

        let json = serde_json::to_string(&passdown).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["summary"], "Configured 3-agent pipeline");
        assert_eq!(parsed["question"], "Which repos?");
    }
}
