//! Decomp protocol expander — fan-out pattern (1 → N) via label routing.
//!
//! Decomposes a task into subtasks and dispatches them via a single for_each
//! step with `routing_mode="label"`. Each port becomes a routing rule mapping
//! the port name to an agent. Multi-assignment works naturally — multiple
//! items with the same port label all route to that port's agent.

use std::collections::HashSet;

use once_cell::sync::Lazy;
use regex::Regex;

use crate::server::hub::protocols::error::ProtocolError;
use crate::server::hub::protocols::expander::ProtocolExpander;
use crate::server::hub::protocols::types::{
    EdgeDefinition, InputPortDefinition, OutputPortDefinition, ProtocolConfig, ProtocolExpansion,
    RoutingRuleDefinition, StepDefinition,
};
use crate::server::hub::protocols::{prompt_gen, schema_gen};

/// Regex for valid port names: lowercase alphanumeric + underscores, starting with a letter.
static PORT_NAME_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[a-z][a-z0-9_]*$").unwrap());

/// Maximum allowed port name length.
const MAX_PORT_NAME_LEN: usize = 50;

/// Validate that a port name is a valid slug.
fn validate_port_name(name: &str) -> bool {
    !name.is_empty() && name.len() <= MAX_PORT_NAME_LEN && PORT_NAME_REGEX.is_match(name)
}

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

        let mut seen = HashSet::new();
        for port in &config.ports {
            // Validate port name format
            if !validate_port_name(&port.port_name) {
                return Err(ProtocolError::InvalidPortName(port.port_name.clone()));
            }

            // Check for duplicates
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

        // Build routing rules: one per port, mapping port_name → agent_id
        let routing_rules: Vec<RoutingRuleDefinition> = config
            .ports
            .iter()
            .map(|port| RoutingRuleDefinition {
                label_value: port.port_name.clone(),
                description: Some(port.description.clone()),
                agent_id: port.agent_id,
                display_order: port.display_order,
            })
            .collect();

        // Single dispatch step: for_each with label routing.
        // The DAG executor iterates the output array, reads the "port" field from
        // each element, and dispatches to the agent via routing rules.
        let fallback_agent_id = config.ports[0].agent_id;
        let steps = vec![StepDefinition {
            port_name: "dispatch".to_string(),
            agent_id: Some(fallback_agent_id),
            execution_mode: "for_each".to_string(),
            prompt_template: None,
            output_schema: None,
            routing_mode: Some("label".to_string()),
            routing_field: Some("port".to_string()),
            for_each_label_field: Some("port".to_string()),
            for_each_ref: Some("{anchor_output}".to_string()),
            routing_rules,
        }];

        // Single edge: orchestrator → dispatch step
        let edges = vec![EdgeDefinition {
            from_output_port: "tasks".to_string(),
            to_input_port: "task_input".to_string(),
            target_port_name: "dispatch".to_string(),
            condition_type: None,
            condition_value: None,
        }];

        // Single output port on the orchestrator for the task array
        let output_ports = vec![OutputPortDefinition {
            port_name: "tasks".to_string(),
            port_type: "array".to_string(),
            json_path: "$".to_string(),
            description: Some("Array of decomposed tasks with port labels".to_string()),
        }];

        // Single input port on the dispatch step
        let input_ports = vec![InputPortDefinition {
            target_port_name: "dispatch".to_string(),
            port_name: "task_input".to_string(),
            port_type: "array".to_string(),
            required: true,
            description: Some("Task array for label-routed dispatch".to_string()),
        }];

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
                    agent_tools: vec![],
                    display_order: 0,
                    content_schema: None,
                },
                PortConfig {
                    port_name: "backend".to_string(),
                    description: "API tasks".to_string(),
                    agent_id: Uuid::new_v4(),
                    agent_name: "BE".to_string(),
                    agent_tools: vec![],
                    display_order: 1,
                    content_schema: None,
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
                    agent_tools: vec![],
                    display_order: 0,
                    content_schema: None,
                },
                PortConfig {
                    port_name: "same".to_string(),
                    description: "".to_string(),
                    agent_id,
                    agent_name: "B".to_string(),
                    agent_tools: vec![],
                    display_order: 1,
                    content_schema: None,
                },
            ],
        };
        assert!(matches!(
            DecompExpander.validate(&config),
            Err(ProtocolError::DuplicatePortName(_))
        ));
    }

    #[test]
    fn validates_invalid_port_names() {
        let agent_id = Uuid::new_v4();
        // Spaces not allowed
        let config = ProtocolConfig {
            protocol_type: "decomp".to_string(),
            config: serde_json::json!({}),
            ports: vec![PortConfig {
                port_name: "front end".to_string(),
                description: "".to_string(),
                agent_id,
                agent_name: "A".to_string(),
                agent_tools: vec![],
                display_order: 0,
                content_schema: None,
            }],
        };
        assert!(matches!(
            DecompExpander.validate(&config),
            Err(ProtocolError::InvalidPortName(_))
        ));

        // Uppercase not allowed
        let config = ProtocolConfig {
            protocol_type: "decomp".to_string(),
            config: serde_json::json!({}),
            ports: vec![PortConfig {
                port_name: "Frontend".to_string(),
                description: "".to_string(),
                agent_id,
                agent_name: "A".to_string(),
                agent_tools: vec![],
                display_order: 0,
                content_schema: None,
            }],
        };
        assert!(matches!(
            DecompExpander.validate(&config),
            Err(ProtocolError::InvalidPortName(_))
        ));

        // Special chars not allowed
        let config = ProtocolConfig {
            protocol_type: "decomp".to_string(),
            config: serde_json::json!({}),
            ports: vec![PortConfig {
                port_name: "c++/systems".to_string(),
                description: "".to_string(),
                agent_id,
                agent_name: "A".to_string(),
                agent_tools: vec![],
                display_order: 0,
                content_schema: None,
            }],
        };
        assert!(matches!(
            DecompExpander.validate(&config),
            Err(ProtocolError::InvalidPortName(_))
        ));
    }

    #[test]
    fn validates_valid_port_names() {
        let config = two_port_config();
        assert!(DecompExpander.validate(&config).is_ok());

        // Underscores and numbers allowed
        let agent_id = Uuid::new_v4();
        let config = ProtocolConfig {
            protocol_type: "decomp".to_string(),
            config: serde_json::json!({}),
            ports: vec![PortConfig {
                port_name: "agent_v2".to_string(),
                description: "".to_string(),
                agent_id,
                agent_name: "A".to_string(),
                agent_tools: vec![],
                display_order: 0,
                content_schema: None,
            }],
        };
        assert!(DecompExpander.validate(&config).is_ok());
    }

    #[test]
    fn expand_creates_single_dispatch_step() {
        let config = two_port_config();
        let expansion = DecompExpander.expand(&config).unwrap();

        // Single dispatch step (not N steps)
        assert_eq!(expansion.steps.len(), 1);
        let step = &expansion.steps[0];
        assert_eq!(step.port_name, "dispatch");
        assert_eq!(step.execution_mode, "for_each");
        assert_eq!(step.routing_mode.as_deref(), Some("label"));
        assert_eq!(step.for_each_label_field.as_deref(), Some("port"));
        assert_eq!(step.for_each_ref.as_deref(), Some("{anchor_output}"));

        // Routing rules match ports
        assert_eq!(step.routing_rules.len(), 2);
        assert_eq!(step.routing_rules[0].label_value, "frontend");
        assert_eq!(step.routing_rules[0].agent_id, config.ports[0].agent_id);
        assert_eq!(step.routing_rules[1].label_value, "backend");
        assert_eq!(step.routing_rules[1].agent_id, config.ports[1].agent_id);

        // Single edge: orchestrator → dispatch
        assert_eq!(expansion.edges.len(), 1);
        assert_eq!(expansion.edges[0].from_output_port, "tasks");
        assert_eq!(expansion.edges[0].to_input_port, "task_input");
        assert_eq!(expansion.edges[0].target_port_name, "dispatch");

        // Single output port for the task array
        assert_eq!(expansion.output_ports.len(), 1);
        assert_eq!(expansion.output_ports[0].port_name, "tasks");
        assert_eq!(expansion.output_ports[0].port_type, "array");

        // Single input port on dispatch step
        assert_eq!(expansion.input_ports.len(), 1);
        assert_eq!(expansion.input_ports[0].target_port_name, "dispatch");
    }

    #[test]
    fn expand_uses_first_port_agent_as_fallback() {
        let config = two_port_config();
        let expansion = DecompExpander.expand(&config).unwrap();

        // Dispatch step's agent_id should be first port's agent (fallback)
        assert_eq!(expansion.steps[0].agent_id, Some(config.ports[0].agent_id));
    }
}
