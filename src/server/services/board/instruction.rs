//! Instruction formatter — converts a board changeset into a structured
//! instruction string for the Board Dispatcher agent.
//!
//! Pure function: no IO, no async, no DB calls. Takes the `FilteredChangeset`,
//! `PhaseZeroResult`, and `CanvasSnapshot` and produces an XML-formatted
//! instruction that the Board Dispatcher reads alongside `board_state`.

use std::collections::HashMap;

use crate::server::hub::board_serializer::{
    CanvasSnapshot, FilteredChangeset, GlobalNote, ScoredChange,
};

use super::executor::PhaseZeroResult;

/// Format a board changeset into an instruction string for the Board Dispatcher.
///
/// Returns `None` if:
/// - `should_dispatch` is false
/// - `meaningful` changes are empty
pub fn format_board_instruction(
    changeset: &FilteredChangeset,
    phase_zero: &PhaseZeroResult,
    snapshot: &CanvasSnapshot,
) -> Option<String> {
    if !changeset.should_dispatch || changeset.meaningful.is_empty() {
        return None;
    }

    // Build element_id → ref_id lookup from Phase 0 results
    let mut ref_ids: HashMap<&str, &str> = HashMap::new();
    for (eid, step) in &phase_zero.created_steps {
        ref_ids.insert(eid.as_str(), step.ref_id.as_deref().unwrap_or(""));
    }
    for (eid, step) in &phase_zero.updated_steps {
        ref_ids.insert(eid.as_str(), step.ref_id.as_deref().unwrap_or(""));
    }

    // Build element_id → node name lookup from snapshot
    let node_names: HashMap<&str, &str> = snapshot
        .nodes
        .iter()
        .map(|n| (n.element_id.as_str(), node_name(&n.raw_text)))
        .collect();

    let mut sections = Vec::new();

    // Header
    sections.push(
        "The user submitted their canvas. Phase 0 has created the structural skeleton.\n\
         Dispatch configuration instructions to each node below using dispatch_to_builders."
            .to_string(),
    );

    // New nodes
    let new_nodes: Vec<_> = changeset
        .meaningful
        .iter()
        .filter_map(|c| match c {
            ScoredChange::NewNode { node, .. } => Some(node),
            _ => None,
        })
        .collect();

    if !new_nodes.is_empty() {
        let mut section = format!("<new_nodes count=\"{}\">", new_nodes.len());
        for node in &new_nodes {
            let ref_id = ref_ids
                .get(node.element_id.as_str())
                .copied()
                .unwrap_or("unknown");
            section.push_str(&format!("\n  <node ref_id=\"{}\">", ref_id));
            section.push_str(&format!("\n    User wrote: \"{}\"", node.raw_text));
            if !node.annotations.is_empty() {
                section.push_str(&format!(
                    "\n    Annotations: \"{}\"",
                    node.annotations.join("\", \"")
                ));
            }
            if let Some(encoding) = &node.stroke_encoding {
                section.push_str(&format!("\n    Stroke data: {}", encoding));
            } else if let Some(sketch) = &node.sketch {
                section.push_str(&format!("\n    Sketch:\n{}", sketch));
            }
            section.push_str("\n  </node>");
        }
        section.push_str("\n</new_nodes>");
        sections.push(section);
    }

    // Updated nodes
    let updated_nodes: Vec<_> = changeset
        .meaningful
        .iter()
        .filter_map(|c| match c {
            ScoredChange::UpdatedNode { update, .. } => Some(update),
            _ => None,
        })
        .collect();

    if !updated_nodes.is_empty() {
        let mut section = format!("<updated_nodes count=\"{}\">", updated_nodes.len());
        for update in &updated_nodes {
            let ref_id = ref_ids
                .get(update.element_id.as_str())
                .copied()
                .unwrap_or("unknown");
            section.push_str(&format!("\n  <node ref_id=\"{}\">", ref_id));
            section.push_str(&format!("\n    Before: \"{}\"", update.old_text));
            section.push_str(&format!("\n    After: \"{}\"", update.new_text));
            if update.old_annotations != update.new_annotations {
                if !update.new_annotations.is_empty() {
                    section.push_str(&format!(
                        "\n    New annotations: \"{}\"",
                        update.new_annotations.join("\", \"")
                    ));
                }
            }
            section.push_str("\n  </node>");
        }
        section.push_str("\n</updated_nodes>");
        sections.push(section);
    }

    // New edges
    let new_edges: Vec<_> = changeset
        .meaningful
        .iter()
        .filter_map(|c| match c {
            ScoredChange::NewEdge { edge, .. } => Some(edge),
            _ => None,
        })
        .collect();

    if !new_edges.is_empty() {
        let mut section = format!("<new_edges count=\"{}\">", new_edges.len());
        for edge in &new_edges {
            let source = ref_ids
                .get(edge.source_node_id.as_str())
                .copied()
                .or_else(|| node_names.get(edge.source_node_id.as_str()).copied())
                .unwrap_or("unknown");
            let target = ref_ids
                .get(edge.target_node_id.as_str())
                .copied()
                .or_else(|| node_names.get(edge.target_node_id.as_str()).copied())
                .unwrap_or("unknown");
            section.push_str(&format!("\n  {} -> {}", source, target));
        }
        section.push_str("\n</new_edges>");
        sections.push(section);
    }

    // Agentless summary
    let agentless = &changeset.agentless;
    let mut agentless_parts = Vec::new();
    if !agentless.deleted_node_ids.is_empty() {
        agentless_parts.push(format!(
            "{} deleted node(s)",
            agentless.deleted_node_ids.len()
        ));
    }
    if !agentless.deleted_edge_ids.is_empty() {
        agentless_parts.push(format!(
            "{} deleted edge(s)",
            agentless.deleted_edge_ids.len()
        ));
    }
    if !agentless.rewired_edges.is_empty() {
        agentless_parts.push(format!("{} rewired edge(s)", agentless.rewired_edges.len()));
    }
    if !agentless.moved_nodes.is_empty() {
        agentless_parts.push(format!("{} moved node(s)", agentless.moved_nodes.len()));
    }

    if !agentless_parts.is_empty() {
        sections.push(format!(
            "<structural_summary>\n  Phase 0 also handled: {}.\n</structural_summary>",
            agentless_parts.join(", ")
        ));
    }

    // Global notes
    let global_notes: Vec<&GlobalNote> = snapshot.global_notes.iter().collect();
    if !global_notes.is_empty() {
        let mut section = String::from("<global_notes>");
        for note in &global_notes {
            section.push_str(&format!("\n  \"{}\"", note.text));
        }
        section.push_str("\n</global_notes>");
        sections.push(section);
    }

    Some(sections.join("\n\n"))
}

/// Extract the node name (first non-empty line) from raw text.
fn node_name(raw_text: &str) -> &str {
    raw_text
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("Untitled")
        .trim()
}

#[cfg(test)]
#[path = "instruction_tests.rs"]
mod instruction_tests;
