//! Route protocol expander — conditional switch pattern (1 → 1 of N).
//!
//! Examines input and routes to exactly one of N downstream agents.

use std::collections::HashSet;

use crate::server::hub::protocols::error::ProtocolError;
use crate::server::hub::protocols::expander::ProtocolExpander;
use crate::server::hub::protocols::types::{
    EdgeDefinition, InputPortDefinition, OutputPortDefinition, ProtocolConfig, ProtocolExpansion,
    StepDefinition,
};
use crate::server::hub::protocols::{prompt_gen, schema_gen};

/// Expander for the "route" (conditional switch) protocol.
pub struct RouteExpander;

impl ProtocolExpander for RouteExpander {
    fn protocol_type(&self) -> &str {
        "route"
    }

    fn description(&self) -> &str {
        "Examine input and route to exactly one of N agents (conditional switch)"
    }

    fn validate(&self, config: &ProtocolConfig) -> Result<(), ProtocolError> {
        if config.ports.is_empty() {
            return Err(ProtocolError::NoPorts);
        }

        let mut seen = HashSet::new();
        for port in &config.ports {
            if !seen.insert(&port.port_name) {
                return Err(ProtocolError::DuplicatePortName(port.port_name.clone()));
            }
        }

        Ok(())
    }

    fn generate_schema(&self, config: &ProtocolConfig) -> Result<serde_json::Value, ProtocolError> {
        Ok(schema_gen::route_schema(&config.ports))
    }

    fn generate_prompt_injection(&self, config: &ProtocolConfig) -> Result<String, ProtocolError> {
        Ok(prompt_gen::route_prompt(&config.ports))
    }

    fn expand(&self, config: &ProtocolConfig) -> Result<ProtocolExpansion, ProtocolError> {
        self.validate(config)?;

        let output_schema = self.generate_schema(config)?;
        let prompt_injection = self.generate_prompt_injection(config)?;

        // Create one downstream step per port (same as decomp)
        let steps: Vec<StepDefinition> = config
            .ports
            .iter()
            .map(|port| StepDefinition {
                port_name: port.port_name.clone(),
                agent_id: port.agent_id,
                execution_mode: "single".to_string(),
                prompt_template: None,
                output_schema: None,
                routing_mode: None,
                routing_field: None,
                for_each_label_field: None,
                for_each_ref: None,
                routing_rules: vec![],
            })
            .collect();

        // Create conditional edges — one per port, activated by port match.
        // Only ONE edge fires at runtime (unlike decomp where all fire).
        let edges: Vec<EdgeDefinition> = config
            .ports
            .iter()
            .map(|port| EdgeDefinition {
                from_output_port: "routed_content".to_string(),
                to_input_port: "task_input".to_string(),
                target_port_name: port.port_name.clone(),
                condition_type: Some("port_match".to_string()),
                condition_value: Some(serde_json::json!({
                    "field": "port",
                    "value": port.port_name
                })),
            })
            .collect();

        // Single output port for the routed content
        let output_ports = vec![OutputPortDefinition {
            port_name: "routed_content".to_string(),
            port_type: "object".to_string(),
            json_path: "content".to_string(),
            description: Some("The content routed to the selected agent".to_string()),
        }];

        // Input port on each downstream step
        let input_ports: Vec<InputPortDefinition> = config
            .ports
            .iter()
            .map(|port| InputPortDefinition {
                target_port_name: port.port_name.clone(),
                port_name: "task_input".to_string(),
                port_type: "object".to_string(),
                required: true,
                description: Some(format!("Routed input for {}", port.port_name)),
            })
            .collect();

        Ok(ProtocolExpansion {
            output_schema,
            prompt_injection,
            steps,
            edges,
            output_ports,
            input_ports,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::hub::protocols::types::PortConfig;
    use uuid::Uuid;

    fn two_port_config() -> ProtocolConfig {
        ProtocolConfig {
            protocol_type: "route".to_string(),
            config: serde_json::json!({}),
            ports: vec![
                PortConfig {
                    port_name: "urgent".to_string(),
                    description: "Urgent handler".to_string(),
                    agent_id: Uuid::new_v4(),
                    agent_name: "Urgent Agent".to_string(),
                    agent_tools: vec![],
                    display_order: 0,
                    content_schema: None,
                },
                PortConfig {
                    port_name: "normal".to_string(),
                    description: "Normal handler".to_string(),
                    agent_id: Uuid::new_v4(),
                    agent_name: "Normal Agent".to_string(),
                    agent_tools: vec![],
                    display_order: 1,
                    content_schema: None,
                },
            ],
        }
    }

    #[test]
    fn expand_creates_conditional_edges() {
        let config = two_port_config();
        let expansion = RouteExpander.expand(&config).unwrap();

        // Should have 2 steps (one per port)
        assert_eq!(expansion.steps.len(), 2);

        // Should have 2 conditional edges
        assert_eq!(expansion.edges.len(), 2);
        assert_eq!(
            expansion.edges[0].condition_type.as_deref(),
            Some("port_match")
        );
        assert_eq!(expansion.edges[0].target_port_name, "urgent");
        assert_eq!(expansion.edges[1].target_port_name, "normal");

        // Output schema should be a single object (not array like decomp)
        assert_eq!(expansion.output_schema["type"], "object");
    }

    #[test]
    fn validates_no_ports() {
        let config = ProtocolConfig {
            protocol_type: "route".to_string(),
            config: serde_json::json!({}),
            ports: vec![],
        };
        assert!(matches!(
            RouteExpander.validate(&config),
            Err(ProtocolError::NoPorts)
        ));
    }
}
