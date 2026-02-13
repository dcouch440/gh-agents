#[cfg(test)]
mod tests {
    use crate::server::hub::dag::designer_input::{
        AgentDefinition, ToolDescription, UpstreamContext,
    };

    use super::super::{
        format_agent_definitions, format_tool_descriptions, format_upstream_context,
    };

    #[test]
    fn test_format_agent_definitions_basic() {
        let agents = vec![AgentDefinition {
            id: "abc-123".into(),
            name: "Scanner".into(),
            role: "Scans files for issues".into(),
            capabilities: vec!["file_read".into(), "grep".into()],
            execution_order: 0,
            additional_context: String::new(),
        }];

        let result = format_agent_definitions(&agents);
        assert!(result.contains("1. Scanner"));
        assert!(result.contains("id: abc-123"));
        assert!(result.contains("Role: Scans files for issues"));
        assert!(result.contains("file_read, grep"));
    }

    #[test]
    fn test_format_agent_definitions_with_additional_context() {
        let agents = vec![AgentDefinition {
            id: "def-456".into(),
            name: "Analyst".into(),
            role: "Analyzes data".into(),
            capabilities: vec![],
            execution_order: 1,
            additional_context: "Focus on security issues.\nCheck for OWASP top 10.".into(),
        }];

        let result = format_agent_definitions(&agents);
        assert!(result.contains("Additional context:"));
        assert!(result.contains("Focus on security issues."));
        // Should NOT contain "Capabilities:" when empty
        assert!(!result.contains("Capabilities:"));
    }

    #[test]
    fn test_format_agent_definitions_multiple() {
        let agents = vec![
            AgentDefinition {
                id: "a".into(),
                name: "First".into(),
                role: "Does stuff".into(),
                capabilities: vec![],
                execution_order: 0,
                additional_context: String::new(),
            },
            AgentDefinition {
                id: "b".into(),
                name: "Second".into(),
                role: "Does more stuff".into(),
                capabilities: vec!["shell".into()],
                execution_order: 1,
                additional_context: String::new(),
            },
        ];

        let result = format_agent_definitions(&agents);
        assert!(result.contains("1. First"));
        assert!(result.contains("2. Second"));
    }

    #[test]
    fn test_format_upstream_context_basic() {
        let upstream = vec![UpstreamContext {
            source_name: "step_1".into(),
            source_type: "context".into(),
            content: "Some upstream content".into(),
        }];

        let result = format_upstream_context(&upstream);
        assert!(result.contains("<upstream source=\"step_1\" type=\"context\">"));
        assert!(result.contains("Some upstream content"));
        assert!(result.contains("</upstream>"));
    }

    #[test]
    fn test_format_upstream_context_empty() {
        let result = format_upstream_context(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_format_tool_descriptions_known() {
        let tools = vec![
            ToolDescription {
                name: "file_read".into(),
                description: "Read file contents from the repository".into(),
            },
            ToolDescription {
                name: "grep".into(),
                description: "Search file contents with regex patterns".into(),
            },
        ];

        let result = format_tool_descriptions(&tools);
        assert!(result.contains("- file_read: Read file contents"));
        assert!(result.contains("- grep: Search file contents"));
    }

    #[test]
    fn test_format_tool_descriptions_empty() {
        let result = format_tool_descriptions(&[]);
        assert!(result.contains("No tools available"));
    }

    // ── DesignerOutputSchema parsing ────────────────────────────────────

    use super::super::DesignerOutputSchema;

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

    #[test]
    fn parse_designer_output_with_receives_from() {
        let json = r#"{
            "agents": [{
                "agent_id": "aaa",
                "agent_name": "Analyzer",
                "tools": ["file_read"],
                "system_prompt": "sys",
                "task_prompt": "task",
                "reasoning": "r",
                "receives_from": ["Scanner"]
            }]
        }"#;
        let parsed: DesignerOutputSchema = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.agents[0].receives_from, vec!["Scanner"]);
    }

    #[test]
    fn parse_designer_output_without_receives_from_defaults_empty() {
        let json = r#"{
            "agents": [{
                "agent_id": "aaa",
                "agent_name": "Scanner",
                "tools": [],
                "system_prompt": "sys",
                "task_prompt": "task",
                "reasoning": "r"
            }]
        }"#;
        let parsed: DesignerOutputSchema = serde_json::from_str(json).unwrap();
        assert!(parsed.agents[0].receives_from.is_empty());
    }

    // ── normalize_agent_name ─────────────────────────────────────────────

    use super::super::normalize_agent_name;
    use super::super::validate_receives_from;

    #[test]
    fn normalize_agent_name_cases() {
        let canonical = normalize_agent_name("SecurityAuditor");
        assert_eq!(canonical, "securityauditor");
        assert_eq!(normalize_agent_name("security_auditor"), canonical);
        assert_eq!(normalize_agent_name("security-auditor"), canonical);
        assert_eq!(normalize_agent_name("Security Auditor"), canonical);
        assert_eq!(normalize_agent_name("SECURITY_AUDITOR"), canonical);
    }

    // ── validate_receives_from ───────────────────────────────────────────

    #[test]
    fn validate_receives_from_exact_match() {
        let names = vec!["Scanner".to_string(), "Analyzer".to_string()];
        let result = validate_receives_from(&["Scanner".to_string()], &names, "Analyzer");
        assert_eq!(result, vec!["Scanner"]);
    }

    #[test]
    fn validate_receives_from_case_mismatch() {
        let names = vec!["SecurityAuditor".to_string(), "Reporter".to_string()];
        let result =
            validate_receives_from(&["security_auditor".to_string()], &names, "Reporter");
        assert_eq!(result, vec!["SecurityAuditor"]);
    }

    #[test]
    fn validate_receives_from_unknown_agent() {
        let names = vec!["Scanner".to_string(), "Analyzer".to_string()];
        let result = validate_receives_from(&["NonExistent".to_string()], &names, "Analyzer");
        assert!(result.is_empty());
    }

    #[test]
    fn validate_receives_from_mixed() {
        let names = vec![
            "Scanner".to_string(),
            "Analyzer".to_string(),
            "Reporter".to_string(),
        ];
        let receives = vec![
            "scanner".to_string(),
            "BadName".to_string(),
            "REPORTER".to_string(),
        ];
        let result = validate_receives_from(&receives, &names, "Worker");
        assert_eq!(result, vec!["Scanner", "Reporter"]);
    }
}
