#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::super::*;

    fn make_deleted_item(name: &str, item_type: DeletedItemType) -> DeletedItem {
        DeletedItem {
            item_type,
            name: name.to_string(),
            id: Uuid::new_v4(),
            source_step_id: Uuid::new_v4(),
            source_step_name: "Source Step".to_string(),
        }
    }

    // ========================================================================
    // format_scan_input
    // ========================================================================

    #[test]
    fn format_scan_input_includes_deletions_and_notes() {
        let item = make_deleted_item("API Specifications", DeletedItemType::DocumentDef);
        let step_id = Uuid::new_v4();
        let notes = vec![(
            step_id,
            Some("Builder".to_string()),
            "task_force".to_string(),
            "Required Reading: API Specifications".to_string(),
        )];

        let output = format_scan_input(&[item.clone()], &notes);

        assert!(output.contains("<deletions>"));
        assert!(output.contains("</deletions>"));
        assert!(output.contains("<notes>"));
        assert!(output.contains("</notes>"));
        assert!(output.contains("Document \"API Specifications\""));
        assert!(output.contains(&item.id.to_string()));
        assert!(output.contains("[Builder"));
        assert!(output.contains("Required Reading: API Specifications"));
    }

    #[test]
    fn format_scan_input_handles_agent_type() {
        let item = make_deleted_item("Security Scanner", DeletedItemType::RosterAgent);
        let notes: Vec<(Uuid, Option<String>, String, String)> = vec![];

        let output = format_scan_input(&[item], &notes);

        assert!(output.contains("Agent \"Security Scanner\""));
    }

    #[test]
    fn format_scan_input_handles_unnamed_steps() {
        let items: Vec<DeletedItem> = vec![];
        let step_id = Uuid::new_v4();
        let notes = vec![(
            step_id,
            None,
            "documenter".to_string(),
            "Some notes".to_string(),
        )];

        let output = format_scan_input(&items, &notes);

        assert!(output.contains("[(unnamed)"));
    }

    #[test]
    fn format_scan_input_multiple_items() {
        let items = vec![
            make_deleted_item("Doc A", DeletedItemType::DocumentDef),
            make_deleted_item("Agent B", DeletedItemType::RosterAgent),
        ];
        let notes: Vec<(Uuid, Option<String>, String, String)> = vec![];

        let output = format_scan_input(&items, &notes);

        assert!(output.contains("Document \"Doc A\""));
        assert!(output.contains("Agent \"Agent B\""));
    }

    // ========================================================================
    // parse_scan_output
    // ========================================================================

    #[test]
    fn parse_scan_output_valid_json() {
        let step_id = Uuid::new_v4();
        let json = format!(
            r#"{{"issues": [{{"step_id": "{}", "step_name": "Builder", "description": "Notes reference deleted document 'API Specs'", "severity": "warning", "deleted_item_name": "API Specs", "deleted_item_type": "document_def"}}]}}"#,
            step_id
        );

        let issues = parse_scan_output(&json);

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].step_id, step_id);
        assert_eq!(issues[0].step_name, "Builder");
        assert_eq!(issues[0].severity, "warning");
        assert_eq!(issues[0].deleted_item_name, "API Specs");
    }

    #[test]
    fn parse_scan_output_empty_issues() {
        let json = r#"{"issues": []}"#;

        let issues = parse_scan_output(json);

        assert!(issues.is_empty());
    }

    #[test]
    fn parse_scan_output_code_fence() {
        let step_id = Uuid::new_v4();
        let json = format!(
            "Here are the results:\n```json\n{{\"issues\": [{{\"step_id\": \"{}\", \"step_name\": \"Test\", \"description\": \"stale ref\", \"severity\": \"warning\", \"deleted_item_name\": \"X\", \"deleted_item_type\": \"document_def\"}}]}}\n```",
            step_id
        );

        let issues = parse_scan_output(&json);

        assert_eq!(issues.len(), 1);
    }

    #[test]
    fn parse_scan_output_malformed_returns_empty() {
        let issues = parse_scan_output("This is not JSON at all");
        assert!(issues.is_empty());
    }

    #[test]
    fn parse_scan_output_wrong_shape_returns_empty() {
        let issues = parse_scan_output(r#"{"not_issues": true}"#);
        assert!(issues.is_empty());
    }
}
