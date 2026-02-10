#[cfg(test)]
mod tests {
    use crate::server::hub::protocols::expander::ProtocolExpander;
    use crate::server::hub::protocols::expanders::documenter::DocumenterExpander;
    use crate::server::hub::protocols::types::ProtocolConfig;
    use serde_json::json;

    fn make_config(doc_defs: serde_json::Value) -> ProtocolConfig {
        ProtocolConfig {
            protocol_type: "documenter".to_string(),
            config: json!({ "document_defs": doc_defs }),
            ports: vec![],
        }
    }

    fn make_config_with_capabilities(
        doc_defs: serde_json::Value,
        capabilities: Vec<&str>,
    ) -> ProtocolConfig {
        ProtocolConfig {
            protocol_type: "documenter".to_string(),
            config: json!({
                "document_defs": doc_defs,
                "available_capabilities": capabilities,
            }),
            ports: vec![],
        }
    }

    fn sample_defs() -> serde_json::Value {
        json!([
            {"name": "API Reference", "description": "REST API docs", "target_length": 5000},
            {"name": "Architecture Guide", "description": "System overview", "target_length": 3000},
        ])
    }

    // =========================================================================
    // Validation
    // =========================================================================

    #[test]
    fn validate_rejects_missing_doc_defs() {
        let config = ProtocolConfig {
            protocol_type: "documenter".to_string(),
            config: json!({}),
            ports: vec![],
        };
        let result = DocumenterExpander.validate(&config);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("document_defs"));
    }

    #[test]
    fn validate_rejects_empty_doc_defs() {
        let config = make_config(json!([]));
        let result = DocumenterExpander.validate(&config);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("at least one"));
    }

    #[test]
    fn validate_rejects_def_without_name() {
        let config = make_config(json!([
            {"description": "Some doc", "target_length": 1000}
        ]));
        let result = DocumenterExpander.validate(&config);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("name"));
    }

    #[test]
    fn validate_rejects_def_with_empty_name() {
        let config = make_config(json!([
            {"name": "", "description": "Some doc", "target_length": 1000}
        ]));
        let result = DocumenterExpander.validate(&config);
        assert!(result.is_err());
    }

    #[test]
    fn validate_rejects_negative_target_length() {
        let config = make_config(json!([
            {"name": "Doc", "target_length": -100}
        ]));
        let result = DocumenterExpander.validate(&config);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("target_length"));
    }

    #[test]
    fn validate_accepts_valid_config() {
        let config = make_config(sample_defs());
        assert!(DocumenterExpander.validate(&config).is_ok());
    }

    // =========================================================================
    // Expansion
    // =========================================================================

    #[test]
    fn expand_returns_empty_steps_and_edges() {
        let config = make_config(sample_defs());
        let expansion = DocumenterExpander.expand(&config).unwrap();

        assert!(expansion.steps.is_empty());
        assert!(expansion.edges.is_empty());
        assert!(expansion.output_ports.is_empty());
        assert!(expansion.input_ports.is_empty());
    }

    #[test]
    fn expand_generates_schema_with_doc_names() {
        let config = make_config(sample_defs());
        let expansion = DocumenterExpander.expand(&config).unwrap();

        let name_enum = &expansion.output_schema["properties"]["document_plans"]["items"]
            ["properties"]["document_name"]["enum"];
        assert_eq!(name_enum[0], "API Reference");
        assert_eq!(name_enum[1], "Architecture Guide");
    }

    #[test]
    fn expand_generates_prompt_with_capabilities() {
        let config =
            make_config_with_capabilities(sample_defs(), vec!["web_search", "code_analysis"]);
        let expansion = DocumenterExpander.expand(&config).unwrap();

        assert!(expansion.prompt_injection.contains("web_search"));
        assert!(expansion.prompt_injection.contains("code_analysis"));
        assert!(expansion
            .prompt_injection
            .contains("Available Research Capabilities"));
    }

    #[test]
    fn expand_generates_prompt_without_capabilities() {
        let config = make_config(sample_defs());
        let expansion = DocumenterExpander.expand(&config).unwrap();

        assert!(!expansion
            .prompt_injection
            .contains("Available Research Capabilities"));
        assert!(expansion.prompt_injection.contains("Document Strategist"));
        assert!(expansion.prompt_injection.contains("API Reference"));
    }

    // =========================================================================
    // Trait metadata
    // =========================================================================

    #[test]
    fn protocol_type_is_documenter() {
        assert_eq!(DocumenterExpander.protocol_type(), "documenter");
    }

    #[test]
    fn description_is_non_empty() {
        assert!(!DocumenterExpander.description().is_empty());
    }
}
