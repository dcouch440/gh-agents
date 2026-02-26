//! Per-node instruction builder — converts a board changeset into individual
//! dispatch instructions for each affected node's L4 workforce builder.
//!
//! Pure function: no IO, no async, no DB calls. Takes the `FilteredChangeset`,
//! `PhaseZeroResult`, and `CanvasSnapshot` and produces one instruction per
//! node that needs configuration.

use std::collections::HashMap;

use uuid::Uuid;

use crate::server::hub::board_serializer::{
    CanvasNode, CanvasSnapshot, FilteredChangeset, NodeUpdate, ScoredChange,
};

use super::executor::PhaseZeroResult;

/// Whether a node is new or being updated.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeChangeType {
    New,
    Updated,
}

/// A dispatch instruction for a single node's L4 workforce builder.
#[derive(Debug, Clone)]
pub struct NodeDispatchInstruction {
    /// Excalidraw element ID of the canvas node.
    pub element_id: String,
    /// DB step ID created/updated by Phase 0.
    pub step_id: Uuid,
    /// The step's execution mode (e.g. "workforce").
    pub execution_mode: String,
    /// The instruction text for the L4 builder.
    pub instruction: String,
    /// Whether this is a new node or an update.
    pub change_type: NodeChangeType,
}

/// Build per-node dispatch instructions from a board changeset.
///
/// Returns an empty vec if:
/// - `should_dispatch` is false
/// - `meaningful` changes are empty
/// - No node changes exist (only edge changes)
pub fn build_per_node_instructions(
    changeset: &FilteredChangeset,
    phase_zero: &PhaseZeroResult,
    snapshot: &CanvasSnapshot,
) -> Vec<NodeDispatchInstruction> {
    if !changeset.should_dispatch || changeset.meaningful.is_empty() {
        return vec![];
    }

    // Build element_id → (step_id, execution_mode) lookup from Phase 0 results
    let mut step_lookup: HashMap<&str, (Uuid, &str)> = HashMap::new();
    for (eid, step) in &phase_zero.created_steps {
        step_lookup.insert(eid.as_str(), (step.id, step.execution_mode.as_str()));
    }
    for (eid, step) in &phase_zero.updated_steps {
        step_lookup.insert(eid.as_str(), (step.id, step.execution_mode.as_str()));
    }

    // Collect global notes for new node instructions
    let global_notes: Vec<&str> = snapshot
        .global_notes
        .iter()
        .map(|n| n.text.as_str())
        .collect();

    let mut instructions = Vec::new();

    for change in &changeset.meaningful {
        match change {
            ScoredChange::NewNode { node, .. } => {
                let Some(&(step_id, execution_mode)) = step_lookup.get(node.element_id.as_str())
                else {
                    continue;
                };

                instructions.push(NodeDispatchInstruction {
                    element_id: node.element_id.clone(),
                    step_id,
                    execution_mode: execution_mode.to_string(),
                    instruction: format_new_node(node, &global_notes),
                    change_type: NodeChangeType::New,
                });
            }
            ScoredChange::UpdatedNode { update, .. } => {
                let Some(&(step_id, execution_mode)) = step_lookup.get(update.element_id.as_str())
                else {
                    continue;
                };

                instructions.push(NodeDispatchInstruction {
                    element_id: update.element_id.clone(),
                    step_id,
                    execution_mode: execution_mode.to_string(),
                    instruction: format_updated_node(update, &global_notes),
                    change_type: NodeChangeType::Updated,
                });
            }
            // Edges don't need builder dispatch — topology is handled by Phase 0
            ScoredChange::NewEdge { .. } => {}
        }
    }

    instructions
}

/// Format the instruction for a new canvas node.
fn format_new_node(node: &CanvasNode, global_notes: &[&str]) -> String {
    let mut parts = Vec::new();

    parts.push("Configure this new workflow node.".to_string());

    parts.push(format!("<user_text>\n{}\n</user_text>", node.raw_text));

    if !node.annotations.is_empty() {
        let items: Vec<String> = node.annotations.iter().map(|a| format!("- {a}")).collect();
        parts.push(format!(
            "<annotations>\n{}\n</annotations>",
            items.join("\n")
        ));
    }

    if let Some(encoding) = &node.stroke_encoding {
        parts.push(format!("<sketch>\n{encoding}\n</sketch>"));
    } else if let Some(sketch) = &node.sketch {
        parts.push(format!("<sketch>\n{sketch}\n</sketch>"));
    }

    if !global_notes.is_empty() {
        let items: Vec<String> = global_notes.iter().map(|n| format!("- {n}")).collect();
        parts.push(format!(
            "<board_notes>\n{}\n</board_notes>",
            items.join("\n")
        ));
    }

    parts.join("\n\n")
}

/// Format the instruction for an updated canvas node.
fn format_updated_node(update: &NodeUpdate, global_notes: &[&str]) -> String {
    let mut parts = Vec::new();

    parts.push("The user updated this node on the canvas.".to_string());

    parts.push(format!(
        "<change>\nBefore: \"{}\"\nAfter: \"{}\"\n</change>",
        update.old_text, update.new_text
    ));

    if update.old_annotations != update.new_annotations {
        let removed: Vec<&String> = update
            .old_annotations
            .iter()
            .filter(|a| !update.new_annotations.contains(a))
            .collect();
        let added: Vec<&String> = update
            .new_annotations
            .iter()
            .filter(|a| !update.old_annotations.contains(a))
            .collect();
        let kept: Vec<&String> = update
            .new_annotations
            .iter()
            .filter(|a| update.old_annotations.contains(a))
            .collect();

        let mut ann_parts = Vec::new();
        for a in &removed {
            ann_parts.push(format!("- [removed] {a}"));
        }
        for a in &added {
            ann_parts.push(format!("- [added] {a}"));
        }
        for a in &kept {
            ann_parts.push(format!("- {a}"));
        }

        parts.push(format!(
            "<annotations>\n{}\n</annotations>",
            ann_parts.join("\n")
        ));
    }

    if let Some(encoding) = &update.stroke_encoding {
        parts.push(format!("<sketch>\n{encoding}\n</sketch>"));
    } else if let Some(sketch) = &update.sketch {
        parts.push(format!("<sketch>\n{sketch}\n</sketch>"));
    }

    if !global_notes.is_empty() {
        let items: Vec<String> = global_notes.iter().map(|n| format!("- {n}")).collect();
        parts.push(format!(
            "<board_notes>\n{}\n</board_notes>",
            items.join("\n")
        ));
    }

    parts.join("\n\n")
}

#[cfg(test)]
#[path = "instruction_tests.rs"]
mod instruction_tests;
