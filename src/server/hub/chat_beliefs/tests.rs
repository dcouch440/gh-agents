#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::db::fixtures::fixtures::*;
    use crate::db::BeliefRow;
    use crate::server::hub::chat_beliefs::{
        format_beliefs_as_board_context, format_beliefs_for_extraction, parse_extraction_output,
    };

    fn make_belief(
        node_name: &str,
        content: &str,
        belief_type: &str,
        confidence: &str,
    ) -> BeliefRow {
        let wf_id = Uuid::new_v4();
        let step_id = Uuid::new_v4();
        BeliefRow {
            content: content.to_string(),
            reasoning: "test".to_string(),
            belief_type: belief_type.to_string(),
            confidence: confidence.to_string(),
            source_phase: "chat".to_string(),
            source_step_name: node_name.to_string(),
            extraction_model: "test".to_string(),
            ..belief(wf_id, step_id)
        }
    }

    fn make_belief_with_tension(
        node_name: &str,
        content: &str,
        belief_type: &str,
        confidence: &str,
        tension: &str,
    ) -> BeliefRow {
        let mut b = make_belief(node_name, content, belief_type, confidence);
        b.cross_source_tension = Some(tension.to_string());
        b
    }

    // ── Board context formatting ────────────────────────────────────────

    #[test]
    fn format_empty_beliefs_returns_placeholder() {
        let result = format_beliefs_as_board_context(&[]);
        assert_eq!(
            result,
            "No neighboring nodes have active conversations yet."
        );
    }

    #[test]
    fn format_single_node_beliefs() {
        let beliefs = vec![make_belief(
            "Research Team",
            "User wants behavioral data",
            "goal",
            "high",
        )];
        let result = format_beliefs_as_board_context(&beliefs);
        assert!(result.contains("Research Team:"));
        assert!(result.contains("- User wants behavioral data [goal]"));
    }

    #[test]
    fn format_multi_node_grouped() {
        let beliefs = vec![
            make_belief("Alpha", "First belief", "goal", "high"),
            make_belief("Beta", "Second belief", "requirement", "high"),
            make_belief("Alpha", "Third belief", "fact", "medium"),
        ];
        let result = format_beliefs_as_board_context(&beliefs);

        // Should be sorted alphabetically by node name
        let alpha_pos = result.find("Alpha:").unwrap();
        let beta_pos = result.find("Beta:").unwrap();
        assert!(alpha_pos < beta_pos);

        // Alpha should have both beliefs
        assert!(result.contains("First belief"));
        assert!(result.contains("Third belief"));
    }

    #[test]
    fn format_confidence_markers() {
        let beliefs = vec![
            make_belief("Node", "High confidence", "goal", "high"),
            make_belief("Node", "Medium confidence", "goal", "medium"),
            make_belief("Node", "Low confidence", "goal", "low"),
        ];
        let result = format_beliefs_as_board_context(&beliefs);

        // High confidence: just [type]
        assert!(result.contains("High confidence [goal]"));
        // Medium: [type, medium]
        assert!(result.contains("Medium confidence [goal, medium]"));
        // Low: [type, low]
        assert!(result.contains("Low confidence [goal, low]"));
    }

    #[test]
    fn format_includes_correction_beliefs_with_tension() {
        // A correction belief (with SUPERSEDED) should still appear —
        // it's the new, correct belief. The tension is informational.
        let beliefs = vec![
            make_belief(
                "Documenter",
                "SVG specs needed for design layout",
                "requirement",
                "high",
            ),
            make_belief_with_tension(
                "Documenter",
                "Project is about SVG icons for four application groups",
                "goal",
                "high",
                "SUPERSEDED: The project is about cats and dogs",
            ),
        ];
        let result = format_beliefs_as_board_context(&beliefs);

        assert!(result.contains("SVG specs needed for design layout"));
        assert!(result.contains("Project is about SVG icons for four application groups"));
    }

    // ── Extraction context formatting ───────────────────────────────────

    #[test]
    fn extraction_format_empty_returns_placeholder() {
        let result = format_beliefs_for_extraction(&[]);
        assert_eq!(result, "No beliefs from other nodes yet.");
    }

    #[test]
    fn extraction_format_single_node() {
        let beliefs = vec![make_belief(
            "Documenter",
            "Creating SVG character graphics",
            "goal",
            "high",
        )];
        let result = format_beliefs_for_extraction(&beliefs);

        assert!(result.contains("[Documenter]"));
        assert!(result.contains("- Creating SVG character graphics (goal)"));
    }

    #[test]
    fn extraction_format_multi_node() {
        let beliefs = vec![
            make_belief("Alpha", "First", "goal", "high"),
            make_belief("Beta", "Second", "requirement", "medium"),
        ];
        let result = format_beliefs_for_extraction(&beliefs);

        assert!(result.contains("[Alpha]"));
        assert!(result.contains("[Beta]"));
        assert!(result.contains("- First (goal)"));
        assert!(result.contains("- Second (requirement)"));
    }

    #[test]
    fn extraction_format_includes_all_beliefs() {
        // All beliefs passed to extraction — including corrections — so Haiku
        // has the full picture of what the board currently believes.
        let beliefs = vec![
            make_belief("Node", "Current belief", "goal", "high"),
            make_belief_with_tension(
                "Node",
                "Corrected belief",
                "goal",
                "high",
                "SUPERSEDED: old direction",
            ),
        ];
        let result = format_beliefs_for_extraction(&beliefs);

        assert!(result.contains("Current belief"));
        assert!(result.contains("Corrected belief"));
    }

    // ── Parsing ─────────────────────────────────────────────────────────

    #[test]
    fn parse_extraction_output_valid_json() {
        let json = r#"{"beliefs": [{"content": "test belief", "reasoning": "said it", "belief_type": "goal", "confidence": "high"}]}"#;
        let result = parse_extraction_output(json);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].content, "test belief");
        assert_eq!(result[0].belief_type, "goal");
    }

    #[test]
    fn parse_extraction_output_code_fenced() {
        let json = "```json\n{\"beliefs\": [{\"content\": \"test\", \"reasoning\": \"r\", \"belief_type\": \"fact\", \"confidence\": \"low\"}]}\n```";
        let result = parse_extraction_output(json);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn parse_extraction_output_invalid_returns_empty() {
        let result = parse_extraction_output("not json at all");
        assert!(result.is_empty());
    }

    #[test]
    fn parse_extraction_output_empty_beliefs() {
        let json = r#"{"beliefs": []}"#;
        let result = parse_extraction_output(json);
        assert!(result.is_empty());
    }

    #[test]
    fn parse_extraction_output_empty_object() {
        // Grok may return {} without a beliefs key — should gracefully return empty
        let result = parse_extraction_output("{}");
        assert!(result.is_empty());
    }

    #[test]
    fn parse_extraction_output_direct_array() {
        // Some models return a bare array instead of {"beliefs": [...]}
        let json = r#"[{"content": "direct", "reasoning": "r", "belief_type": "goal", "confidence": "high"}]"#;
        let result = parse_extraction_output(json);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].content, "direct");
    }

    #[test]
    fn parse_extraction_output_with_cross_source_tension() {
        let json = r#"{"beliefs": [{"content": "New direction", "reasoning": "user pivoted", "belief_type": "goal", "confidence": "high", "cross_source_tension": "SUPERSEDED: old cats and dogs idea"}]}"#;
        let result = parse_extraction_output(json);
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].cross_source_tension.as_deref(),
            Some("SUPERSEDED: old cats and dogs idea")
        );
    }
}
