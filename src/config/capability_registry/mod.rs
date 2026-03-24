//! In-memory capability registry loaded from YAML at startup.
//!
//! Resolves capability keys (e.g. `["file_read", "content_search"]`) to
//! concrete `Tool` definitions by looking up tool assignments from
//! `config/system/tool_assignments.yaml` and fetching each tool's runtime
//! definition from the static registry.
//!
//! Replaces the previous DB-based resolution that required syncing YAML
//! to PostgreSQL and querying at runtime.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::{Context, Result};

use crate::config::sync::{CapabilitiesYaml, ToolAssignmentsYaml};
use crate::llm::Tool;
use crate::tools::registry;

#[cfg(test)]
mod tests;

/// In-memory capability registry backed by YAML config files.
///
/// Loaded once at startup and shared immutably via `Arc<CapabilityRegistry>`
/// in `AppState`. Provides fast, DB-free capability → tool resolution.
#[derive(Debug)]
pub struct CapabilityRegistry {
    /// capability_key → [tool_names] (reverse index from tool_assignments.yaml)
    capability_to_tools: HashMap<String, Vec<String>>,
}

impl CapabilityRegistry {
    /// Load the registry from YAML files in the config directory.
    ///
    /// Reads `system/capabilities.yaml` for descriptions and
    /// `system/tool_assignments.yaml` for capability → tool mappings.
    pub fn load(config_dir: &Path) -> Result<Self> {
        let _capabilities = load_capabilities_yaml(config_dir)?;
        let tool_assignments = load_tool_assignments_yaml(config_dir)?;

        // Build reverse index: capability_key → [tool_names]
        let mut capability_to_tools: HashMap<String, Vec<String>> = HashMap::new();
        for (tool_name, assignment) in &tool_assignments.tool_assignments {
            for cap_key in &assignment.capabilities {
                capability_to_tools
                    .entry(cap_key.clone())
                    .or_default()
                    .push(tool_name.clone());
            }
        }

        Ok(Self {
            capability_to_tools,
        })
    }

    /// Create an empty registry (for tests that don't need capabilities).
    pub fn empty() -> Self {
        Self {
            capability_to_tools: HashMap::new(),
        }
    }

    /// Resolve capability keys to concrete `Tool` definitions.
    ///
    /// For each capability key, finds all tools that provide it (from
    /// tool_assignments.yaml), then looks up each tool's runtime definition
    /// from the static registry. Returns `(tools, tool_names)` with
    /// duplicates removed.
    pub fn resolve_tools(&self, capability_keys: &[String]) -> (Vec<Tool>, Vec<String>) {
        if capability_keys.is_empty() {
            return (vec![], vec![]);
        }

        let mut tools = Vec::new();
        let mut seen = HashSet::new();

        for cap_key in capability_keys {
            if let Some(tool_names) = self.capability_to_tools.get(cap_key) {
                for tool_name in tool_names {
                    if seen.insert(tool_name.clone()) {
                        if let Some(tool_def) = registry::get_tool_definition(tool_name) {
                            tools.push(tool_def);
                        }
                    }
                }
            }
        }

        let tool_names = tools.iter().map(|t| t.name.clone()).collect();
        (tools, tool_names)
    }
}

/// Load capabilities.yaml from the system/ subdirectory.
fn load_capabilities_yaml(config_dir: &Path) -> Result<CapabilitiesYaml> {
    let path = config_dir.join("system/capabilities.yaml");
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    serde_yaml::from_str(&content).with_context(|| format!("Failed to parse {}", path.display()))
}

/// Load tool_assignments.yaml from the system/ subdirectory.
fn load_tool_assignments_yaml(config_dir: &Path) -> Result<ToolAssignmentsYaml> {
    let path = config_dir.join("system/tool_assignments.yaml");
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    serde_yaml::from_str(&content).with_context(|| format!("Failed to parse {}", path.display()))
}
