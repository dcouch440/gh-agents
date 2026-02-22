#[cfg(test)]
mod tests {
    use super::super::format_notes_for_summarization;

    #[test]
    fn format_notes_single_step() {
        let notes = vec![(
            "Security Scanner (workforce)".to_string(),
            "## Direction\n- Scanning repos for vulnerabilities".to_string(),
        )];
        let result = format_notes_for_summarization(&notes);
        assert_eq!(
            result,
            "[Security Scanner (workforce)]\n## Direction\n- Scanning repos for vulnerabilities"
        );
    }

    #[test]
    fn format_notes_multiple_steps() {
        let notes = vec![
            ("Alpha (workforce)".to_string(), "Alpha notes".to_string()),
            ("Beta (single)".to_string(), "Beta notes".to_string()),
        ];
        let result = format_notes_for_summarization(&notes);
        assert_eq!(
            result,
            "[Alpha (workforce)]\nAlpha notes\n\n[Beta (single)]\nBeta notes"
        );
    }

    #[test]
    fn format_notes_empty() {
        let notes: Vec<(String, String)> = vec![];
        let result = format_notes_for_summarization(&notes);
        assert!(result.is_empty());
    }
}
