#[cfg(test)]
mod tests {
    use crate::server::api::protocols::*;

    #[test]
    fn create_protocol_request_deserializes_required_fields() {
        let json = r#"{"name": "Frontend/Backend Decomp", "protocol_type": "decomp"}"#;
        let req: CreateProtocolRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "Frontend/Backend Decomp");
        assert_eq!(req.protocol_type, "decomp");
        assert!(req.description.is_none());
        assert!(req.config.is_none());
        assert!(req.agent_id.is_none());
        assert!(req.output_schema_id.is_none());
        assert!(req.prompt_template_id.is_none());
    }

    #[test]
    fn create_protocol_request_deserializes_all_fields() {
        let json = r#"{
            "name": "Code Review Gate",
            "description": "Quality review before merge",
            "protocol_type": "review",
            "config": {"decisions": ["approve", "reject", "revise"]},
            "agent_id": "00000000-0000-0000-0000-000000000001",
            "output_schema_id": "00000000-0000-0000-0000-000000000002",
            "prompt_template_id": "00000000-0000-0000-0000-000000000003"
        }"#;
        let req: CreateProtocolRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "Code Review Gate");
        assert_eq!(
            req.description.as_deref(),
            Some("Quality review before merge")
        );
        assert_eq!(req.protocol_type, "review");
        assert!(req.config.is_some());
        assert!(req.agent_id.is_some());
        assert!(req.output_schema_id.is_some());
        assert!(req.prompt_template_id.is_some());
    }

    #[test]
    fn update_protocol_request_partial() {
        let json = r#"{"name": "Updated Name"}"#;
        let req: UpdateProtocolRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name.as_deref(), Some("Updated Name"));
        assert!(req.description.is_none());
        assert!(req.config.is_none());
        assert!(req.agent_id.is_none());
        assert!(req.output_schema_id.is_none());
        assert!(req.prompt_template_id.is_none());
    }

    #[test]
    fn update_protocol_request_all_fields() {
        let json = r#"{
            "name": "New Name",
            "description": "New desc",
            "config": {"key": "value"},
            "agent_id": "00000000-0000-0000-0000-000000000001",
            "output_schema_id": "00000000-0000-0000-0000-000000000002",
            "prompt_template_id": "00000000-0000-0000-0000-000000000003"
        }"#;
        let req: UpdateProtocolRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name.as_deref(), Some("New Name"));
        assert_eq!(req.description.as_deref(), Some("New desc"));
        assert!(req.config.is_some());
        assert!(req.agent_id.is_some());
        assert!(req.output_schema_id.is_some());
        assert!(req.prompt_template_id.is_some());
    }

    #[test]
    fn create_port_request_deserializes_required_fields() {
        let json = r#"{
            "port_name": "frontend",
            "agent_id": "00000000-0000-0000-0000-000000000001"
        }"#;
        let req: CreatePortRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.port_name, "frontend");
        assert!(req.description.is_none());
        assert!(req.display_order.is_none());
    }

    #[test]
    fn create_port_request_deserializes_all_fields() {
        let json = r#"{
            "port_name": "backend",
            "description": "Handles API work",
            "agent_id": "00000000-0000-0000-0000-000000000002",
            "display_order": 1
        }"#;
        let req: CreatePortRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.port_name, "backend");
        assert_eq!(req.description.as_deref(), Some("Handles API work"));
        assert_eq!(req.display_order, Some(1));
    }

    #[test]
    fn update_port_request_partial() {
        let json = r#"{"display_order": 3}"#;
        let req: UpdatePortRequest = serde_json::from_str(json).unwrap();
        assert!(req.port_name.is_none());
        assert!(req.description.is_none());
        assert!(req.agent_id.is_none());
        assert_eq!(req.display_order, Some(3));
    }

    #[test]
    fn protocol_response_serializes_with_associations() {
        let resp = ProtocolResponse {
            id: uuid::Uuid::nil(),
            name: "Test Protocol".to_string(),
            description: "A test".to_string(),
            protocol_type: "decomp".to_string(),
            config: serde_json::json!({}),
            version: 1,
            ports: vec![ProtocolPortResponse {
                id: uuid::Uuid::nil(),
                port_name: "frontend".to_string(),
                description: "UI work".to_string(),
                agent_id: uuid::Uuid::nil(),
                display_order: 0,
            }],
            agent: Some(ProtocolAgentResponse {
                id: uuid::Uuid::nil(),
                name: "Test Agent".to_string(),
                system_prompt: "You are a test agent".to_string(),
                model_provider: "anthropic".to_string(),
                model_id: "claude-sonnet-4-20250514".to_string(),
            }),
            output_schema: Some(ProtocolSchemaResponse {
                id: uuid::Uuid::nil(),
                name: "Test Schema".to_string(),
                schema: serde_json::json!({"type": "object"}),
            }),
            prompt_template: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["name"], "Test Protocol");
        assert_eq!(json["protocol_type"], "decomp");
        assert_eq!(json["version"], 1);
        assert_eq!(json["ports"][0]["port_name"], "frontend");
        assert!(json["agent"].is_object());
        assert_eq!(json["agent"]["name"], "Test Agent");
        assert!(json["output_schema"].is_object());
        assert_eq!(json["output_schema"]["name"], "Test Schema");
        assert!(json["prompt_template"].is_null());
    }

    #[test]
    fn protocol_response_serializes_without_associations() {
        let resp = ProtocolResponse {
            id: uuid::Uuid::nil(),
            name: "Minimal".to_string(),
            description: String::new(),
            protocol_type: "default".to_string(),
            config: serde_json::json!({}),
            version: 1,
            ports: vec![],
            agent: None,
            output_schema: None,
            prompt_template: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert!(json["agent"].is_null());
        assert!(json["output_schema"].is_null());
        assert!(json["prompt_template"].is_null());
    }

    #[test]
    fn protocol_types_response_serializes() {
        let resp = ProtocolTypesResponse {
            types: vec![
                ProtocolTypeInfo {
                    name: "decomp".to_string(),
                    description: "Fan-out decomposition".to_string(),
                },
                ProtocolTypeInfo {
                    name: "transform".to_string(),
                    description: "1:1 transform".to_string(),
                },
            ],
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["types"].as_array().unwrap().len(), 2);
        assert_eq!(json["types"][0]["name"], "decomp");
    }

    #[test]
    fn apply_response_serializes() {
        let resp = ApplyResponse {
            output_schema_id: uuid::Uuid::nil(),
            created_steps: vec![CreatedStepResponse {
                port_name: "frontend".to_string(),
                step_id: uuid::Uuid::nil(),
                agent_id: Some(uuid::Uuid::nil()),
            }],
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert!(json["output_schema_id"].is_string());
        assert_eq!(json["created_steps"][0]["port_name"], "frontend");
    }

    #[test]
    fn protocol_port_response_serializes() {
        let resp = ProtocolPortResponse {
            id: uuid::Uuid::nil(),
            port_name: "backend".to_string(),
            description: "Server-side work".to_string(),
            agent_id: uuid::Uuid::nil(),
            display_order: 2,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["port_name"], "backend");
        assert_eq!(json["display_order"], 2);
    }

    // =========================================================================
    // ApplyProtocolRequest Tests
    // =========================================================================

    #[test]
    fn apply_request_empty_body_deserializes() {
        let json = r#"{}"#;
        let req: ApplyProtocolRequest = serde_json::from_str(json).unwrap();
        assert!(req.decision_routing.is_none());
    }

    #[test]
    fn apply_request_with_decision_routing() {
        let json = r#"{
            "decision_routing": {
                "approve": "00000000-0000-0000-0000-000000000001",
                "reject": "00000000-0000-0000-0000-000000000002"
            }
        }"#;
        let req: ApplyProtocolRequest = serde_json::from_str(json).unwrap();
        let routing = req.decision_routing.unwrap();
        assert_eq!(routing.len(), 2);
        assert!(routing.contains_key("approve"));
        assert!(routing.contains_key("reject"));
    }

    #[test]
    fn apply_request_null_decision_routing() {
        let json = r#"{"decision_routing": null}"#;
        let req: ApplyProtocolRequest = serde_json::from_str(json).unwrap();
        assert!(req.decision_routing.is_none());
    }

    // =========================================================================
    // New Association Response Types
    // =========================================================================

    #[test]
    fn protocol_agent_response_serializes() {
        let resp = ProtocolAgentResponse {
            id: uuid::Uuid::nil(),
            name: "Decomp Agent".to_string(),
            system_prompt: "You decompose tasks".to_string(),
            model_provider: "anthropic".to_string(),
            model_id: "claude-sonnet-4-20250514".to_string(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["name"], "Decomp Agent");
        assert_eq!(json["model_provider"], "anthropic");
    }

    #[test]
    fn protocol_schema_response_serializes() {
        let resp = ProtocolSchemaResponse {
            id: uuid::Uuid::nil(),
            name: "Decomp Output".to_string(),
            schema: serde_json::json!({"type": "object", "required": ["subtasks"]}),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["name"], "Decomp Output");
        assert_eq!(json["schema"]["type"], "object");
    }

    #[test]
    fn protocol_template_response_serializes() {
        let resp = ProtocolTemplateResponse {
            id: uuid::Uuid::nil(),
            name: "Decomp Prompt".to_string(),
            content: "Decompose the input into subtasks.".to_string(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["name"], "Decomp Prompt");
        assert!(!json["content"].as_str().unwrap().is_empty());
    }
}
