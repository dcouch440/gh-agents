#[cfg(test)]
mod tests {
    use super::super::{
        build_previous_outputs_block, build_team_roster_string, compose_combined_output,
    };
    use uuid::Uuid;

    #[test]
    fn build_team_roster_string_formats_agents() {
        use chrono::Utc;

        let roster = vec![
            crate::db::TaskAgentRosterRow {
                id: Uuid::new_v4(),
                mission_brief_id: Uuid::new_v4(),
                name: "Scanner".to_string(),
                role_description: "Scans the codebase for issues".to_string(),
                capabilities: vec!["file_read".to_string(), "grep".to_string()],
                execution_order: 0,
                created_at: Utc::now(),
            },
            crate::db::TaskAgentRosterRow {
                id: Uuid::new_v4(),
                mission_brief_id: Uuid::new_v4(),
                name: "Analyzer".to_string(),
                role_description: "Analyzes findings".to_string(),
                capabilities: vec![],
                execution_order: 1,
                created_at: Utc::now(),
            },
        ];

        let result = build_team_roster_string(&roster);
        assert!(result.contains("**Scanner** (order 0)"));
        assert!(result.contains("[file_read, grep]"));
        assert!(result.contains("**Analyzer** (order 1)"));
        // Analyzer has no caps, so no brackets
        let analyzer_line = result.lines().find(|l| l.contains("Analyzer")).unwrap();
        assert!(!analyzer_line.contains('['));
    }

    #[test]
    fn build_previous_outputs_block_empty() {
        let result = build_previous_outputs_block(&[]);
        assert!(result.contains("first agent"));
    }

    #[test]
    fn build_previous_outputs_block_with_entries() {
        let outputs = vec![
            ("Scanner".to_string(), "Found 3 issues.".to_string()),
            ("Analyzer".to_string(), "Prioritized issues.".to_string()),
        ];
        let result = build_previous_outputs_block(&outputs);
        assert!(result.contains("### Scanner"));
        assert!(result.contains("Found 3 issues."));
        assert!(result.contains("### Analyzer"));
        assert!(result.contains("Prioritized issues."));
    }

    #[test]
    fn compose_combined_output_basic() {
        let outputs = vec![
            ("Code Scanner".to_string(), "raw text output".to_string()),
            (
                "Analyzer".to_string(),
                r#"{"priority": "high"}"#.to_string(),
            ),
        ];
        let result = compose_combined_output(&outputs);
        let obj = result.as_object().unwrap();

        // Key is lowercased with spaces replaced by underscores
        assert!(obj.contains_key("code_scanner"));
        assert_eq!(obj["code_scanner"], "raw text output");

        // JSON output is parsed
        assert!(obj.contains_key("analyzer"));
        assert_eq!(obj["analyzer"]["priority"], "high");
    }
}
