#[cfg(test)]
mod tests {
    use crate::server::api::routing_rules::*;

    #[test]
    fn create_routing_rule_request_deserializes() {
        let json =
            r#"{"label_value": "frontend", "agent_id": "00000000-0000-0000-0000-000000000001"}"#;
        let req: CreateRoutingRuleRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.label_value, "frontend");
        assert_eq!(req.display_order, 0); // default
        assert!(req.description.is_none());
    }

    #[test]
    fn create_routing_rule_request_with_all_fields() {
        let json = r#"{
            "label_value": "backend",
            "agent_id": "00000000-0000-0000-0000-000000000002",
            "description": "Backend specialist",
            "display_order": 1
        }"#;
        let req: CreateRoutingRuleRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.label_value, "backend");
        assert_eq!(req.display_order, 1);
        assert_eq!(req.description.as_deref(), Some("Backend specialist"));
    }

    #[test]
    fn update_routing_rule_request_partial() {
        let json = r#"{"display_order": 5}"#;
        let req: UpdateRoutingRuleRequest = serde_json::from_str(json).unwrap();
        assert!(req.agent_id.is_none());
        assert!(req.description.is_none());
        assert_eq!(req.display_order, Some(5));
    }

    #[test]
    fn routing_rule_response_serializes() {
        let resp = RoutingRuleResponse {
            id: uuid::Uuid::nil(),
            label_value: "frontend".to_string(),
            description: Some("UI work".to_string()),
            agent_id: uuid::Uuid::nil(),
            display_order: 0,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["label_value"], "frontend");
        assert_eq!(json["description"], "UI work");
    }
}
