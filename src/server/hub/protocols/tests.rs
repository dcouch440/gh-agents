#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use uuid::Uuid;

    use crate::server::hub::protocols::types::PortConfig;
    use crate::server::hub::protocols::ProtocolEngine;

    fn make_decomp_config() -> super::super::types::ProtocolConfig {
        super::super::types::ProtocolConfig {
            protocol_type: "decomp".to_string(),
            config: serde_json::json!({}),
            ports: vec![
                PortConfig {
                    port_name: "frontend".to_string(),
                    description: "Handles UI/UX tasks".to_string(),
                    agent_id: Uuid::new_v4(),
                    agent_name: "Frontend Dev".to_string(),
                    agent_tools: vec![],
                    display_order: 0,
                    content_schema: None,
                },
                PortConfig {
                    port_name: "backend".to_string(),
                    description: "Handles API and server tasks".to_string(),
                    agent_id: Uuid::new_v4(),
                    agent_name: "Backend Dev".to_string(),
                    agent_tools: vec![],
                    display_order: 1,
                    content_schema: None,
                },
            ],
        }
    }

    #[test]
    fn engine_registers_all_builtins() {
        let engine = ProtocolEngine::new();
        let types = engine.list_types();
        let type_names: Vec<&str> = types.iter().map(|(t, _)| *t).collect();
        assert!(type_names.contains(&"decomp"));
        assert!(type_names.contains(&"transform"));
        assert!(type_names.contains(&"review"));
        assert!(type_names.contains(&"route"));
        assert!(type_names.contains(&"default"));
    }

    #[test]
    fn engine_expands_decomp() {
        let engine = ProtocolEngine::new();
        let config = make_decomp_config();
        let expansion = engine.expand(&config).unwrap();

        // Should create 1 dispatch step with label routing
        assert_eq!(expansion.steps.len(), 1);
        assert_eq!(expansion.steps[0].port_name, "dispatch");
        assert_eq!(expansion.steps[0].execution_mode, "for_each");
        assert_eq!(expansion.steps[0].routing_mode.as_deref(), Some("label"));
        assert_eq!(expansion.steps[0].routing_rules.len(), 2);

        // Should create 1 edge
        assert_eq!(expansion.edges.len(), 1);

        // Output schema should have port enum
        let port_enum = &expansion.output_schema["items"]["properties"]["port"]["enum"];
        assert_eq!(port_enum[0], "frontend");
        assert_eq!(port_enum[1], "backend");

        // Prompt injection should mention agents
        assert!(expansion.prompt_injection.contains("frontend"));
        assert!(expansion.prompt_injection.contains("backend"));
        assert!(expansion.prompt_injection.contains("Frontend Dev"));
    }

    #[test]
    fn engine_rejects_unknown_type() {
        let engine = ProtocolEngine::new();
        let config = super::super::types::ProtocolConfig {
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
            "decomp",
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
            "decomp",
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
