#[cfg(test)]
mod tests {
    use crate::server::hub::question_extraction::parse_extraction_output;

    #[test]
    fn parse_valid_with_question() {
        let input = r#"{"status": "Ready for web scraping", "question": "Which competitors?"}"#;
        let (status, question) = parse_extraction_output(input).unwrap();
        assert_eq!(status, "Ready for web scraping");
        assert_eq!(question.as_deref(), Some("Which competitors?"));
    }

    #[test]
    fn parse_valid_without_question() {
        let input = r#"{"status": "Configured for Acme + Widget", "question": null}"#;
        let (status, question) = parse_extraction_output(input).unwrap();
        assert_eq!(status, "Configured for Acme + Widget");
        assert!(question.is_none());
    }

    #[test]
    fn parse_from_code_fence() {
        let input =
            "```json\n{\"status\": \"Pipeline ready\", \"question\": \"What format?\"}\n```";
        let (status, question) = parse_extraction_output(input).unwrap();
        assert_eq!(status, "Pipeline ready");
        assert_eq!(question.as_deref(), Some("What format?"));
    }

    #[test]
    fn parse_invalid_json_returns_error() {
        let input = "This is not JSON at all";
        assert!(parse_extraction_output(input).is_err());
    }

    #[test]
    fn parse_missing_status_returns_error() {
        let input = r#"{"question": "What format?"}"#;
        assert!(parse_extraction_output(input).is_err());
    }

    #[test]
    fn parse_with_surrounding_whitespace() {
        let input = "  \n{\"status\": \"All set\", \"question\": null}\n  ";
        let (status, question) = parse_extraction_output(input).unwrap();
        assert_eq!(status, "All set");
        assert!(question.is_none());
    }
}
