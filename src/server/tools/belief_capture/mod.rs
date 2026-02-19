//! Tool execution handlers for the belief capture archetype.
//!
//! These tools operate on a specific belief capture workflow step, managing
//! extraction plans (focus, tag vocabulary, contradiction handling, confidence
//! thresholds). The chat strategy calls `execute_belief_capture_tool` directly
//! via dispatch.

use serde_json::{json, Value};
use uuid::Uuid;

use crate::db::traits::WorkflowRepo;
use crate::db::BeliefExtractionPlanRow;
use crate::server::tools::shared::{require_array, require_str};

mod tests;

/// Ambient context for belief capture tool execution.
pub struct BeliefCaptureToolContext {
    pub workflow_id: Uuid,
    pub step_id: Uuid,
}

/// Valid contradiction handling modes.
const VALID_CONTRADICTION_MODES: &[&str] = &["flag", "resolve", "keep_both"];

/// Valid confidence threshold values.
const VALID_CONFIDENCE_THRESHOLDS: &[&str] = &["low", "medium", "high"];

/// Execute a belief capture tool by name.
pub async fn execute_belief_capture_tool(
    name: &str,
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &BeliefCaptureToolContext,
) -> Value {
    match name {
        "set_extraction_focus" => execute_set_extraction_focus(input, repo, ctx).await,
        "set_tag_vocabulary" => execute_set_tag_vocabulary(input, repo, ctx).await,
        "set_contradiction_handling" => execute_set_contradiction_handling(input, repo, ctx).await,
        "set_confidence_threshold" => execute_set_confidence_threshold(input, repo, ctx).await,
        _ => json!({ "error": format!("Unknown belief capture tool: {}", name) }),
    }
}

/// Read-preserve-write helper for extraction plan fields.
///
/// Loads the existing plan (if any), merges in provided overrides, and upserts.
/// Fields set to `None` keep their existing (or default) values.
async fn upsert_extraction_plan_field(
    repo: &dyn WorkflowRepo,
    step_id: Uuid,
    extraction_focus: Option<&str>,
    tag_vocabulary: Option<&[String]>,
    contradiction_handling: Option<&str>,
    confidence_threshold: Option<&str>,
) -> Result<BeliefExtractionPlanRow, String> {
    let existing = repo.get_extraction_plan(step_id).await.ok().flatten();

    let focus = extraction_focus.map(String::from).unwrap_or_else(|| {
        existing
            .as_ref()
            .map_or(String::new(), |p| p.extraction_focus.clone())
    });
    let tags = tag_vocabulary.map(|t| t.to_vec()).unwrap_or_else(|| {
        existing
            .as_ref()
            .map_or(vec![], |p| p.tag_vocabulary.clone())
    });
    let contra = contradiction_handling.map(String::from).unwrap_or_else(|| {
        existing
            .as_ref()
            .map_or("flag".to_string(), |p| p.contradiction_handling.clone())
    });
    let conf = confidence_threshold.map(String::from).unwrap_or_else(|| {
        existing
            .as_ref()
            .map_or("low".to_string(), |p| p.confidence_threshold.clone())
    });

    repo.upsert_extraction_plan(step_id, &focus, &tags, &contra, &conf)
        .await
        .map_err(|e| e.to_string())
}

async fn execute_set_extraction_focus(
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &BeliefCaptureToolContext,
) -> Value {
    let guidance = match require_str(input, "guidance") {
        Ok(v) => v,
        Err(e) => return e,
    };

    match upsert_extraction_plan_field(repo, ctx.step_id, Some(guidance), None, None, None).await {
        Ok(plan) => json!({
            "step_id": ctx.step_id.to_string(),
            "extraction_focus": plan.extraction_focus,
        }),
        Err(e) => json!({ "error": e }),
    }
}

async fn execute_set_tag_vocabulary(
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &BeliefCaptureToolContext,
) -> Value {
    let tags_arr = match require_array(input, "tags") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let tags: Vec<String> = tags_arr
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();

    match upsert_extraction_plan_field(repo, ctx.step_id, None, Some(&tags), None, None).await {
        Ok(plan) => json!({
            "tag_vocabulary": plan.tag_vocabulary,
        }),
        Err(e) => json!({ "error": e }),
    }
}

async fn execute_set_contradiction_handling(
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &BeliefCaptureToolContext,
) -> Value {
    let mode = match require_str(input, "mode") {
        Ok(v) => v,
        Err(e) => return e,
    };

    if !VALID_CONTRADICTION_MODES.contains(&mode) {
        return json!({
            "error": format!(
                "Invalid contradiction handling mode '{}'. Must be one of: {}",
                mode,
                VALID_CONTRADICTION_MODES.join(", ")
            )
        });
    }

    match upsert_extraction_plan_field(repo, ctx.step_id, None, None, Some(mode), None).await {
        Ok(plan) => json!({
            "contradiction_handling": plan.contradiction_handling,
        }),
        Err(e) => json!({ "error": e }),
    }
}

async fn execute_set_confidence_threshold(
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &BeliefCaptureToolContext,
) -> Value {
    let threshold = match require_str(input, "threshold") {
        Ok(v) => v,
        Err(e) => return e,
    };

    if !VALID_CONFIDENCE_THRESHOLDS.contains(&threshold) {
        return json!({
            "error": format!(
                "Invalid confidence threshold '{}'. Must be one of: {}",
                threshold,
                VALID_CONFIDENCE_THRESHOLDS.join(", ")
            )
        });
    }

    match upsert_extraction_plan_field(repo, ctx.step_id, None, None, None, Some(threshold)).await {
        Ok(plan) => json!({
            "confidence_threshold": plan.confidence_threshold,
        }),
        Err(e) => json!({ "error": e }),
    }
}

// =========================================================================
// Context Building (public helper for system prompt injection)
// =========================================================================

/// Build the config snapshot string for `{{.System.current_config}}` injection.
///
/// Called by the hub each turn to provide the assistant with the live state
/// of the belief capture step.
pub async fn build_config_snapshot(
    repo: &dyn WorkflowRepo,
    ctx: &BeliefCaptureToolContext,
) -> Result<String, String> {
    // Load step
    let step = repo
        .get_step(ctx.step_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Step not found".to_string())?;

    // Load extraction plan
    let plan = repo
        .get_extraction_plan(ctx.step_id)
        .await
        .map_err(|e| e.to_string())?;

    // Load edges to find upstream steps
    let edges = repo
        .list_edges(ctx.workflow_id)
        .await
        .map_err(|e| e.to_string())?;

    let upstream_step_ids: Vec<Uuid> = edges
        .iter()
        .filter(|e| e.to_step_id == ctx.step_id)
        .map(|e| e.from_step_id)
        .collect();

    let mut out = String::new();

    // Step config
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

    // Extraction plan
    if let Some(ref plan) = plan {
        out.push_str(&format!(
            "\nExtraction Focus: {}\n",
            if plan.extraction_focus.is_empty() {
                "(not set)"
            } else {
                &plan.extraction_focus
            }
        ));

        if plan.tag_vocabulary.is_empty() {
            out.push_str("Tag Vocabulary: (not set)\n");
        } else {
            out.push_str(&format!(
                "Tag Vocabulary: {}\n",
                plan.tag_vocabulary.join(", ")
            ));
        }

        out.push_str(&format!(
            "Contradiction Handling: {}\n",
            plan.contradiction_handling
        ));
        out.push_str(&format!(
            "Confidence Threshold: {}\n",
            plan.confidence_threshold
        ));
    } else {
        out.push_str("\nExtraction Focus: (not set)\n");
        out.push_str("Tag Vocabulary: (not set)\n");
        out.push_str("Contradiction Handling: flag\n");
        out.push_str("Confidence Threshold: low\n");
    }

    // Incoming context
    out.push_str("\nIncoming Context:\n");
    if upstream_step_ids.is_empty() {
        out.push_str("  (no connected sources)\n");
    } else {
        for upstream_id in upstream_step_ids {
            let upstream = match repo.get_step(upstream_id).await {
                Ok(Some(s)) => s,
                _ => continue,
            };

            let (status, preview, word_count) =
                crate::server::tools::shared::classify_content_status(&upstream);
            let name = upstream
                .name
                .unwrap_or_else(|| format!("Step {}", upstream.id));

            out.push_str(&format!(
                "  - {} ({}) — {}\n",
                name, upstream.execution_mode, status
            ));
            if !upstream.description.is_empty() {
                out.push_str(&format!("    Description: {}\n", upstream.description));
            }
            if let Some(preview) = preview {
                out.push_str(&format!(
                    "    Preview ({} words): {}\n",
                    word_count.unwrap_or(0),
                    preview
                ));
            }
        }
    }

    Ok(out)
}
