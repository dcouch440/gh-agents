#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use uuid::Uuid;

    use crate::server::hub::protocols::types::ProtocolConfig;
    use crate::server::hub::protocols::ProtocolEngine;

    #[test]
    fn engine_has_no_registered_types() {
        let engine = ProtocolEngine::new();
        let types = engine.list_types();
        assert!(types.is_empty());
    }

    #[test]
    fn engine_rejects_unknown_type() {
        let engine = ProtocolEngine::new();
        let config = ProtocolConfig {
            protocol_type: "nonexistent".to_string(),
            config: serde_json::json!({}),
            ports: vec![],
        };
        let result = engine.expand(&config);
        assert!(result.is_err());
    }

    #[test]
    fn engine_build_config_maps_agent_names() {
        let engine = ProtocolEngine::new();
        let agent_id_1 = Uuid::new_v4();
        let agent_id_2 = Uuid::new_v4();

        let ports = vec![
            crate::db::ProtocolPortRow {
                id: Uuid::new_v4(),
                protocol_id: Uuid::new_v4(),
                port_name: "frontend".to_string(),
                description: "FE work".to_string(),
                agent_id: agent_id_1,
                display_order: 0,
            },
            crate::db::ProtocolPortRow {
                id: Uuid::new_v4(),
                protocol_id: Uuid::new_v4(),
                port_name: "backend".to_string(),
                description: "BE work".to_string(),
                agent_id: agent_id_2,
                display_order: 1,
            },
        ];

        let mut agent_names = HashMap::new();
        agent_names.insert(agent_id_1, "FE Agent".to_string());
        agent_names.insert(agent_id_2, "BE Agent".to_string());

        let agent_tools = HashMap::new();
        let agent_schemas = HashMap::new();
        let config = engine.build_config(
            "test_type",
            serde_json::json!({}),
            &ports,
            &agent_names,
            &agent_tools,
            &agent_schemas,
        );

        assert_eq!(config.ports[0].agent_name, "FE Agent");
        assert_eq!(config.ports[1].agent_name, "BE Agent");
    }

    #[test]
    fn engine_build_config_passes_content_schemas() {
        let engine = ProtocolEngine::new();
        let agent_id_1 = Uuid::new_v4();
        let agent_id_2 = Uuid::new_v4();

        let ports = vec![
            crate::db::ProtocolPortRow {
                id: Uuid::new_v4(),
                protocol_id: Uuid::new_v4(),
                port_name: "frontend".to_string(),
                description: "FE work".to_string(),
                agent_id: agent_id_1,
                display_order: 0,
            },
            crate::db::ProtocolPortRow {
                id: Uuid::new_v4(),
                protocol_id: Uuid::new_v4(),
                port_name: "backend".to_string(),
                description: "BE work".to_string(),
                agent_id: agent_id_2,
                display_order: 1,
            },
        ];

        let mut agent_names = HashMap::new();
        agent_names.insert(agent_id_1, "FE Agent".to_string());
        agent_names.insert(agent_id_2, "BE Agent".to_string());

        let agent_tools = HashMap::new();

        let fe_schema = serde_json::json!({
            "type": "object",
            "properties": {"component": {"type": "string"}}
        });
        let mut agent_schemas = HashMap::new();
        agent_schemas.insert(agent_id_1, fe_schema.clone());
        // agent_id_2 intentionally has no schema

        let config = engine.build_config(
            "test_type",
            serde_json::json!({}),
            &ports,
            &agent_names,
            &agent_tools,
            &agent_schemas,
        );

        assert_eq!(config.ports[0].content_schema, Some(fe_schema));
        assert_eq!(config.ports[1].content_schema, None);
    }
}
