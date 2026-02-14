#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use crate::db::BeliefRow;
    use crate::server::hub::chat_beliefs::{format_beliefs_as_board_context, parse_extraction_output};

    fn make_belief(
        node_name: &str,
        content: &str,
        belief_type: &str,
        confidence: &str,
    ) -> BeliefRow {
        BeliefRow {
            id: Uuid::new_v4(),
            workflow_id: Uuid::new_v4(),
            workflow_execution_id: None,
            source_step_id: Uuid::new_v4(),
            source_document_title: None,
            source_document_def_id: None,
            source_phase: "chat".to_string(),
            content: content.to_string(),
            reasoning: "test".to_string(),
            belief_type: belief_type.to_string(),
            confidence: confidence.to_string(),
            confidence_justification: None,
            semantic_tags: vec![],
            emotional_tone: None,
            cross_source_tension: None,
            source_step_name: node_name.to_string(),
            extraction_model: "test".to_string(),
            extraction_tokens_in: 0,
            extraction_tokens_out: 0,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn format_empty_beliefs_returns_placeholder() {
        let result = format_beliefs_as_board_context(&[]);
        assert_eq!(result, "No neighboring nodes have active conversations yet.");
    }

    #[test]
    fn format_single_node_beliefs() {
        let beliefs = vec![
            make_belief("Research Team", "User wants behavioral data", "goal", "high"),
        ];
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
}
