//! Web tools: outbound HTTP on the agent's behalf.
//!
//! These tools need neither a workspace nor a container — only network access —
//! so they are dispatched ahead of the container and execution-context branches
//! in [`crate::server::tools::execution::dispatch_tool_cascade`]. Routing keys
//! off [`is_web_tool`] rather than a literal name list, so adding a third web
//! tool is a one-line change here instead of an edit in three files with a
//! silent misroute if one is missed.

pub mod format;

use serde_json::Value;

use crate::server::state::AppState;
use crate::server::tools::shared::error_json;

#[cfg(test)]
mod tests;

/// Every tool handled by this module.
///
/// Keep in sync with the arms in [`crate::tools::registry::get_tool_definition`]
/// and the assignments in `config/system/tool_assignments.yaml`. The test
/// `web_tools_all_have_registry_definitions` enforces the first of those.
pub const WEB_TOOLS: &[&str] = &["brave_search", "read_webpage"];

/// Whether `name` is a web tool, and therefore dispatched before the
/// container and execution-context branches of the cascade.
///
/// # Examples
///
/// ```
/// use nexor::server::tools::web::is_web_tool;
///
/// assert!(is_web_tool("brave_search"));
/// assert!(is_web_tool("read_webpage"));
/// assert!(!is_web_tool("run_command"));
/// ```
pub fn is_web_tool(name: &str) -> bool {
    WEB_TOOLS.contains(&name)
}

/// Execute a web tool.
///
/// Called from [`crate::server::tools::execution::dispatch_tool_cascade`] ahead
/// of the container and execution-context branches. The allow-list has already
/// been enforced by the caller.
///
/// Returns the tool result as a `Value::String` on success — the engine
/// forwards a string verbatim to the model, which is what makes the labeled
/// rendering in [`format`] reach it intact. Failures return
/// `{"error": ...}` so the engine's failure breaker can see them.
pub async fn dispatch(name: &str, _input: &Value, _state: Option<&AppState>) -> Value {
    match name {
        // Handlers land in the web-tools slice; the routing above is what
        // needed to exist first, because getting it wrong sends a web tool
        // into the container where it can never run.
        "brave_search" | "read_webpage" => {
            error_json(format!("Tool '{}' is not available yet", name))
        }
        other => error_json(format!("Unknown web tool: {}", other)),
    }
}
