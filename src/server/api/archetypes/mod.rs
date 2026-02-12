//! Archetype catalog endpoint.
//!
//! Returns the static list of available node archetypes. No DB required —
//! backed by a Rust const array.

use axum::Json;
use serde::Serialize;

mod tests;

/// Archetype metadata for the frontend catalog.
#[derive(Debug, Clone, Serialize)]
pub struct ArchetypeInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub icon: &'static str,
    pub color: &'static str,
}

const ARCHETYPES: &[ArchetypeInfo] = &[
    ArchetypeInfo {
        id: "documenter",
        name: "Documenter",
        description:
            "Research-and-write pipeline that produces structured documents from incoming context.",
        icon: "file-text",
        color: "#4A90D9",
    },
    ArchetypeInfo {
        id: "task_force",
        name: "Task Force",
        description:
            "Team of agents that executes a multi-step mission with planning and deliverables.",
        icon: "users",
        color: "#E67E22",
    },
    ArchetypeInfo {
        id: "belief_capture",
        name: "Belief Capture",
        description: "Context summarizer that extracts structured knowledge from upstream results.",
        icon: "lightbulb",
        color: "#9B59B6",
    },
    ArchetypeInfo {
        id: "room",
        name: "Room",
        description: "Meeting space where agents discuss, debate, or review a topic.",
        icon: "message-circle",
        color: "#2ECC71",
    },
];

/// GET /api/archetypes — returns the static archetype catalog.
pub async fn list_archetypes() -> Json<Vec<ArchetypeInfo>> {
    Json(ARCHETYPES.to_vec())
}
