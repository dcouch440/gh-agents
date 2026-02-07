//! Transform protocol expander — 1:1 processing pattern.
//!
//! Takes input, produces structured output. Used for ticket designers,
//! summarizers, formatters, etc.

use crate::server::hub::protocols::error::ProtocolError;
use crate::server::hub::protocols::expander::ProtocolExpander;
use crate::server::hub::protocols::types::{ProtocolConfig, ProtocolExpansion};
use crate::server::hub::protocols::{prompt_gen, schema_gen};

/// Expander for the "transform" (1:1 processing) protocol.
pub struct TransformExpander;

impl ProtocolExpander for TransformExpander {
    fn protocol_type(&self) -> &str {
        "transform"
    }

    fn description(&self) -> &str {
        "Process input and produce structured output (1:1 transform)"
    }

    fn validate(&self, _config: &ProtocolConfig) -> Result<(), ProtocolError> {
        // Transform is valid with zero ports (it IS the step, no downstream).
        // Config may optionally contain an "output_schema" key.
        Ok(())
    }

    fn generate_schema(&self, config: &ProtocolConfig) -> Result<serde_json::Value, ProtocolError> {
        let user_schema = config.config.get("output_schema");
        Ok(schema_gen::transform_schema(user_schema))
    }

    fn generate_prompt_injection(&self, config: &ProtocolConfig) -> Result<String, ProtocolError> {
        let desc = config.config.get("description").and_then(|v| v.as_str());
        Ok(prompt_gen::transform_prompt(desc))
    }

    fn expand(&self, config: &ProtocolConfig) -> Result<ProtocolExpansion, ProtocolError> {
        self.validate(config)?;

        let output_schema = self.generate_schema(config)?;
        let prompt_injection = self.generate_prompt_injection(config)?;

        // Transform creates no downstream steps — it IS the processing step.
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
            protocol_type: "transform".to_string(),
            config: serde_json::json!({
                "output_schema": {
                    "type": "object",
                    "properties": {
                        "title": {"type": "string"},
                        "acceptance_criteria": {"type": "array", "items": {"type": "string"}}
                    }
                }
            }),
            ports: vec![],
        };

        let expansion = TransformExpander.expand(&config).unwrap();
        assert!(expansion.steps.is_empty());
        assert!(expansion.edges.is_empty());
        assert_eq!(expansion.output_schema["type"], "object");
        assert!(expansion
            .output_schema
            .get("properties")
            .and_then(|p| p.get("title"))
            .is_some());
    }

    #[test]
    fn expand_with_description() {
        let config = ProtocolConfig {
            protocol_type: "transform".to_string(),
            config: serde_json::json!({
                "description": "Create a detailed ticket"
            }),
            ports: vec![],
        };

        let expansion = TransformExpander.expand(&config).unwrap();
        assert!(expansion.prompt_injection.contains("detailed ticket"));
    }
}
