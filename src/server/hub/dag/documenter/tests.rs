#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::server::hub::dag::documenter::{build_context_block, build_documents_output, ContextDocument};
    use crate::server::hub::dag::utils::StepOutput;
    use crate::server::ws::events::WorkflowEventKind;

    #[test]
    fn documenter_output_json_shape() {
        let statuses = vec![
            json!({"name": "API Reference", "status": "complete"}),
            json!({"name": "Architecture Guide", "status": "failed", "error": "research failed"}),
        ];

        let output = build_documents_output(statuses);
        let docs = output["documents"].as_array().unwrap();
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0]["name"], "API Reference");
        assert_eq!(docs[0]["status"], "complete");
        assert_eq!(docs[1]["status"], "failed");
        assert_eq!(docs[1]["error"], "research failed");
    }

    #[test]
    fn documenter_result_constructs_step_output() {
        let structured =
            build_documents_output(vec![json!({"name": "README", "status": "complete"})]);
        let raw = serde_json::to_string_pretty(&structured).unwrap();

        let output = StepOutput {
            variable_name: "documenter_output".into(),
            structured_output: Some(structured.clone()),
            raw_output: raw.clone(),
        };

        assert_eq!(output.variable_name, "documenter_output");
        assert!(output.structured_output.is_some());
        assert!(output.raw_output.contains("README"));
    }

    #[test]
    fn ws_documenter_phase_progress_serializes() {
        let event = WorkflowEventKind::DocumenterPhaseProgress {
            step_id: uuid::Uuid::nil(),
            phase: "research".into(),
            completed: 2,
            total: 3,
            document_name: Some("API Reference".into()),
        };

        let json = serde_json::to_value(&event).unwrap();
        // serde(rename_all = "snake_case") wraps in the variant name
        let inner = &json["documenter_phase_progress"];
        assert_eq!(inner["phase"], "research");
        assert_eq!(inner["completed"], 2);
        assert_eq!(inner["total"], 3);
        assert_eq!(inner["document_name"], "API Reference");
    }

    #[test]
    fn ws_documenter_phase_progress_omits_none_doc_name() {
        let event = WorkflowEventKind::DocumenterPhaseProgress {
            step_id: uuid::Uuid::nil(),
            phase: "strategy".into(),
            completed: 1,
            total: 1,
            document_name: None,
        };

        let json = serde_json::to_value(&event).unwrap();
        let inner = &json["documenter_phase_progress"];
        assert_eq!(inner["phase"], "strategy");
        assert!(inner.get("document_name").is_none());
    }

    // ── build_context_block tests ────────────────────────────────────────

    fn make_context_docs() -> Vec<ContextDocument> {
        vec![
            ContextDocument {
                short_id: "550e8400".into(),
                title: "API Spec".into(),
                content: "OpenAPI specification content".into(),
            },
            ContextDocument {
                short_id: "a1b2c3d4".into(),
                title: "Style Guide".into(),
                content: "Style guide content".into(),
            },
            ContextDocument {
                short_id: "deadbeef".into(),
                title: "Architecture".into(),
                content: "Architecture overview".into(),
            },
        ]
    }

    #[test]
    fn build_context_block_empty_docs_returns_empty() {
        let result = build_context_block(&[], &[]);
        assert!(result.is_empty());
    }

    #[test]
    fn build_context_block_empty_docs_with_ids_returns_empty() {
        let result = build_context_block(&["550e8400".into()], &[]);
        assert!(result.is_empty());
    }

    #[test]
    fn build_context_block_empty_ids_includes_all_docs() {
        let docs = make_context_docs();
        let result = build_context_block(&[], &docs);
        assert!(result.contains("<context>"));
        assert!(result.contains("</context>"));
        assert!(result.contains("<document_550e8400"));
        assert!(result.contains("<document_a1b2c3d4"));
        assert!(result.contains("<document_deadbeef"));
        assert!(result.contains("API Spec"));
        assert!(result.contains("Style Guide"));
        assert!(result.contains("Architecture"));
    }

    #[test]
    fn build_context_block_filters_by_assigned_ids() {
        let docs = make_context_docs();
        let ids = vec!["550e8400".into(), "deadbeef".into()];
        let result = build_context_block(&ids, &docs);
        assert!(result.contains("<document_550e8400"));
        assert!(result.contains("<document_deadbeef"));
        assert!(!result.contains("<document_a1b2c3d4"));
        assert!(result.contains("API Spec"));
        assert!(!result.contains("Style Guide"));
        assert!(result.contains("Architecture"));
    }

    #[test]
    fn build_context_block_no_matching_ids_returns_empty() {
        let docs = make_context_docs();
        let ids = vec!["00000000".into()];
        let result = build_context_block(&ids, &docs);
        assert!(result.is_empty());
    }

    #[test]
    fn build_context_block_single_doc_format() {
        let docs = vec![ContextDocument {
            short_id: "abcd1234".into(),
            title: "Test Doc".into(),
            content: "Test content".into(),
        }];
        let result = build_context_block(&[], &docs);
        assert!(result.starts_with("<context>"));
        assert!(result.ends_with("</context>"));
        assert!(result.contains("<document_abcd1234 title=\"Test Doc\">"));
        assert!(result.contains("Test content"));
        assert!(result.contains("</document_abcd1234>"));
    }
}
