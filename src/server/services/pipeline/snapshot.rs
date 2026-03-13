//! Build a human-readable snapshot of pipeline configuration.

use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use crate::db::traits::WorkflowRepo;
use crate::server::services::ServiceError;

use super::types::PipelineContext;

/// Build a human-readable snapshot of the pipeline configuration for
/// system prompt injection.
///
/// Shows step topology, execution ordering, and dependency graph.
pub async fn build_snapshot(
    repo: &dyn WorkflowRepo,
    ctx: &PipelineContext,
) -> Result<String, ServiceError> {
    let step = repo
        .get_step(ctx.parent_step_id)
        .await
        .map_err(ServiceError::Internal)?
        .ok_or_else(|| ServiceError::not_found("Parent step"))?;

    let pipeline_id = match step.child_workflow_id {
        Some(id) => id,
        None => return Ok(build_empty_snapshot(&step)),
    };

    let child_steps = repo
        .list_steps(pipeline_id)
        .await
        .map_err(ServiceError::Internal)?;

    let child_edges = repo
        .list_edges(pipeline_id)
        .await
        .map_err(ServiceError::Internal)?;

    let pipeline_steps: Vec<_> = child_steps.iter().collect();

    // Build step ID → name lookup
    let step_to_name: HashMap<Uuid, &str> = pipeline_steps
        .iter()
        .map(|s| (s.id, s.name.as_deref().unwrap_or("(unnamed)")))
        .collect();

    let step_ids: HashSet<&Uuid> = step_to_name.keys().collect();

    // Build dependency info
    let mut receives_from: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut dep_edges: Vec<(&str, &str)> = Vec::new();

    for edge in &child_edges {
        if step_ids.contains(&edge.from_step_id) && step_ids.contains(&edge.to_step_id) {
            if let (Some(&from_name), Some(&to_name)) = (
                step_to_name.get(&edge.from_step_id),
                step_to_name.get(&edge.to_step_id),
            ) {
                dep_edges.push((from_name, to_name));
                receives_from.entry(to_name).or_default().push(from_name);
            }
        }
    }

    let mut out = String::new();

    // Step info
    out.push_str(&format!(
        "Name: {}\n",
        step.name.as_deref().unwrap_or("(not set)")
    ));
    out.push_str(&format!(
        "Description: {}\n",
        if step.description.is_empty() {
            "(not set)"
        } else {
            &step.description
        }
    ));

    // Pipeline steps
    out.push_str("\nPipeline steps (execution order):\n");
    if pipeline_steps.is_empty() {
        out.push_str("  (none)\n");
    } else {
        let mut sorted_steps = pipeline_steps.clone();
        sorted_steps.sort_by_key(|s| s.display_order);

        for (i, ps) in sorted_steps.iter().enumerate() {
            let name = ps.name.as_deref().unwrap_or("(unnamed)");
            let routing = receives_from
                .get(name)
                .map(|froms| format!(" \u{2190} receives from: {}", froms.join(", ")))
                .unwrap_or_default();

            out.push_str(&format!(
                "  {}. {} (mode: {}){}{}\n",
                i + 1,
                name,
                ps.execution_mode,
                if ps.description.is_empty() {
                    String::new()
                } else {
                    format!(" \u{2014} {}", ps.description)
                },
                routing,
            ));
        }
    }

    // Dependencies
    out.push_str("\nDependencies:\n");
    if dep_edges.is_empty() {
        out.push_str("  (none)\n");
    } else {
        for (from, to) in &dep_edges {
            out.push_str(&format!("  {} \u{2192} {}\n", from, to));
        }
    }

    // Incoming context from parent workflow
    let parent_edges = repo
        .list_edges(ctx.parent_workflow_id)
        .await
        .map_err(ServiceError::Internal)?;

    let upstream_step_ids: Vec<Uuid> = parent_edges
        .iter()
        .filter(|e| e.to_step_id == ctx.parent_step_id)
        .map(|e| e.from_step_id)
        .collect();

    out.push_str("\nIncoming Context:\n");
    if upstream_step_ids.is_empty() {
        out.push_str("  (no connected sources)\n");
    } else {
        for upstream_id in upstream_step_ids {
            let upstream = match repo.get_step(upstream_id).await {
                Ok(Some(s)) => s,
                _ => continue,
            };
            let name = upstream
                .name
                .unwrap_or_else(|| format!("Step {}", upstream.id));
            out.push_str(&format!("  - {} ({})\n", name, upstream.execution_mode));
        }
    }

    Ok(out)
}

fn build_empty_snapshot(step: &crate::db::WorkflowStepRow) -> String {
    format!(
        "Name: {}\nDescription: {}\n\nPipeline steps:\n  (none)\n",
        step.name.as_deref().unwrap_or("(not set)"),
        if step.description.is_empty() {
            "(not set)"
        } else {
            &step.description
        },
    )
}
