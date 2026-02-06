#[cfg(test)]
mod tests {
    use crate::server::api::system_config::*;

    #[test]
    fn create_system_config_request_deserializes() {
        let json = r#"{
            "config_type": "execution",
            "config_key": "max_concurrent_agents",
            "config_value": 10
        }"#;
        let req: CreateSystemConfigRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.config_type, "execution");
        assert_eq!(req.config_key, "max_concurrent_agents");
        assert_eq!(req.config_value, serde_json::json!(10));
        assert!(req.description.is_none());
    }

    #[test]
    fn create_system_config_request_with_description() {
        let json = r#"{
            "config_type": "safety",
            "config_key": "unsafe_operations_enabled",
            "config_value": false,
            "description": "Toggle unsafe tool operations"
        }"#;
        let req: CreateSystemConfigRequest = serde_json::from_str(json).unwrap();
        assert_eq!(
            req.description.as_deref(),
            Some("Toggle unsafe tool operations")
        );
    }

    #[test]
    fn system_config_response_serializes() {
        let resp = SystemConfigResponse {
            id: uuid::Uuid::nil(),
            config_type: "execution".to_string(),
            config_key: "timeout_ms".to_string(),
            config_value: serde_json::json!(30000),
            description: None,
            updated_at: chrono::Utc::now(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["config_key"], "timeout_ms");
        assert_eq!(json["config_value"], 30000);
    }

    #[test]
    fn system_config_query_deserializes_empty() {
        let query: SystemConfigQuery = serde_json::from_str("{}").unwrap();
        assert!(query.config_type.is_none());
    }

    #[test]
    fn system_config_query_with_filter() {
        let query: SystemConfigQuery =
            serde_json::from_str(r#"{"config_type": "safety"}"#).unwrap();
        assert_eq!(query.config_type.as_deref(), Some("safety"));
    }
}
