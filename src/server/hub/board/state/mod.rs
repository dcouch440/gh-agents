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
//! template variable (e.g. `{{.System.board_state}}`).

use anyhow::Result;
use uuid::Uuid;

use crate::db::traits::{SessionRepo, WorkflowRepo};

mod agent;
pub mod enrich;
mod fetch;
mod node;
mod port;
mod render;
pub mod types;

pub use enrich::enrich_design_status;
pub use render::render;
pub use types::{
    AgentDesignStatus, AgentSnapshot, BoardSnapshot, BoardStateVariant, IncomingContextSnapshot,
    InputPortSnapshot, NodeSnapshot, OutputPortSnapshot, Scope,
};

mod tests;

/// Build a [`BoardSnapshot`] without rendering.
///
/// For `OwnNode` scope (L3/L4), fetches the single step and its detail.
/// For `AllNodes` scope (L1/L2), bulk-loads all visible steps in the workflow.
///
/// The returned snapshot can be enriched (e.g. with design status) before
/// rendering via [`render`].
pub async fn build_snapshot(
    repo: &dyn WorkflowRepo,
    sessions: Option<&dyn SessionRepo>,
    variant: BoardStateVariant,
    workflow_id: Uuid,
    step_id: Uuid,
) -> Result<BoardSnapshot> {
    let mut snapshot = match variant.scope() {
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

    // Populate initial_instructions_sent for L1/L2 variants.
    if variant.include_initial_instructions() {
        if let Some(sessions) = sessions {
            let step_ids: Vec<Uuid> = snapshot.nodes.iter().map(|n| n.id).collect();
            if let Ok(instructed) = sessions.check_initial_instructions_sent(&step_ids).await {
                for node in &mut snapshot.nodes {
                    if instructed.contains(&node.id) {
                        node.initial_instructions_sent = true;
                    }
                }
            }
        }
    }

    Ok(snapshot)
}

/// Build `<board_state>` XML for the given variant (convenience wrapper).
///
/// Calls [`build_snapshot`] then [`render`].
pub async fn build(
    repo: &dyn WorkflowRepo,
    sessions: Option<&dyn SessionRepo>,
    variant: BoardStateVariant,
    workflow_id: Uuid,
    step_id: Uuid,
) -> Result<String> {
    let snapshot = build_snapshot(repo, sessions, variant, workflow_id, step_id).await?;
    Ok(render(&snapshot, variant))
}
