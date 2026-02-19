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

const ARCHETYPES: &[ArchetypeInfo] = &[ArchetypeInfo {
    id: "workforce",
    name: "Workforce",
    description:
        "Team of agents that executes a mission with configurable deliverables and agent roles.",
    icon: "users",
    color: "#E67E22",
}];

/// GET /api/archetypes — returns the static archetype catalog.
pub async fn list_archetypes() -> Json<Vec<ArchetypeInfo>> {
    Json(ARCHETYPES.to_vec())
}
