//! Workflow agent service — bidirectional projection between DB board state
//! and a file-based repo (`topology.json` + `nodes/*.md`).
//!
//! Mirrors `system_node` one level up: where system_node projects agents within
//! a single node, this module projects nodes across the entire workflow.
//!
//! Used by:
//! - DB → repo projection (before agent turn)
//! - Repo → DB sync (after agent writes)
//! - `<current_state>` XML builder (every turn)
//! - File validation (write-time)

pub mod file_reader;
pub mod project;
pub mod state;
pub mod sync;
pub mod validate;
pub mod versions;

use std::path::PathBuf;

use uuid::Uuid;

use crate::server::state::AppState;

// ── Base directory resolution ──────────────────────────────────────────────

/// Resolve the base_dir for the workflow agent's board repo.
///
/// With JuiceFS: `{mount}/workflows/{wf_id}/board/`
/// Without JuiceFS: `{tmp}/nexor_workflow_agent/{wf_id}/`
pub fn resolve_base_dir(state: &AppState, workflow_id: Uuid) -> PathBuf {
    if let Some(workspace) = state.workspace() {
        let path = workspace.board_path(workflow_id);
        let _ = std::fs::create_dir_all(&path);
        return path;
    }

    let path = std::env::temp_dir()
        .join("nexor_workflow_agent")
        .join(workflow_id.to_string());
    let _ = std::fs::create_dir_all(&path);
    path
}

// ── Slug utilities ─────────────────────────────────────────────────────────

/// Convert a human-readable name to a filesystem-safe slug.
///
/// "Market Research" → "market_research"
/// "Fact Checker" → "fact_checker"
/// "Data-Pipeline" → "data_pipeline"
///
/// Uses lowercase + underscore separators (unlike `system_node::normalize_agent_name`
/// which strips all separators). Slugs are used as topology.json keys and node filenames.
pub fn name_to_slug(name: &str) -> String {
    let mut slug = String::with_capacity(name.len());
    let mut prev_separator = false;

    for ch in name.chars() {
        if ch.is_alphanumeric() {
            if prev_separator && !slug.is_empty() {
                slug.push('_');
            }
            slug.extend(ch.to_lowercase());
            prev_separator = false;
        } else {
            // spaces, hyphens, underscores, etc. → collapse into single underscore
            prev_separator = true;
        }
    }

    // Ensure slug starts with a letter
    if slug.starts_with(|c: char| c.is_ascii_digit()) {
        slug.insert(0, 'n');
    }

    if slug.is_empty() {
        "unnamed".to_string()
    } else {
        slug
    }
}

/// Generate the next unnamed slug: unnamed_01, unnamed_02, etc.
///
/// Scans existing slugs for the highest `unnamed_NN` and increments.
pub fn next_unnamed_slug(existing: &[&str]) -> String {
    let max_num = existing
        .iter()
        .filter_map(|s| s.strip_prefix("unnamed_"))
        .filter_map(|n| n.parse::<u32>().ok())
        .max()
        .unwrap_or(0);

    format!("unnamed_{:02}", max_num + 1)
}

/// Convert a slug back to a human-readable display name.
///
/// "market_research" → "Market Research"
/// "fact_checker" → "Fact Checker"
/// "unnamed_01" → "Unnamed 01"
pub fn slug_to_display_name(slug: &str) -> String {
    slug.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    let upper: String = first.to_uppercase().collect();
                    upper + chars.as_str()
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_to_slug_basic() {
        assert_eq!(name_to_slug("Market Research"), "market_research");
        assert_eq!(name_to_slug("Fact Checker"), "fact_checker");
        assert_eq!(name_to_slug("Data-Pipeline"), "data_pipeline");
        assert_eq!(name_to_slug("simple"), "simple");
    }

    #[test]
    fn name_to_slug_edge_cases() {
        assert_eq!(name_to_slug("  Leading Spaces  "), "leading_spaces");
        assert_eq!(name_to_slug("Multiple   Spaces"), "multiple_spaces");
        assert_eq!(name_to_slug("123_numeric"), "n123_numeric");
        assert_eq!(name_to_slug(""), "unnamed");
        assert_eq!(name_to_slug("---"), "unnamed");
        assert_eq!(name_to_slug("CamelCase"), "camelcase");
    }

    #[test]
    fn next_unnamed_slug_empty() {
        assert_eq!(next_unnamed_slug(&[]), "unnamed_01");
    }

    #[test]
    fn next_unnamed_slug_increments() {
        assert_eq!(
            next_unnamed_slug(&["unnamed_01", "unnamed_03", "research"]),
            "unnamed_04"
        );
    }

    #[test]
    fn slug_to_display_name_basic() {
        assert_eq!(slug_to_display_name("market_research"), "Market Research");
        assert_eq!(slug_to_display_name("fact_checker"), "Fact Checker");
        assert_eq!(slug_to_display_name("simple"), "Simple");
        assert_eq!(slug_to_display_name("unnamed_01"), "Unnamed 01");
    }
}
