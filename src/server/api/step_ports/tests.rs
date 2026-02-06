#[cfg(test)]
mod tests {
    use crate::server::api::step_ports::*;

    #[test]
    fn create_step_input_request_deserializes() {
        let json = r#"{"port_name": "context", "required": true}"#;
        let req: CreateStepInputRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.port_name, "context");
        assert!(req.required);
        assert_eq!(req.port_type, "string"); // default
        assert!(req.default_value.is_none());
    }

    #[test]
    fn create_step_input_request_with_all_fields() {
        let json = r#"{
            "port_name": "data",
            "port_type": "json",
            "required": false,
            "default_value": {"key": "val"},
            "description": "Input data port",
            "json_schema": {"type": "object"}
        }"#;
        let req: CreateStepInputRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.port_name, "data");
        assert_eq!(req.port_type, "json");
        assert!(!req.required);
        assert!(req.default_value.is_some());
        assert_eq!(req.description.as_deref(), Some("Input data port"));
    }

    #[test]
    fn create_step_output_request_deserializes() {
        let json = r#"{"port_name": "result", "json_path": "output.summary"}"#;
        let req: CreateStepOutputRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.port_name, "result");
        assert_eq!(req.json_path, "output.summary");
        assert_eq!(req.port_type, "string"); // default
    }

    #[test]
    fn step_input_response_serializes() {
        let resp = StepInputResponse {
            id: uuid::Uuid::nil(),
            port_name: "test".to_string(),
            port_type: "string".to_string(),
            required: true,
            default_value: None,
            description: None,
            json_schema: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["port_name"], "test");
        assert_eq!(json["required"], true);
    }

    #[test]
    fn step_output_response_serializes() {
        let resp = StepOutputResponse {
            id: uuid::Uuid::nil(),
            port_name: "analysis".to_string(),
            port_type: "json".to_string(),
            json_path: "results.data".to_string(),
            description: Some("Analysis output".to_string()),
            json_schema: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["port_name"], "analysis");
        assert_eq!(json["json_path"], "results.data");
        assert_eq!(json["description"], "Analysis output");
    }
}
