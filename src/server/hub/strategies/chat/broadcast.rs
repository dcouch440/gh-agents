//! Workflow event broadcasting for step tool mutations.

use serde_json::Value;
use uuid::Uuid;

use crate::server::hub::consistency_scanner::{self, DeletedItem, DeletedItemType};
use crate::server::state::AppState;
use crate::server::ws::events::{WorkflowEvent, WorkflowEventKind};
use crate::types::UserId;

use super::config::StepChatContext;

/// The effect a tool call should produce on the workflow event bus.
enum ToolEffect {
    ArchetypeChanged,
    NameUpdated,
    DescriptionUpdated,
    DocDefCreated,
    DocDefUpdated,
    DocDefDeleted,
    ConfigUpdated,
    RosterChanged,
    MembersChanged,
    NotesUpdated,
}

impl ToolEffect {
    /// Parse a tool name into its broadcast effect, if any.
    fn from_tool_name(name: &str) -> Option<Self> {
        match name {
            "set_node_archetype" => Some(Self::ArchetypeChanged),
            "set_node_name" => Some(Self::NameUpdated),
            "set_node_description" => Some(Self::DescriptionUpdated),

            "create_doc_def" => Some(Self::DocDefCreated),
            "update_doc_def" => Some(Self::DocDefUpdated),
            "delete_doc_def" => Some(Self::DocDefDeleted),

            "update_config"
            | "set_task"
            | "set_capabilities"
            | "set_failure_mode"
            | "set_extraction_focus"
            | "set_tag_vocabulary"
            | "set_contradiction_handling"
            | "set_confidence_threshold"
            | "set_meeting_purpose"
            | "set_max_turns"
            | "set_interaction_mode" => Some(Self::ConfigUpdated),

            "add_agent" | "update_agent" | "remove_agent" => Some(Self::RosterChanged),
            "add_member" | "update_member" | "remove_member" => Some(Self::MembersChanged),

            "update_notes" => Some(Self::NotesUpdated),

            _ => None,
        }
    }

    /// Build the concrete workflow event kind from the tool effect and
    /// the step/input/result context.
    fn into_event_kind(self, step_id: Uuid, input: &Value, result: &Value) -> WorkflowEventKind {
        match self {
            Self::ArchetypeChanged => {
                let archetype = result["archetype"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string();
                WorkflowEventKind::ArchetypeChanged { step_id, archetype }
            }
            Self::NameUpdated => {
                let name = result["name"].as_str().unwrap_or("").to_string();
                WorkflowEventKind::StepNameUpdated { step_id, name }
            }
            Self::DescriptionUpdated | Self::ConfigUpdated => {
                WorkflowEventKind::StepConfigUpdated { step_id }
            }
            Self::DocDefCreated => {
                let doc_def_id = parse_uuid_field(result, "id").unwrap_or_else(Uuid::new_v4);
                let name = result["name"].as_str().unwrap_or("Untitled").to_string();
                WorkflowEventKind::DocDefCreated {
                    step_id,
                    doc_def_id,
                    name,
                }
            }
            Self::DocDefUpdated => {
                let doc_def_id = parse_uuid_field(result, "id")
                    .or_else(|| parse_uuid_field(input, "doc_def_id"));
                let doc_def_id = doc_def_id.unwrap_or_else(Uuid::new_v4);
                let name = result["name"].as_str().unwrap_or("Untitled").to_string();
                WorkflowEventKind::DocDefUpdated {
                    step_id,
                    doc_def_id,
                    name,
                }
            }
            Self::DocDefDeleted => {
                let doc_def_id = parse_uuid_field(input, "doc_def_id").unwrap_or_else(Uuid::new_v4);
                WorkflowEventKind::DocDefDeleted {
                    step_id,
                    doc_def_id,
                }
            }
            Self::RosterChanged => WorkflowEventKind::RosterChanged { step_id },
            Self::MembersChanged => WorkflowEventKind::RoomMembersChanged { step_id },
            Self::NotesUpdated => {
                let content = input["content"].as_str().unwrap_or("").to_string();
                WorkflowEventKind::AssistantNotesUpdated { step_id, content }
            }
        }
    }
}

/// Extract a UUID from a JSON object field.
fn parse_uuid_field(value: &Value, field: &str) -> Option<Uuid> {
    value[field].as_str().and_then(|s| Uuid::parse_str(s).ok())
}

/// Broadcast a workflow event when a step tool mutates data.
///
/// Handles both universal tools (archetype, name, description) and
/// archetype-specific tools (doc defs, config). Only emits if the
/// step context is present and the tool result indicates success.
pub(super) fn broadcast_step_event(
    state: &AppState,
    step_context: Option<&StepChatContext>,
    user_id: UserId,
    name: &str,
    input: &Value,
    result: &Value,
) {
    let Some(ctx) = step_context else {
        return;
    };

    if result.get("error").is_some() {
        return;
    }

    let Some(effect) = ToolEffect::from_tool_name(name) else {
        return;
    };

    let kind = effect.into_event_kind(ctx.step_id, input, result);

    state.broadcast_workflow(WorkflowEvent {
        run_id: None,
        workflow_id: ctx.workflow_id,
        user_id: Some(user_id.0),
        kind,
    });

    // Schedule consistency scan for deletion events
    schedule_consistency_scan_if_deletion(state, ctx, name, input, result);
}

/// If the tool was a deletion (doc def or roster agent), schedule a
/// debounced consistency scan to detect stale references in other notes.
fn schedule_consistency_scan_if_deletion(
    state: &AppState,
    ctx: &StepChatContext,
    tool_name: &str,
    input: &Value,
    result: &Value,
) {
    let (item_type, id_field) = match tool_name {
        "delete_doc_def" => (DeletedItemType::DocumentDef, "doc_def_id"),
        "remove_agent" => (DeletedItemType::RosterAgent, "agent_id"),
        _ => return,
    };

    let item_name = result["name"].as_str().unwrap_or("Unknown").to_string();
    let item_id = parse_uuid_field(result, "id")
        .or_else(|| parse_uuid_field(input, id_field))
        .unwrap_or_else(Uuid::new_v4);

    consistency_scanner::schedule_consistency_scan(
        state.clone(),
        ctx.workflow_id,
        DeletedItem {
            item_type,
            name: item_name,
            id: item_id,
            source_step_id: ctx.step_id,
            source_step_name: ctx.step_name.clone(),
        },
    );
}
