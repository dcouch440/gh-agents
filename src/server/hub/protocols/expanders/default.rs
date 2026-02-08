//! Default protocol expander — simple structured output.
//!
//! Applies a standard `{"response": "string"}` output schema so every agent
//! gets structured output without manual schema creation. The existing filter
//! pipeline (reasoning trace, schema enhancement, validation retry) activates
//! automatically when the output schema is set.

use crate::server::hub::protocols::error::ProtocolError;
use crate::server::hub::protocols::expander::ProtocolExpander;
use crate::server::hub::protocols::types::{ProtocolConfig, ProtocolExpansion};

/// Expander for the "default" (simple structured output) protocol.
pub struct DefaultExpander;

impl ProtocolExpander for DefaultExpander {
    fn protocol_type(&self) -> &str {
        "default"
    }

    fn description(&self) -> &str {
        "Standard structured output with a response field"
    }

    fn validate(&self, _config: &ProtocolConfig) -> Result<(), ProtocolError> {
        Ok(())
    }

    fn generate_schema(&self, config: &ProtocolConfig) -> Result<serde_json::Value, ProtocolError> {
        // Use config.output_schema if provided, otherwise the standard response schema.
        match config.config.get("output_schema") {
            Some(schema) => Ok(schema.clone()),
            None => Ok(serde_json::json!({
                "type": "object",
                "required": ["response"],
                "properties": {
                    "response": { "type": "string" }
                },
                "additionalProperties": false
            })),
        }
    }

    fn generate_prompt_injection(&self, _config: &ProtocolConfig) -> Result<String, ProtocolError> {
        // No prompt injection — the step's own prompt_template provides the task.
        Ok(String::new())
    }

    fn expand(&self, config: &ProtocolConfig) -> Result<ProtocolExpansion, ProtocolError> {
        self.validate(config)?;

        let output_schema = self.generate_schema(config)?;
        let prompt_injection = self.generate_prompt_injection(config)?;

        // Default creates no downstream steps — it IS the processing step.
        Ok(ProtocolExpansion {
            output_schema,
            prompt_injection,
            steps: vec![],
            edges: vec![],
            output_ports: vec![],
            input_ports: vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_creates_no_downstream() {
        let config = ProtocolConfig {
            protocol_type: "default".to_string(),
            config: serde_json::json!({}),
            ports: vec![],
        };

        let expansion = DefaultExpander.expand(&config).unwrap();
        assert!(expansion.steps.is_empty());
        assert!(expansion.edges.is_empty());
        assert!(expansion.output_ports.is_empty());
        assert!(expansion.input_ports.is_empty());
    }

    #[test]
    fn expand_uses_fallback_schema() {
        let config = ProtocolConfig {
            protocol_type: "default".to_string(),
            config: serde_json::json!({}),
            ports: vec![],
        };

        let expansion = DefaultExpander.expand(&config).unwrap();
        assert_eq!(expansion.output_schema["type"], "object");
        assert!(expansion
            .output_schema
            .get("properties")
            .and_then(|p| p.get("response"))
            .is_some());
    }

    #[test]
    fn expand_uses_config_schema_when_provided() {
        let config = ProtocolConfig {
            protocol_type: "default".to_string(),
            config: serde_json::json!({
                "output_schema": {
                    "type": "object",
                    "properties": {
                        "title": {"type": "string"},
                        "body": {"type": "string"}
                    }
                }
            }),
            ports: vec![],
        };

        let expansion = DefaultExpander.expand(&config).unwrap();
        assert!(expansion
            .output_schema
            .get("properties")
            .and_then(|p| p.get("title"))
            .is_some());
    }

    #[test]
    fn prompt_injection_is_empty() {
        let config = ProtocolConfig {
            protocol_type: "default".to_string(),
            config: serde_json::json!({}),
            ports: vec![],
        };

        let expansion = DefaultExpander.expand(&config).unwrap();
        assert!(expansion.prompt_injection.is_empty());
    }
}
