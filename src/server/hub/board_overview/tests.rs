#[cfg(test)]
mod tests {
    use super::super::format_notes_for_summarization;

    #[test]
    fn format_notes_single_step() {
        let notes = vec![(
            "Security Scanner (task_force)".to_string(),
            "## Direction\n- Scanning repos for vulnerabilities".to_string(),
        )];
        let result = format_notes_for_summarization(&notes);
        assert_eq!(
            result,
            "[Security Scanner (task_force)]\n## Direction\n- Scanning repos for vulnerabilities"
        );
    }

    #[test]
    fn format_notes_multiple_steps() {
        let notes = vec![
            ("Alpha (task_force)".to_string(), "Alpha notes".to_string()),
            ("Beta (documenter)".to_string(), "Beta notes".to_string()),
        ];
        let result = format_notes_for_summarization(&notes);
        assert_eq!(
            result,
            "[Alpha (task_force)]\nAlpha notes\n\n[Beta (documenter)]\nBeta notes"
        );
    }

    #[test]
    fn format_notes_empty() {
        let notes: Vec<(String, String)> = vec![];
        let result = format_notes_for_summarization(&notes);
        assert!(result.is_empty());
    }
}
