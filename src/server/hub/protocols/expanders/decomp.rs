//! Decomp protocol expander — fan-out pattern (1 → N).
//!
//! Decomposes a task into subtasks, each routed to a specialist agent.

use std::collections::HashSet;

use crate::server::hub::protocols::error::ProtocolError;
use crate::server::hub::protocols::expander::ProtocolExpander;
use crate::server::hub::protocols::types::{
    EdgeDefinition, InputPortDefinition, OutputPortDefinition, ProtocolConfig, ProtocolExpansion,
    StepDefinition,
};
use crate::server::hub::protocols::{prompt_gen, schema_gen};

/// Expander for the "decomp" (decomposition / fan-out) protocol.
pub struct DecompExpander;

impl ProtocolExpander for DecompExpander {
    fn protocol_type(&self) -> &str {
        "decomp"
    }

    fn description(&self) -> &str {
        "Decompose a task into subtasks and route each to a specialist agent (fan-out)"
    }

    fn validate(&self, config: &ProtocolConfig) -> Result<(), ProtocolError> {
        if config.ports.is_empty() {
            return Err(ProtocolError::NoPorts);
        }

        // Check for duplicate port names
        let mut seen = HashSet::new();
        for port in &config.ports {
            if !seen.insert(&port.port_name) {
                return Err(ProtocolError::DuplicatePortName(port.port_name.clone()));
            }
        }

        Ok(())
    }

    fn generate_schema(&self, config: &ProtocolConfig) -> Result<serde_json::Value, ProtocolError> {
        Ok(schema_gen::decomp_schema(&config.ports))
    }

    fn generate_prompt_injection(&self, config: &ProtocolConfig) -> Result<String, ProtocolError> {
        Ok(prompt_gen::decomp_prompt(&config.ports))
    }

    fn expand(&self, config: &ProtocolConfig) -> Result<ProtocolExpansion, ProtocolError> {
        self.validate(config)?;

        let output_schema = self.generate_schema(config)?;
        let prompt_injection = self.generate_prompt_injection(config)?;

        // Create one downstream step per port
        let steps: Vec<StepDefinition> = config
            .ports
            .iter()
            .map(|port| StepDefinition {
                port_name: port.port_name.clone(),
                agent_id: port.agent_id,
                execution_mode: "single".to_string(),
                prompt_template: None,
                output_schema: None,
            })
            .collect();

        // Create one edge per port: orchestrator → downstream step
        let edges: Vec<EdgeDefinition> = config
            .ports
            .iter()
            .map(|port| EdgeDefinition {
                from_output_port: port.port_name.clone(),
                to_input_port: "task_input".to_string(),
                target_port_name: port.port_name.clone(),
                condition_type: None,
                condition_value: None,
            })
            .collect();

        // Output ports on the orchestrator (one per port name)
        let output_ports: Vec<OutputPortDefinition> = config
            .ports
            .iter()
            .enumerate()
            .map(|(i, port)| OutputPortDefinition {
                port_name: port.port_name.clone(),
                port_type: "object".to_string(),
                json_path: format!("{}.content", i),
                description: Some(format!("Task for {} agent", port.port_name)),
            })
            .collect();

        // Input port on each downstream step
        let input_ports: Vec<InputPortDefinition> = config
            .ports
            .iter()
            .map(|port| InputPortDefinition {
                target_port_name: port.port_name.clone(),
                port_name: "task_input".to_string(),
                port_type: "object".to_string(),
                required: true,
                description: Some(format!("Task input from decomp for {}", port.port_name)),
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
            protocol_type: "decomp".to_string(),
            config: serde_json::json!({}),
            ports: vec![
                PortConfig {
                    port_name: "frontend".to_string(),
                    description: "UI tasks".to_string(),
                    agent_id: Uuid::new_v4(),
                    agent_name: "FE".to_string(),
                    display_order: 0,
                },
                PortConfig {
                    port_name: "backend".to_string(),
                    description: "API tasks".to_string(),
                    agent_id: Uuid::new_v4(),
                    agent_name: "BE".to_string(),
                    display_order: 1,
                },
            ],
        }
    }

    #[test]
    fn validates_no_ports() {
        let config = ProtocolConfig {
            protocol_type: "decomp".to_string(),
            config: serde_json::json!({}),
            ports: vec![],
        };
        assert!(matches!(
            DecompExpander.validate(&config),
            Err(ProtocolError::NoPorts)
        ));
    }

    #[test]
    fn validates_duplicate_ports() {
        let agent_id = Uuid::new_v4();
        let config = ProtocolConfig {
            protocol_type: "decomp".to_string(),
            config: serde_json::json!({}),
            ports: vec![
                PortConfig {
                    port_name: "same".to_string(),
                    description: "".to_string(),
                    agent_id,
                    agent_name: "A".to_string(),
                    display_order: 0,
                },
                PortConfig {
                    port_name: "same".to_string(),
                    description: "".to_string(),
                    agent_id,
                    agent_name: "B".to_string(),
                    display_order: 1,
                },
            ],
        };
        assert!(matches!(
            DecompExpander.validate(&config),
            Err(ProtocolError::DuplicatePortName(_))
        ));
    }

    #[test]
    fn expand_creates_correct_primitives() {
        let config = two_port_config();
        let expansion = DecompExpander.expand(&config).unwrap();

        assert_eq!(expansion.steps.len(), 2);
        assert_eq!(expansion.edges.len(), 2);
        assert_eq!(expansion.output_ports.len(), 2);
        assert_eq!(expansion.input_ports.len(), 2);

        // Each step maps to a port
        assert_eq!(expansion.steps[0].port_name, "frontend");
        assert_eq!(expansion.steps[0].execution_mode, "single");
        assert_eq!(expansion.steps[1].port_name, "backend");

        // Each edge maps orchestrator output port → downstream input port
        assert_eq!(expansion.edges[0].from_output_port, "frontend");
        assert_eq!(expansion.edges[0].to_input_port, "task_input");
        assert_eq!(expansion.edges[1].from_output_port, "backend");
    }
}
