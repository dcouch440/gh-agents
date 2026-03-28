//! Workflow event broadcasting for step tool mutations.

use serde_json::Value;
use uuid::Uuid;

use crate::server::state::AppState;
use crate::server::ws::events::{WorkflowEvent, WorkflowEventKind};
use crate::types::UserId;

use super::config::StepChatContext;

/// The effect a tool call should produce on the workflow event bus.
enum ToolEffect {
    Name,
    Description,
    Plan,
}

impl ToolEffect {
    /// Parse a tool name into its broadcast effect, if any.
    fn from_tool_name(name: &str) -> Option<Self> {
        match name {
            "set_node_name" => Some(Self::Name),
            "set_node_description" => Some(Self::Description),
            "update_plan" => Some(Self::Plan),
            _ => None,
        }
    }

    /// Build the concrete workflow event kind from the tool effect and
    /// the step/input/result context.
    fn into_event_kind(self, step_id: Uuid, input: &Value, result: &Value) -> WorkflowEventKind {
        match self {
            Self::Name => {
                let name = result["name"].as_str().unwrap_or("").to_string();
                WorkflowEventKind::StepNameUpdated { step_id, name }
            }
            Self::Description => WorkflowEventKind::StepConfigUpdated { step_id },
            Self::Plan => {
                let content = input["content"].as_str().unwrap_or("").to_string();
                WorkflowEventKind::PlanUpdated { step_id, content }
            }
        }
    }
}

/// Broadcast a workflow event when a step tool mutates data.
///
/// Handles universal tools (name, description, plan). Only emits if the
/// step context is present and the tool result indicates success.
pub(crate) fn broadcast_step_event(
    state: &AppState,
    step_context: Option<&StepChatContext>,
    user_id: Option<UserId>,
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
        user_id: user_id.map(|u| u.0),
        kind,
    });
}
