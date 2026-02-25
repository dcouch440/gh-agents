//! Dispatch status module.
//!
//! Renders structured `<dispatch_status>` XML showing active and recent
//! background dispatch tasks for a step. Injected into assistant system
//! prompts so the conversational agent knows what's in flight.

use uuid::Uuid;

use crate::server::state::task_registry::TaskRegistry;

mod fetch;
mod render;
pub mod types;

mod tests;

pub use types::{DispatchSnapshot, DispatchStatus};

/// Build the dispatch status XML for a step.
///
/// Reads from `TaskRegistry`, renders as structured XML.
/// Returns empty string when no tasks exist.
pub fn build(registry: &TaskRegistry, step_id: Uuid) -> String {
    let snapshots = fetch::fetch(registry, step_id);
    render::render(&snapshots)
}
