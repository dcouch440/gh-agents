//! Review protocol expander — quality gate pattern.
//!
//! Examines input and makes a decision (approve, reject, revise, etc.).
//! Can create conditional edges for routing based on the decision.

use crate::server::hub::protocols::error::ProtocolError;
use crate::server::hub::protocols::expander::ProtocolExpander;
use crate::server::hub::protocols::types::{
    EdgeDefinition, OutputPortDefinition, ProtocolConfig, ProtocolExpansion,
};
use crate::server::hub::protocols::{prompt_gen, schema_gen};

const DEFAULT_DECISIONS: &[&str] = &["approve", "reject", "revise"];

/// Expander for the "review" (quality gate) protocol.
pub struct ReviewExpander;

impl ReviewExpander {
    /// Extract decision options from config, falling back to defaults.
    fn get_decisions(config: &ProtocolConfig) -> Vec<String> {
        config
            .config
            .get("decisions")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_else(|| DEFAULT_DECISIONS.iter().map(|s| s.to_string()).collect())
    }
}

impl ProtocolExpander for ReviewExpander {
    fn protocol_type(&self) -> &str {
        "review"
    }

    fn description(&self) -> &str {
        "Review input and make a decision (quality gate with conditional routing)"
    }

    fn validate(&self, config: &ProtocolConfig) -> Result<(), ProtocolError> {
        let decisions = Self::get_decisions(config);
        if decisions.is_empty() {
            return Err(ProtocolError::InvalidConfig(
                "Review protocol requires at least one decision option".to_string(),
            ));
        }
        Ok(())
    }

    fn generate_schema(&self, config: &ProtocolConfig) -> Result<serde_json::Value, ProtocolError> {
        let decisions = Self::get_decisions(config);
        Ok(schema_gen::review_schema(&decisions))
    }

    fn generate_prompt_injection(&self, config: &ProtocolConfig) -> Result<String, ProtocolError> {
        let decisions = Self::get_decisions(config);
        Ok(prompt_gen::review_prompt(&decisions))
    }

    fn expand(&self, config: &ProtocolConfig) -> Result<ProtocolExpansion, ProtocolError> {
        self.validate(config)?;

        let decisions = Self::get_decisions(config);
        let output_schema = self.generate_schema(config)?;
        let prompt_injection = self.generate_prompt_injection(config)?;

        // Create conditional edges — one per decision option.
        // These are templates; the apply step will wire them to actual target steps.
        let edges: Vec<EdgeDefinition> = decisions
            .iter()
            .map(|decision| EdgeDefinition {
                from_output_port: "decision".to_string(),
                to_input_port: "review_result".to_string(),
                target_port_name: decision.clone(),
                condition_type: Some("equals".to_string()),
                condition_value: Some(serde_json::json!({
                    "field": "decision",
                    "value": decision
                })),
            })
            .collect();

        // Output port for the decision
        let output_ports = vec![OutputPortDefinition {
            port_name: "decision".to_string(),
            port_type: "object".to_string(),
            json_path: "decision".to_string(),
            description: Some("The review decision and feedback".to_string()),
        }];

        Ok(ProtocolExpansion {
            output_schema,
            prompt_injection,
            steps: vec![], // Review doesn't create downstream steps by itself
            edges,
            output_ports,
            input_ports: vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_uses_default_decisions() {
        let config = ProtocolConfig {
            protocol_type: "review".to_string(),
            config: serde_json::json!({}),
            ports: vec![],
        };

        let expansion = ReviewExpander.expand(&config).unwrap();
        let decision_enum = &expansion.output_schema["properties"]["decision"]["enum"];
        assert_eq!(decision_enum[0], "approve");
        assert_eq!(decision_enum[1], "reject");
        assert_eq!(decision_enum[2], "revise");

        // One conditional edge per decision
        assert_eq!(expansion.edges.len(), 3);
        assert_eq!(expansion.edges[0].target_port_name, "approve");
        assert!(expansion.edges[0].condition_type.is_some());
    }

    #[test]
    fn expand_uses_custom_decisions() {
        let config = ProtocolConfig {
            protocol_type: "review".to_string(),
            config: serde_json::json!({
                "decisions": ["pass", "fail"]
            }),
            ports: vec![],
        };

        let expansion = ReviewExpander.expand(&config).unwrap();
        let decision_enum = &expansion.output_schema["properties"]["decision"]["enum"];
        assert_eq!(decision_enum[0], "pass");
        assert_eq!(decision_enum[1], "fail");
        assert_eq!(expansion.edges.len(), 2);
    }

    #[test]
    fn expand_creates_no_downstream_steps() {
        let config = ProtocolConfig {
            protocol_type: "review".to_string(),
            config: serde_json::json!({}),
            ports: vec![],
        };

        let expansion = ReviewExpander.expand(&config).unwrap();
        assert!(expansion.steps.is_empty());
    }
}
