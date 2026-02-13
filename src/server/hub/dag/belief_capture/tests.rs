#[cfg(test)]
mod tests {
    use super::super::{confidence_meets_threshold, parse_extraction_output};

    // ── confidence_meets_threshold ──────────────────────────────────────────

    #[test]
    fn confidence_high_meets_high() {
        assert!(confidence_meets_threshold("high", "high"));
    }

    #[test]
    fn confidence_high_meets_medium() {
        assert!(confidence_meets_threshold("high", "medium"));
    }

    #[test]
    fn confidence_high_meets_low() {
        assert!(confidence_meets_threshold("high", "low"));
    }

    #[test]
    fn confidence_medium_does_not_meet_high() {
        assert!(!confidence_meets_threshold("medium", "high"));
    }

    #[test]
    fn confidence_medium_meets_medium() {
        assert!(confidence_meets_threshold("medium", "medium"));
    }

    #[test]
    fn confidence_medium_meets_low() {
        assert!(confidence_meets_threshold("medium", "low"));
    }

    #[test]
    fn confidence_low_does_not_meet_high() {
        assert!(!confidence_meets_threshold("low", "high"));
    }

    #[test]
    fn confidence_low_does_not_meet_medium() {
        assert!(!confidence_meets_threshold("low", "medium"));
    }

    #[test]
    fn confidence_low_meets_low() {
        assert!(confidence_meets_threshold("low", "low"));
    }

    #[test]
    fn confidence_unknown_does_not_meet_any() {
        assert!(!confidence_meets_threshold("unknown", "low"));
        assert!(!confidence_meets_threshold("unknown", "medium"));
        assert!(!confidence_meets_threshold("unknown", "high"));
    }

    // ── parse_extraction_output ─────────────────────────────────────────────

    #[test]
    fn parse_extraction_output_valid_full() {
        let content = r#"```json
{
  "beliefs": [
    {
      "content": "The system uses microservices architecture",
      "reasoning": "Multiple references to independent service deployments",
      "belief_type": "factual",
      "confidence": "high",
      "confidence_justification": "Directly stated in the document",
      "semantic_tags": ["architecture", "microservices"],
      "emotional_tone": "neutral",
      "cross_source_tension": null
    }
  ]
}
```"#;
        let beliefs = parse_extraction_output(content);
        assert_eq!(beliefs.len(), 1);
        assert_eq!(beliefs[0].content, "The system uses microservices architecture");
        assert_eq!(beliefs[0].belief_type, "factual");
        assert_eq!(beliefs[0].confidence, "high");
        assert_eq!(
            beliefs[0].confidence_justification.as_deref(),
            Some("Directly stated in the document")
        );
        assert_eq!(beliefs[0].semantic_tags, vec!["architecture", "microservices"]);
        assert_eq!(beliefs[0].emotional_tone.as_deref(), Some("neutral"));
        assert!(beliefs[0].cross_source_tension.is_none());
    }

    #[test]
    fn parse_extraction_output_minimal_fields() {
        let content = r#"```json
{
  "beliefs": [
    {
      "content": "Users prefer dark mode",
      "reasoning": "Survey data shows 70% preference",
      "belief_type": "preference",
      "confidence": "medium"
    }
  ]
}
```"#;
        let beliefs = parse_extraction_output(content);
        assert_eq!(beliefs.len(), 1);
        assert_eq!(beliefs[0].content, "Users prefer dark mode");
        assert_eq!(beliefs[0].confidence, "medium");
        assert!(beliefs[0].confidence_justification.is_none());
        assert!(beliefs[0].semantic_tags.is_empty());
        assert!(beliefs[0].emotional_tone.is_none());
        assert!(beliefs[0].cross_source_tension.is_none());
    }

    #[test]
    fn parse_extraction_output_multiple_beliefs() {
        let content = r#"```json
{
  "beliefs": [
    {
      "content": "First belief",
      "reasoning": "Reason 1",
      "belief_type": "factual",
      "confidence": "high"
    },
    {
      "content": "Second belief",
      "reasoning": "Reason 2",
      "belief_type": "opinion",
      "confidence": "low"
    }
  ]
}
```"#;
        let beliefs = parse_extraction_output(content);
        assert_eq!(beliefs.len(), 2);
        assert_eq!(beliefs[0].content, "First belief");
        assert_eq!(beliefs[1].content, "Second belief");
    }

    #[test]
    fn parse_extraction_output_empty_beliefs_array() {
        let content = r#"```json
{
  "beliefs": []
}
```"#;
        let beliefs = parse_extraction_output(content);
        assert!(beliefs.is_empty());
    }

    #[test]
    fn parse_extraction_output_malformed_json() {
        let content = "This is not JSON at all, just plain text.";
        let beliefs = parse_extraction_output(content);
        assert!(beliefs.is_empty());
    }

    #[test]
    fn parse_extraction_output_wrong_schema() {
        let content = r#"```json
{
  "wrong_key": "not beliefs"
}
```"#;
        let beliefs = parse_extraction_output(content);
        assert!(beliefs.is_empty());
    }

    #[test]
    fn parse_extraction_output_raw_json_no_fences() {
        let content = r#"{"beliefs": [{"content": "Raw belief", "reasoning": "Direct", "belief_type": "factual", "confidence": "high"}]}"#;
        let beliefs = parse_extraction_output(content);
        assert_eq!(beliefs.len(), 1);
        assert_eq!(beliefs[0].content, "Raw belief");
    }
}
