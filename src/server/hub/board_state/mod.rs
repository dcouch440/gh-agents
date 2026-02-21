//! Board state module — renders `<board_state>` XML for system prompts.
//!
//! Produces structured XML snapshots of the workflow board at different
//! zoom levels, one per layer of the manager node stack:
//!
//! | Variant            | Scope    | Detail level                              |
//! |--------------------|----------|-------------------------------------------|
//! | `ManagerAssistant` | All nodes | Compressed status, `<asking>`, no ids    |
//! | `ManagerBuilder`   | All nodes | Ids, capabilities, agent summary         |
//! | `NodeAssistant`    | Own node  | Agents, incoming ports, no agent ids     |
//! | `Dispatch`         | Own node  | Full detail: port schemas, roster, notes |
//!
//! # Usage
//!
//! ```ignore
//! let xml = board_state::build(
//!     repo,
//!     BoardStateVariant::NodeAssistant,
//!     workflow_id,
//!     step_id,
//! ).await?;
//! ```
//!
//! The returned string is ready for injection into a system prompt
//! template variable (e.g. `{{.System.current_config}}`).

use anyhow::Result;
use uuid::Uuid;

use crate::db::traits::WorkflowRepo;

mod agent;
mod fetch;
mod node;
mod port;
mod render;
pub mod types;

pub use types::{
    AgentSnapshot, BoardSnapshot, BoardStateVariant, IncomingContextSnapshot, InputPortSnapshot,
    NodeSnapshot, OutputPortSnapshot, Scope,
};

mod tests;

/// Build `<board_state>` XML for the given variant.
///
/// For `OwnNode` scope (L3/L4), fetches the single step and its detail.
/// For `AllNodes` scope (L1/L2), bulk-loads all visible steps in the workflow.
pub async fn build(
    repo: &dyn WorkflowRepo,
    variant: BoardStateVariant,
    workflow_id: Uuid,
    step_id: Uuid,
) -> Result<String> {
    let snapshot = match variant.scope() {
        Scope::OwnNode => {
            let node = fetch::fetch_node(repo, workflow_id, step_id).await?;
            BoardSnapshot {
                workflow_name: String::new(),
                workflow_id,
                nodes: vec![node],
                available_capabilities: vec![],
            }
        }
        Scope::AllNodes => fetch::fetch_board(repo, workflow_id).await?,
    };

    Ok(render::render(&snapshot, variant))
}
