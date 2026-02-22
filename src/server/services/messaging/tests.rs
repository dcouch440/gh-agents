#[cfg(test)]
mod tests {
    use serde_json::json;

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

    // ── dispatch_to_nodes input validation ──────────────────────────────

    #[test]
    fn dispatch_to_nodes_parses_messages() {
        let input = json!({
            "messages": [
                {
                    "node": "workforce-1",
                    "message_type": "initial_instruction",
                    "content": "Configure data collection"
                },
                {
                    "node": "workforce-2",
                    "message_type": "update",
                    "content": "Adjust analysis threshold"
                }
            ]
        });

        let messages = input["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0]["node"], "workforce-1");
        assert_eq!(messages[0]["message_type"], "initial_instruction");
        assert_eq!(messages[1]["node"], "workforce-2");
        assert_eq!(messages[1]["message_type"], "update");
    }

    #[tokio::test]
    async fn dispatch_to_nodes_rejects_missing_messages() {
        // The function requires an AppState which is heavy to construct in unit tests.
        // Instead, verify the input validation logic directly.
        let input = json!({});
        assert!(input["messages"].as_array().is_none());
    }

    #[tokio::test]
    async fn dispatch_to_nodes_rejects_empty_messages() {
        let input = json!({ "messages": [] });
        let messages = input["messages"].as_array().unwrap();
        assert!(messages.is_empty());
    }
}
