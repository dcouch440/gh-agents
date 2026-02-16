//! Tool execution handlers for the documenter assistant.
//!
//! These tools operate on a specific documenter workflow step, managing
//! document definitions and step configuration. They are NOT wired into
//! the generic `execute_tool` dispatch — the chat strategy calls
//! `execute_documenter_tool` directly.
//!
//! Context (current config, incoming sources) is injected into the system
//! prompt via `{{.System.current_config}}` — see [`build_config_snapshot`]
//! for the formatting logic used by the strategy in Phase 2.

use chrono::Utc;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::db::traits::WorkflowRepo;

#[cfg(test)]
mod tests;

/// Ambient context for documenter tool execution.
///
/// Provides the workflow and step IDs so individual tools don't need
/// them in their input schemas.
pub struct DocumenterToolContext {
    pub workflow_id: Uuid,
    pub step_id: Uuid,
}

/// Execute a documenter-scoped tool by name.
///
/// Returns a JSON value describing the result. Unknown tool names
/// return an error object.
pub async fn execute_documenter_tool(
    name: &str,
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &DocumenterToolContext,
) -> Value {
    match name {
        "create_doc_def" => execute_create_doc_def(input, repo, ctx).await,
        "update_doc_def" => execute_update_doc_def(input, repo, ctx).await,
        "delete_doc_def" => execute_delete_doc_def(input, repo, ctx).await,
        "update_config" => execute_update_config(input, repo, ctx).await,
        _ => json!({ "error": format!("Unknown documenter tool: {}", name) }),
    }
}

// =========================================================================
// Tool Handlers
// =========================================================================

/// Create a new document definition on the documenter step.
async fn execute_create_doc_def(
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &DocumenterToolContext,
) -> Value {
    let Some(name) = input["name"].as_str() else {
        return json!({ "error": "Missing required parameter: name" });
    };

    let description = input["description"].as_str().unwrap_or("").to_string();
    let target_length = input["target_length"].as_i64().unwrap_or(1500) as i32;

    let def = crate::db::ProtocolDocumentDefRow {
        id: Uuid::new_v4(),
        step_id: Some(ctx.step_id),
        name: name.to_string(),
        description,
        target_length,
        display_order: 0,
        created_at: Utc::now(),
        protocol_id: None,
        document_id: None,
        agent_roster_entry_id: None,
    };

    match repo.create_document_def(def).await {
        Ok(row) => json!({
            "id": row.id.to_string(),
            "name": row.name,
            "description": row.description,
            "target_length": row.target_length,
        }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

/// Update an existing document definition.
///
/// Supports partial updates: loads the existing def first, then merges
/// only the fields provided in the input.
async fn execute_update_doc_def(
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &DocumenterToolContext,
) -> Value {
    let Some(id_str) = input["doc_def_id"].as_str() else {
        return json!({ "error": "Missing required parameter: doc_def_id" });
    };
    let Ok(def_id) = Uuid::parse_str(id_str) else {
        return json!({ "error": format!("Invalid UUID: {}", id_str) });
    };

    // Load existing defs to find the one being updated (for merge)
    let existing_defs = match repo.list_document_defs(ctx.step_id).await {
        Ok(defs) => defs,
        Err(e) => return json!({ "error": e.to_string() }),
    };

    let existing = match existing_defs.into_iter().find(|d| d.id == def_id) {
        Some(d) => d,
        None => return json!({ "error": "Document definition not found" }),
    };

    let name = input["name"]
        .as_str()
        .map(String::from)
        .unwrap_or(existing.name);
    let description = input["description"]
        .as_str()
        .map(String::from)
        .unwrap_or(existing.description);
    let target_length = input["target_length"]
        .as_i64()
        .map(|v| v as i32)
        .unwrap_or(existing.target_length);

    match repo
        .update_document_def(def_id, name, description, target_length)
        .await
    {
        Ok(row) => json!({
            "id": row.id.to_string(),
            "name": row.name,
            "description": row.description,
            "target_length": row.target_length,
        }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

/// Delete a document definition.
///
/// Looks up the def name before deleting so the result can be used by the
/// consistency scanner to detect stale references in other nodes' notes.
async fn execute_delete_doc_def(
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &DocumenterToolContext,
) -> Value {
    let Some(id_str) = input["doc_def_id"].as_str() else {
        return json!({ "error": "Missing required parameter: doc_def_id" });
    };
    let Ok(def_id) = Uuid::parse_str(id_str) else {
        return json!({ "error": format!("Invalid UUID: {}", id_str) });
    };

    // Load the def name before deleting (needed for consistency scanning)
    let def_name = repo
        .list_document_defs(ctx.step_id)
        .await
        .ok()
        .and_then(|defs| defs.into_iter().find(|d| d.id == def_id))
        .map(|d| d.name)
        .unwrap_or_default();

    match repo.delete_document_def(def_id).await {
        Ok(()) => json!({
            "deleted": true,
            "name": def_name,
            "id": def_id.to_string(),
        }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

/// Update the documenter step's configuration (name, description, prompt).
///
/// All fields are optional — only provided fields are changed.
async fn execute_update_config(
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &DocumenterToolContext,
) -> Value {
    let has_name = input["name"].is_string();
    let has_description = input["description"].is_string();
    let has_prompt = input["prompt_template"].is_string();

    if !has_name && !has_description && !has_prompt {
        return json!({ "error": "At least one field (name, description, prompt_template) must be provided" });
    }

    // Load existing step, merge provided fields, save
    let mut step = match repo.get_step(ctx.step_id).await {
        Ok(Some(s)) => s,
        Ok(None) => return json!({ "error": "Step not found" }),
        Err(e) => return json!({ "error": format!("Failed to load step: {}", e) }),
    };

    if let Some(name) = input["name"].as_str() {
        step.name = Some(name.to_string());
    }
    if let Some(description) = input["description"].as_str() {
        step.description = description.to_string();
    }
    if let Some(prompt_template) = input["prompt_template"].as_str() {
        step.prompt_template = prompt_template.to_string();
    }

    match repo.update_step(step.clone()).await {
        Ok(_) => json!({
            "updated": true,
            "name": step.name,
            "description": step.description,
            "prompt_template": step.prompt_template,
        }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

// =========================================================================
// Context Building (public helpers for Phase 2 strategy)
// =========================================================================

// Re-export from shared module for backward compatibility.
pub use super::shared::classify_content_status;

/// Build the config snapshot string for `{{.System.current_config}}` injection.
///
/// Called by the chat strategy each turn to provide the assistant with
/// live state of the documenter step.
pub async fn build_config_snapshot(
    repo: &dyn WorkflowRepo,
    ctx: &DocumenterToolContext,
) -> Result<String, String> {
    // Load step
    let step = repo
        .get_step(ctx.step_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Step not found".to_string())?;

    // Load document definitions
    let doc_defs = repo
        .list_document_defs(ctx.step_id)
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

    // Build output
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
    out.push_str(&format!(
        "Prompt: {}\n",
        if step.prompt_template.is_empty() {
            "(not set)"
        } else {
            &step.prompt_template
        }
    ));

    // Document definitions
    out.push_str("\nDocuments:\n");
    if doc_defs.is_empty() {
        out.push_str("  (none)\n");
    } else {
        for def in &doc_defs {
            out.push_str(&format!(
                "  - {} (id: {}, target: ~{} words){}\n",
                def.name,
                def.id,
                def.target_length,
                if def.description.is_empty() {
                    String::new()
                } else {
                    format!(" — {}", def.description)
                }
            ));
        }
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

            let (status, preview, word_count) = classify_content_status(&upstream);
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
