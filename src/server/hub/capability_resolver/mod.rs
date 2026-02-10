//! Capability → tool resolution utility.
//!
//! Resolves a list of capability keys (e.g. `["web_search", "code_analysis"]`)
//! into concrete `Tool` definitions by querying the DB for assigned tools and
//! looking up each tool's runtime definition from the static registry.

use std::collections::HashSet;

use crate::db::traits::ToolCapabilityRepo;
use crate::llm::Tool;
use crate::server::hub::error::HubError;
use crate::tools::registry;

mod tests;

/// Resolve a list of capability keys to concrete Tool definitions.
///
/// Queries the DB for tools assigned to any of the given capabilities,
/// then looks up each tool's runtime definition from the registry.
/// Returns `(tools, tool_names)` with duplicates removed.
pub async fn resolve_capabilities_to_tools(
    capability_keys: &[String],
    repo: &dyn ToolCapabilityRepo,
) -> Result<(Vec<Tool>, Vec<String>), HubError> {
    if capability_keys.is_empty() {
        return Ok((vec![], vec![]));
    }

    let tool_rows = repo
        .get_tools_by_capabilities(capability_keys)
        .await
        .map_err(HubError::Internal)?;

    let mut tools = Vec::new();
    let mut seen = HashSet::new();
    for row in &tool_rows {
        if seen.insert(row.name.clone()) {
            if let Some(tool_def) = registry::get_tool_definition(&row.name) {
                tools.push(tool_def);
            }
        }
    }

    let tool_names = tools.iter().map(|t| t.name.clone()).collect();
    Ok((tools, tool_names))
}
