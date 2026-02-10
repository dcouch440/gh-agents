#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::server::hub::dag::documenter::build_documents_output;
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
}
