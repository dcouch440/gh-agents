//! Documenter protocol expander — structured document generation pipeline.
//!
//! Unlike other protocols that create downstream workflow steps and edges,
//! the documenter creates NO downstream primitives. Instead, the phased
//! pipeline (strategy → research → write) executes at runtime via
//! `DocumenterExecutor`. This expander handles only schema and prompt
//! generation for the strategy phase.

use crate::server::hub::protocols::error::ProtocolError;
use crate::server::hub::protocols::expander::ProtocolExpander;
use crate::server::hub::protocols::types::{ProtocolConfig, ProtocolExpansion};
use crate::server::hub::protocols::{prompt_gen, schema_gen};

mod tests;

/// Expander for the "documenter" (document generation) protocol.
pub struct DocumenterExpander;

/// Extract document definitions from the protocol config.
///
/// Returns `Err(InvalidConfig)` if `document_defs` is missing or not an array.
fn extract_doc_defs(config: &ProtocolConfig) -> Result<Vec<serde_json::Value>, ProtocolError> {
    config
        .config
        .get("document_defs")
        .and_then(|v| v.as_array())
        .cloned()
        .ok_or_else(|| {
            ProtocolError::InvalidConfig(
                "Documenter protocol requires a \"document_defs\" array in config".to_string(),
            )
        })
}

/// Extract available capability keys from the protocol config.
///
/// Returns an empty vec if not present (capabilities are optional).
fn extract_capabilities(config: &ProtocolConfig) -> Vec<String> {
    config
        .config
        .get("available_capabilities")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

impl ProtocolExpander for DocumenterExpander {
    fn protocol_type(&self) -> &str {
        "documenter"
    }

    fn description(&self) -> &str {
        "Generate structured documents using a strategy-research-write pipeline"
    }

    fn validate(&self, config: &ProtocolConfig) -> Result<(), ProtocolError> {
        let defs = extract_doc_defs(config)?;

        if defs.is_empty() {
            return Err(ProtocolError::InvalidConfig(
                "Documenter protocol requires at least one document definition".to_string(),
            ));
        }

        for (i, def) in defs.iter().enumerate() {
            let name = def["name"].as_str().unwrap_or("");
            if name.is_empty() {
                return Err(ProtocolError::InvalidConfig(format!(
                    "Document definition at index {} is missing a non-empty \"name\"",
                    i
                )));
            }

            if let Some(target) = def.get("target_length") {
                if let Some(n) = target.as_i64() {
                    if n <= 0 {
                        return Err(ProtocolError::InvalidConfig(format!(
                            "Document definition \"{}\" has invalid target_length: must be positive",
                            name
                        )));
                    }
                }
            }
        }

        Ok(())
    }

    fn generate_schema(&self, config: &ProtocolConfig) -> Result<serde_json::Value, ProtocolError> {
        let defs = extract_doc_defs(config)?;
        Ok(schema_gen::documenter_schema(&defs))
    }

    fn generate_prompt_injection(&self, config: &ProtocolConfig) -> Result<String, ProtocolError> {
        let defs = extract_doc_defs(config)?;
        let capabilities = extract_capabilities(config);
        // Context documents may or may not be present at runtime; always enable the
        // instruction so the strategy LLM knows the field exists.
        Ok(prompt_gen::documenter_prompt(&defs, &capabilities, true))
    }

    fn expand(&self, config: &ProtocolConfig) -> Result<ProtocolExpansion, ProtocolError> {
        self.validate(config)?;

        let output_schema = self.generate_schema(config)?;
        let prompt_injection = self.generate_prompt_injection(config)?;

        // The documenter creates no downstream steps or edges.
        // The entire research/write pipeline runs inside DocumenterExecutor at runtime.
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
