#[cfg(test)]
mod tests {
    // The formatting and parsing tests moved to agent_designer/tests.rs
    // since the generic module now owns those functions.
    // This file tests the task-force-specific mapping layer.

    use crate::db::{TaskAgentRosterRow, TaskMissionBriefRow};
    use crate::server::hub::dag::designer_input::task_force::build_task_force_designer_input;
    use chrono::Utc;
    use std::collections::HashMap;
    use uuid::Uuid;

    fn make_brief() -> TaskMissionBriefRow {
        TaskMissionBriefRow {
            id: Uuid::new_v4(),
            step_id: Uuid::new_v4(),
            task_description: "Review the auth system".to_string(),
            failure_mode: "halt_and_report".to_string(),
            available_capabilities: vec!["file_read".to_string(), "grep".to_string()],
            downstream_context: Some("Results feed into the patcher".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn make_roster() -> Vec<TaskAgentRosterRow> {
        vec![
            TaskAgentRosterRow {
                id: Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap(),
                mission_brief_id: Uuid::new_v4(),
                name: "Scanner".to_string(),
                role_description: "Scans the codebase for issues".to_string(),
                capabilities: vec!["file_read".to_string(), "grep".to_string()],
                execution_order: 0,
                created_at: Utc::now(),
            },
            TaskAgentRosterRow {
                id: Uuid::parse_str("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb").unwrap(),
                mission_brief_id: Uuid::new_v4(),
                name: "Analyzer".to_string(),
                role_description: "Analyzes findings".to_string(),
                capabilities: vec!["file_read".to_string()],
                execution_order: 1,
                created_at: Utc::now(),
            },
        ]
    }

    #[test]
    fn build_task_force_input_sets_archetype() {
        let brief = make_brief();
        let roster = make_roster();
        let input = build_task_force_designer_input(&brief, &roster, &HashMap::new());
        assert_eq!(input.archetype, "task_force");
    }

    #[test]
    fn build_task_force_input_maps_roster_to_agents() {
        let brief = make_brief();
        let roster = make_roster();
        let input = build_task_force_designer_input(&brief, &roster, &HashMap::new());
        assert_eq!(input.agents.len(), 2);
        assert_eq!(input.agents[0].name, "Scanner");
        assert_eq!(input.agents[0].id, "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa");
        assert_eq!(input.agents[1].name, "Analyzer");
    }

    #[test]
    fn build_task_force_input_includes_guidance() {
        let brief = make_brief();
        let roster = make_roster();
        let input = build_task_force_designer_input(&brief, &roster, &HashMap::new());
        assert!(input.archetype_guidance.contains("halt_and_report"));
        assert!(input
            .archetype_guidance
            .contains("Results feed into the patcher"));
    }

    #[test]
    fn build_task_force_input_includes_tools() {
        let brief = make_brief();
        let roster = make_roster();
        let input = build_task_force_designer_input(&brief, &roster, &HashMap::new());
        assert_eq!(input.available_tools.len(), 2);
        assert_eq!(input.available_tools[0].name, "file_read");
    }
}
