//! Tool execution handlers for the room archetype.
//!
//! These tools operate on a specific room workflow step, managing
//! meeting configuration (purpose, members, turns, interaction mode).
//! The chat strategy calls `execute_room_config_tool` directly via dispatch.

use serde_json::{json, Value};
use uuid::Uuid;

use crate::db::traits::WorkflowRepo;

mod tests;

/// Ambient context for room config tool execution.
pub struct RoomConfigToolContext {
    pub workflow_id: Uuid,
    pub step_id: Uuid,
}

/// Valid interaction modes.
const VALID_INTERACTION_MODES: &[&str] = &["round_robin", "moderated", "open_floor"];

/// Execute a room config tool by name.
pub async fn execute_room_config_tool(
    name: &str,
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &RoomConfigToolContext,
) -> Value {
    match name {
        "set_meeting_purpose" => execute_set_meeting_purpose(input, repo, ctx).await,
        "add_member" => execute_add_member(input, repo, ctx).await,
        "update_member" => execute_update_member(input, repo, ctx).await,
        "remove_member" => execute_remove_member(input, repo, ctx).await,
        "set_max_turns" => execute_set_max_turns(input, repo, ctx).await,
        "set_interaction_mode" => execute_set_interaction_mode(input, repo, ctx).await,
        _ => json!({ "error": format!("Unknown room config tool: {}", name) }),
    }
}

async fn execute_set_meeting_purpose(
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &RoomConfigToolContext,
) -> Value {
    let Some(description) = input["description"].as_str() else {
        return json!({ "error": "Missing required parameter: description" });
    };

    // Load existing config to preserve other fields, or use defaults
    let existing = repo.get_room_step_config(ctx.step_id).await.ok().flatten();

    let (max_turns, interaction_mode, gatekeeper_enabled) = match &existing {
        Some(config) => (
            config.max_turns,
            config.interaction_mode.clone(),
            config.gatekeeper_enabled,
        ),
        None => (20, "moderated".to_string(), true),
    };

    match repo
        .upsert_room_step_config(
            ctx.step_id,
            description,
            max_turns,
            &interaction_mode,
            gatekeeper_enabled,
        )
        .await
    {
        Ok(config) => json!({
            "step_id": ctx.step_id.to_string(),
            "meeting_purpose": config.meeting_purpose,
        }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

async fn execute_add_member(
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &RoomConfigToolContext,
) -> Value {
    let Some(name) = input["name"].as_str() else {
        return json!({ "error": "Missing required parameter: name" });
    };
    let Some(role) = input["role"].as_str() else {
        return json!({ "error": "Missing required parameter: role" });
    };
    let perspective = input["perspective"].as_str().unwrap_or("");

    // Auto-assign display_order based on existing member count
    let display_order = match repo.list_room_step_members(ctx.step_id).await {
        Ok(members) => members.len() as i32,
        Err(_) => 0,
    };

    match repo
        .add_room_step_member(ctx.step_id, name, role, perspective, display_order)
        .await
    {
        Ok(member) => json!({
            "id": member.id.to_string(),
            "name": member.name,
            "role": member.role,
            "perspective": member.perspective,
            "display_order": member.display_order,
        }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

async fn execute_update_member(
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &RoomConfigToolContext,
) -> Value {
    let Some(name) = input["name"].as_str() else {
        return json!({ "error": "Missing required parameter: name" });
    };

    // Find member by name (case-insensitive)
    let members = match repo.list_room_step_members(ctx.step_id).await {
        Ok(m) => m,
        Err(e) => return json!({ "error": format!("Failed to list members: {}", e) }),
    };

    let name_lower = name.to_lowercase();
    let member = members.iter().find(|m| m.name.to_lowercase() == name_lower);
    let Some(member) = member else {
        return json!({ "error": format!("Member '{}' not found", name) });
    };

    let new_role = input["role"].as_str().map(String::from);
    let new_perspective = input["perspective"].as_str().map(String::from);

    match repo
        .update_room_step_member(member.id, None, new_role, new_perspective)
        .await
    {
        Ok(updated) => json!({
            "id": updated.id.to_string(),
            "name": updated.name,
            "role": updated.role,
            "perspective": updated.perspective,
        }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

async fn execute_remove_member(
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &RoomConfigToolContext,
) -> Value {
    let Some(name) = input["name"].as_str() else {
        return json!({ "error": "Missing required parameter: name" });
    };

    // Find member by name (case-insensitive)
    let members = match repo.list_room_step_members(ctx.step_id).await {
        Ok(m) => m,
        Err(e) => return json!({ "error": format!("Failed to list members: {}", e) }),
    };

    let name_lower = name.to_lowercase();
    let member = members.iter().find(|m| m.name.to_lowercase() == name_lower);
    let Some(member) = member else {
        return json!({ "error": format!("Member '{}' not found", name) });
    };

    match repo.remove_room_step_member(member.id).await {
        Ok(()) => json!({
            "removed": name,
        }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

async fn execute_set_max_turns(
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &RoomConfigToolContext,
) -> Value {
    let Some(count) = input["count"].as_i64() else {
        return json!({ "error": "Missing required parameter: count" });
    };

    if !(1..=100).contains(&count) {
        return json!({ "error": "max_turns must be between 1 and 100" });
    }

    // Load existing config to preserve other fields
    let existing = repo.get_room_step_config(ctx.step_id).await.ok().flatten();

    let (meeting_purpose, interaction_mode, gatekeeper_enabled) = match &existing {
        Some(config) => (
            config.meeting_purpose.clone(),
            config.interaction_mode.clone(),
            config.gatekeeper_enabled,
        ),
        None => (String::new(), "moderated".to_string(), true),
    };

    match repo
        .upsert_room_step_config(
            ctx.step_id,
            &meeting_purpose,
            count as i32,
            &interaction_mode,
            gatekeeper_enabled,
        )
        .await
    {
        Ok(config) => json!({
            "max_turns": config.max_turns,
        }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

async fn execute_set_interaction_mode(
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &RoomConfigToolContext,
) -> Value {
    let Some(mode) = input["mode"].as_str() else {
        return json!({ "error": "Missing required parameter: mode" });
    };

    if !VALID_INTERACTION_MODES.contains(&mode) {
        return json!({
            "error": format!(
                "Invalid interaction mode '{}'. Must be one of: {}",
                mode,
                VALID_INTERACTION_MODES.join(", ")
            )
        });
    }

    // Derive gatekeeper_enabled from mode
    let gatekeeper_enabled = mode != "round_robin";

    // Load existing config to preserve other fields
    let existing = repo.get_room_step_config(ctx.step_id).await.ok().flatten();

    let (meeting_purpose, max_turns) = match &existing {
        Some(config) => (config.meeting_purpose.clone(), config.max_turns),
        None => (String::new(), 20),
    };

    match repo
        .upsert_room_step_config(
            ctx.step_id,
            &meeting_purpose,
            max_turns,
            mode,
            gatekeeper_enabled,
        )
        .await
    {
        Ok(config) => json!({
            "interaction_mode": config.interaction_mode,
            "gatekeeper_enabled": config.gatekeeper_enabled,
        }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

// =========================================================================
// Context Building (public helper for system prompt injection)
// =========================================================================

/// Build the config snapshot string for `{{.System.current_config}}` injection.
///
/// Called by the hub each turn to provide the assistant with the live state
/// of the room step.
pub async fn build_config_snapshot(
    repo: &dyn WorkflowRepo,
    ctx: &RoomConfigToolContext,
) -> Result<String, String> {
    // Load step
    let step = repo
        .get_step(ctx.step_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Step not found".to_string())?;

    // Load room config
    let config = repo
        .get_room_step_config(ctx.step_id)
        .await
        .map_err(|e| e.to_string())?;

    // Load members
    let members = repo
        .list_room_step_members(ctx.step_id)
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

    // Room config
    if let Some(ref config) = config {
        out.push_str(&format!(
            "\nMeeting Purpose: {}\n",
            if config.meeting_purpose.is_empty() {
                "(not set)"
            } else {
                &config.meeting_purpose
            }
        ));
        out.push_str(&format!("Max Turns: {}\n", config.max_turns));
        out.push_str(&format!("Interaction Mode: {}\n", config.interaction_mode));
        out.push_str(&format!(
            "Gatekeeper: {}\n",
            if config.gatekeeper_enabled {
                "enabled"
            } else {
                "disabled"
            }
        ));
    } else {
        out.push_str("\nMeeting Purpose: (not set)\n");
        out.push_str("Max Turns: 20\n");
        out.push_str("Interaction Mode: moderated\n");
        out.push_str("Gatekeeper: enabled\n");
    }

    // Members
    out.push_str("\nMembers:\n");
    if members.is_empty() {
        out.push_str("  (none)\n");
    } else {
        for member in &members {
            if member.perspective.is_empty() {
                out.push_str(&format!(
                    "  - {} (id: {}) — {}\n",
                    member.name, member.id, member.role,
                ));
            } else {
                out.push_str(&format!(
                    "  - {} (id: {}) — {} [{}]\n",
                    member.name, member.id, member.role, member.perspective,
                ));
            }
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
