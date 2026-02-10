#[cfg(test)]
mod tests {
    use crate::server::hub::protocols::compiler::ProtocolCompiler;
    use crate::server::hub::protocols::compilers::documenter::prompt::{
        documenter_prompt, format_context_documents_instruction,
    };
    use crate::server::hub::protocols::compilers::documenter::schema::documenter_schema;
    use crate::server::hub::protocols::compilers::documenter::DocumenterCompiler;
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

    fn make_doc_defs() -> Vec<serde_json::Value> {
        vec![
            json!({"name": "API Reference", "description": "REST API docs", "target_length": 5000}),
            json!({"name": "Architecture Guide", "description": "System overview", "target_length": 3000}),
        ]
    }

    // =========================================================================
    // Compiler: Validation
    // =========================================================================

    #[test]
    fn validate_rejects_missing_doc_defs() {
        let config = ProtocolConfig {
            protocol_type: "documenter".to_string(),
            config: json!({}),
            ports: vec![],
        };
        let result = DocumenterCompiler.validate(&config);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("document_defs"));
    }

    #[test]
    fn validate_rejects_empty_doc_defs() {
        let config = make_config(json!([]));
        let result = DocumenterCompiler.validate(&config);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("at least one"));
    }

    #[test]
    fn validate_rejects_def_without_name() {
        let config = make_config(json!([
            {"description": "Some doc", "target_length": 1000}
        ]));
        let result = DocumenterCompiler.validate(&config);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("name"));
    }

    #[test]
    fn validate_rejects_def_with_empty_name() {
        let config = make_config(json!([
            {"name": "", "description": "Some doc", "target_length": 1000}
        ]));
        let result = DocumenterCompiler.validate(&config);
        assert!(result.is_err());
    }

    #[test]
    fn validate_rejects_negative_target_length() {
        let config = make_config(json!([
            {"name": "Doc", "target_length": -100}
        ]));
        let result = DocumenterCompiler.validate(&config);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("target_length"));
    }

    #[test]
    fn validate_accepts_valid_config() {
        let config = make_config(sample_defs());
        assert!(DocumenterCompiler.validate(&config).is_ok());
    }

    // =========================================================================
    // Compiler: Compilation
    // =========================================================================

    #[test]
    fn compile_returns_empty_steps_and_edges() {
        let config = make_config(sample_defs());
        let expansion = DocumenterCompiler.compile(&config).unwrap();

        assert!(expansion.steps.is_empty());
        assert!(expansion.edges.is_empty());
        assert!(expansion.output_ports.is_empty());
        assert!(expansion.input_ports.is_empty());
    }

    #[test]
    fn compile_generates_schema_with_doc_names() {
        let config = make_config(sample_defs());
        let expansion = DocumenterCompiler.compile(&config).unwrap();

        let name_enum = &expansion.output_schema["properties"]["document_plans"]["items"]
            ["properties"]["document_name"]["enum"];
        assert_eq!(name_enum[0], "API Reference");
        assert_eq!(name_enum[1], "Architecture Guide");
    }

    #[test]
    fn compile_generates_prompt_with_capabilities() {
        let config =
            make_config_with_capabilities(sample_defs(), vec!["web_search", "code_analysis"]);
        let expansion = DocumenterCompiler.compile(&config).unwrap();

        assert!(expansion.prompt_injection.contains("web_search"));
        assert!(expansion.prompt_injection.contains("code_analysis"));
        assert!(expansion
            .prompt_injection
            .contains("Available Research Capabilities"));
    }

    #[test]
    fn compile_generates_prompt_without_capabilities() {
        let config = make_config(sample_defs());
        let expansion = DocumenterCompiler.compile(&config).unwrap();

        assert!(!expansion
            .prompt_injection
            .contains("Available Research Capabilities"));
        assert!(expansion.prompt_injection.contains("Document Strategist"));
        assert!(expansion.prompt_injection.contains("API Reference"));
    }

    // =========================================================================
    // Compiler: Trait metadata
    // =========================================================================

    #[test]
    fn protocol_type_is_documenter() {
        assert_eq!(DocumenterCompiler.protocol_type(), "documenter");
    }

    #[test]
    fn description_is_non_empty() {
        assert!(!DocumenterCompiler.description().is_empty());
    }

    // =========================================================================
    // Schema generation
    // =========================================================================

    #[test]
    fn documenter_schema_has_document_plans_array() {
        let defs = make_doc_defs();
        let schema = documenter_schema(&defs);

        assert_eq!(schema["type"], "object");
        assert!(schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!("document_plans")));
        assert_eq!(schema["properties"]["document_plans"]["type"], "array");
    }

    #[test]
    fn documenter_schema_has_document_name_enum() {
        let defs = make_doc_defs();
        let schema = documenter_schema(&defs);

        let name_enum =
            &schema["properties"]["document_plans"]["items"]["properties"]["document_name"]["enum"];
        assert_eq!(name_enum[0], "API Reference");
        assert_eq!(name_enum[1], "Architecture Guide");
    }

    #[test]
    fn documenter_schema_requires_all_plan_fields() {
        let defs = make_doc_defs();
        let schema = documenter_schema(&defs);

        let required = schema["properties"]["document_plans"]["items"]["required"]
            .as_array()
            .unwrap();
        assert!(required.contains(&json!("document_name")));
        assert!(required.contains(&json!("research_strategy")));
        assert!(required.contains(&json!("required_capabilities")));
        assert!(required.contains(&json!("writer_prompt")));
    }

    #[test]
    fn documenter_schema_empty_defs_omits_enum() {
        let schema = documenter_schema(&[]);

        let name_schema =
            &schema["properties"]["document_plans"]["items"]["properties"]["document_name"];
        assert_eq!(name_schema["type"], "string");
        assert!(name_schema["enum"].is_null());
    }

    #[test]
    fn documenter_schema_disallows_additional_properties() {
        let defs = make_doc_defs();
        let schema = documenter_schema(&defs);

        assert_eq!(schema["additionalProperties"], false);
        assert_eq!(
            schema["properties"]["document_plans"]["items"]["additionalProperties"],
            false
        );
    }

    #[test]
    fn documenter_schema_includes_optional_context_document_ids() {
        let defs = make_doc_defs();
        let schema = documenter_schema(&defs);

        let ctx_field =
            &schema["properties"]["document_plans"]["items"]["properties"]["context_document_ids"];
        assert_eq!(ctx_field["type"], "array");
        assert_eq!(ctx_field["items"]["type"], "string");

        // context_document_ids is NOT required
        let required = schema["properties"]["document_plans"]["items"]["required"]
            .as_array()
            .unwrap();
        assert!(!required.contains(&json!("context_document_ids")));
    }

    // =========================================================================
    // Prompt generation
    // =========================================================================

    #[test]
    fn documenter_prompt_includes_all_documents() {
        let defs = make_doc_defs();
        let prompt = documenter_prompt(&defs, &[], false);
        assert!(prompt.contains("\"API Reference\""));
        assert!(prompt.contains("\"Architecture Guide\""));
        assert!(prompt.contains("~5000 characters"));
        assert!(prompt.contains("~3000 characters"));
        assert!(prompt.contains("REST API docs"));
        assert!(prompt.contains("System overview"));
    }

    #[test]
    fn documenter_prompt_includes_strategist_role() {
        let defs = make_doc_defs();
        let prompt = documenter_prompt(&defs, &[], false);
        assert!(prompt.contains("Document Strategist"));
    }

    #[test]
    fn documenter_prompt_includes_capabilities() {
        let defs = make_doc_defs();
        let caps = vec!["web_search".to_string(), "code_analysis".to_string()];
        let prompt = documenter_prompt(&defs, &caps, false);
        assert!(prompt.contains("Available Research Capabilities:"));
        assert!(prompt.contains("- web_search"));
        assert!(prompt.contains("- code_analysis"));
    }

    #[test]
    fn documenter_prompt_omits_capabilities_when_empty() {
        let defs = make_doc_defs();
        let prompt = documenter_prompt(&defs, &[], false);
        assert!(!prompt.contains("Available Research Capabilities:"));
    }

    #[test]
    fn documenter_prompt_includes_response_format() {
        let defs = make_doc_defs();
        let prompt = documenter_prompt(&defs, &[], false);
        assert!(prompt.contains("document_name"));
        assert!(prompt.contains("research_strategy"));
        assert!(prompt.contains("required_capabilities"));
        assert!(prompt.contains("writer_prompt"));
        assert!(prompt.contains("document_plans"));
    }

    #[test]
    fn documenter_prompt_handles_empty_description() {
        let defs = vec![json!({"name": "Readme", "description": "", "target_length": 1000})];
        let prompt = documenter_prompt(&defs, &[], false);
        assert!(prompt.contains("\"Readme\""));
        assert!(prompt.contains("~1000 characters"));
        // Should not have an em dash for empty description
        assert!(!prompt.contains("\"Readme\" \u{2014}"));
    }

    #[test]
    fn documenter_prompt_includes_context_instruction_when_enabled() {
        let defs = make_doc_defs();
        let prompt = documenter_prompt(&defs, &[], true);
        assert!(prompt.contains("Context Documents:"));
        assert!(prompt.contains("<document_XXXXXXXX>"));
        assert!(prompt.contains("context_document_ids"));
    }

    #[test]
    fn documenter_prompt_omits_context_instruction_when_disabled() {
        let defs = make_doc_defs();
        let prompt = documenter_prompt(&defs, &[], false);
        assert!(!prompt.contains("Context Documents:"));
    }

    #[test]
    fn format_context_documents_instruction_empty_when_false() {
        assert_eq!(format_context_documents_instruction(false), "");
    }

    #[test]
    fn format_context_documents_instruction_present_when_true() {
        let block = format_context_documents_instruction(true);
        assert!(block.contains("Context Documents:"));
        assert!(block.contains("8-character IDs"));
    }

    // =========================================================================
    // Capabilities block helper
    // =========================================================================

    #[test]
    fn format_capabilities_block_empty_returns_empty() {
        use super::super::prompt::format_capabilities_block;
        assert_eq!(format_capabilities_block(&[]), "");
    }

    #[test]
    fn format_capabilities_block_with_caps() {
        use super::super::prompt::format_capabilities_block;
        let caps = vec!["search".to_string(), "code".to_string()];
        let block = format_capabilities_block(&caps);
        assert!(block.contains("Available Research Capabilities:"));
        assert!(block.contains("- search"));
        assert!(block.contains("- code"));
    }
}
