#[cfg(test)]
mod tests {
    use super::super::{
        format_capability_descriptions, format_roster_for_designer, format_upstream_for_designer,
        truncate_for_context, DesignerOutputSchema,
    };
    use crate::db::TaskAgentRosterRow;
    use crate::types::StepExecutionEnvelope;
    use chrono::Utc;
    use std::collections::HashMap;
    use uuid::Uuid;

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
                role_description: "Analyzes findings and prioritizes".to_string(),
                capabilities: vec!["file_read".to_string()],
                execution_order: 1,
                created_at: Utc::now(),
            },
            TaskAgentRosterRow {
                id: Uuid::parse_str("cccccccc-cccc-cccc-cccc-cccccccccccc").unwrap(),
                mission_brief_id: Uuid::new_v4(),
                name: "Reporter".to_string(),
                role_description: "Produces the final report".to_string(),
                capabilities: vec!["file_write".to_string()],
                execution_order: 2,
                created_at: Utc::now(),
            },
        ]
    }

    // ── format_roster_for_designer ──────────────────────────────────────

    #[test]
    fn format_roster_for_designer_basic() {
        let roster = make_roster();
        let result = format_roster_for_designer(&roster);

        assert!(result.contains("1. Scanner"));
        assert!(result.contains("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa"));
        assert!(result.contains("Role: Scans the codebase for issues"));
        assert!(result.contains("Execution Order: 0"));

        assert!(result.contains("2. Analyzer"));
        assert!(result.contains("3. Reporter"));
    }

    #[test]
    fn format_roster_for_designer_empty() {
        let result = format_roster_for_designer(&[]);
        assert!(result.is_empty());
    }

    // ── format_upstream_for_designer ────────────────────────────────────

    #[test]
    fn format_upstream_for_designer_empty() {
        let envelopes = HashMap::new();
        let result = format_upstream_for_designer(&envelopes);
        assert!(result.contains("No upstream outputs available"));
    }

    #[test]
    fn format_upstream_for_designer_with_data() {
        use crate::types::{ExecutionMetadata, ExecutionStatus};

        let mut envelopes = HashMap::new();
        let step_id = Uuid::new_v4();
        envelopes.insert(
            step_id,
            StepExecutionEnvelope {
                status: ExecutionStatus::Success,
                data: Some(serde_json::json!({"result": "found 3 issues"})),
                metadata: ExecutionMetadata {
                    execution_id: step_id,
                    execution_time_ms: 100,
                    tokens_in: Some(500),
                    tokens_out: Some(200),
                    cost_usd: Some(0.01),
                    model: None,
                    agent_id: None,
                    iteration_index: None,
                    iteration_label: None,
                    routing_label: None,
                    upstream_agent_id: None,
                    upstream_routing_label: None,
                    room_session_id: None,
                    room_id: None,
                    total_rounds: None,
                },
                error: None,
            },
        );

        let result = format_upstream_for_designer(&envelopes);
        assert!(result.contains("<upstream_step"));
        assert!(result.contains("found 3 issues"));
    }

    // ── format_capability_descriptions ──────────────────────────────────

    #[test]
    fn format_capability_descriptions_known() {
        let caps = vec![
            "file_read".to_string(),
            "grep".to_string(),
            "shell".to_string(),
        ];
        let result = format_capability_descriptions(&caps);
        assert!(result.contains("file_read: Read file contents"));
        assert!(result.contains("grep: Search file contents"));
        assert!(result.contains("shell: Execute shell commands"));
    }

    #[test]
    fn format_capability_descriptions_unknown() {
        let caps = vec!["custom_tool".to_string()];
        let result = format_capability_descriptions(&caps);
        assert!(result.contains("- custom_tool"));
    }

    // ── truncate_for_context ────────────────────────────────────────────

    #[test]
    fn truncate_short_content() {
        let result = truncate_for_context("hello", 100);
        assert_eq!(result, "hello");
    }

    #[test]
    fn truncate_long_content() {
        let result = truncate_for_context("hello world", 5);
        assert_eq!(result, "hello");
    }

    #[test]
    fn truncate_at_zero() {
        let result = truncate_for_context("hello", 0);
        assert_eq!(result, "");
    }

    // ── DesignerOutputSchema parsing ────────────────────────────────────

    #[test]
    fn parse_designer_output_valid() {
        let json = r#"{
            "agents": [{
                "agent_id": "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
                "agent_name": "Scanner",
                "tools": ["file_read", "grep"],
                "system_prompt": "You are a scanner...",
                "task_prompt": "Scan the repo for...",
                "reasoning": "Identity framing emphasizes thoroughness..."
            }]
        }"#;
        let parsed: DesignerOutputSchema = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.agents.len(), 1);
        assert_eq!(parsed.agents[0].agent_name, "Scanner");
        assert_eq!(parsed.agents[0].tools, vec!["file_read", "grep"]);
    }

    #[test]
    fn parse_designer_output_multiple_agents() {
        let json = r#"{
            "agents": [
                {
                    "agent_id": "aaa",
                    "agent_name": "Scanner",
                    "tools": ["file_read"],
                    "system_prompt": "sys1",
                    "task_prompt": "task1",
                    "reasoning": "r1"
                },
                {
                    "agent_id": "bbb",
                    "agent_name": "Analyzer",
                    "tools": [],
                    "system_prompt": "sys2",
                    "task_prompt": "task2",
                    "reasoning": "r2"
                }
            ]
        }"#;
        let parsed: DesignerOutputSchema = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.agents.len(), 2);
        assert_eq!(parsed.agents[1].agent_name, "Analyzer");
        assert!(parsed.agents[1].tools.is_empty());
    }

    #[test]
    fn parse_designer_output_malformed() {
        let json = r#"{"agents": "not an array"}"#;
        let result: Result<DesignerOutputSchema, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn parse_designer_output_missing_field() {
        let json = r#"{"agents": [{"agent_name": "Scanner"}]}"#;
        let result: Result<DesignerOutputSchema, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }
}
