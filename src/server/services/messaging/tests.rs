#[cfg(test)]
mod tests {
    use crate::server::services::messaging::wrap_agent_xml;

    #[test]
    fn wrap_xml_with_ref() {
        let result = wrap_agent_xml("Manager", "initial_instruction", Some("c8f2"), "Hello node");
        assert!(result.contains(r#"from="Manager""#));
        assert!(result.contains(r#"type="initial_instruction""#));
        assert!(result.contains(r#"ref="c8f2""#));
        assert!(result.contains("Hello node"));
        assert!(result.starts_with("<agent_message"));
        assert!(result.ends_with("</agent_message>"));
    }

    #[test]
    fn wrap_xml_without_ref() {
        let result = wrap_agent_xml("Collector", "coordination", None, "Status update");
        assert!(result.contains(r#"from="Collector""#));
        assert!(result.contains(r#"type="coordination""#));
        assert!(!result.contains("ref="));
        assert!(result.contains("Status update"));
    }

    #[test]
    fn wrap_xml_preserves_multiline_content() {
        let content = "Line one\nLine two\nLine three";
        let result = wrap_agent_xml("Manager", "update", None, content);
        assert!(result.contains("Line one\nLine two\nLine three"));
    }
}
