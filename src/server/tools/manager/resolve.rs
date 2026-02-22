//! Universal node resolver: accepts both ref IDs ("workforce-1") and
//! human-readable names ("Collector") for node identification.

use crate::db::traits::WorkflowRepo;
use crate::db::WorkflowStepRow;
use uuid::Uuid;

/// Resolve a node identifier to a `WorkflowStepRow`.
///
/// Resolution order:
/// 1. Try exact ref_id match (e.g. "workforce-1")
/// 2. Try case-insensitive name match
/// 3. Error if ambiguous (multiple name matches) or not found
pub async fn resolve_node(
    repo: &dyn WorkflowRepo,
    workflow_id: Uuid,
    identifier: &str,
) -> Result<WorkflowStepRow, String> {
    // 1. Try ref_id lookup first (fast, exact match)
    match repo.find_step_by_ref_id(workflow_id, identifier).await {
        Ok(Some(step)) => return Ok(step),
        Ok(None) => {}
        Err(e) => return Err(format!("DB error resolving ref \"{identifier}\": {e}")),
    }

    // 2. Fall back to case-insensitive name match
    let steps = repo
        .list_steps(workflow_id)
        .await
        .map_err(|e| format!("DB error listing steps: {e}"))?;

    let identifier_lower = identifier.to_lowercase();
    let matches: Vec<&WorkflowStepRow> = steps
        .iter()
        .filter(|s| {
            s.name
                .as_deref()
                .is_some_and(|n| n.to_lowercase() == identifier_lower)
        })
        .collect();

    match matches.len() {
        1 => Ok(matches[0].clone()),
        0 => Err(format!("Node \"{identifier}\" not found in this workflow")),
        _ => {
            let refs: Vec<String> = matches
                .iter()
                .filter_map(|s| s.ref_id.as_deref())
                .map(|r| format!("\"{r}\""))
                .collect();
            Err(format!(
                "Multiple nodes named \"{identifier}\". Use a ref ID to disambiguate: {}",
                refs.join(", ")
            ))
        }
    }
}

/// Check that a node name is unique within a workflow.
///
/// Returns `Ok(())` if no existing step has the same name (case-insensitive),
/// or `Err` with a descriptive message if a duplicate exists.
pub fn check_name_unique(existing_steps: &[WorkflowStepRow], name: &str) -> Result<(), String> {
    let name_lower = name.to_lowercase();
    if let Some(existing) = existing_steps.iter().find(|s| {
        s.name
            .as_deref()
            .is_some_and(|n| n.to_lowercase() == name_lower)
    }) {
        let ref_hint = existing
            .ref_id
            .as_deref()
            .map(|r| format!(" (ref: {r})"))
            .unwrap_or_default();
        Err(format!(
            "A node named \"{name}\" already exists{ref_hint}. Use a unique name."
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests;
