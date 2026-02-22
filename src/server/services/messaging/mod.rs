//! Agent messaging service: inject messages into chat sessions.
//!
//! Foundational communication layer for inter-agent messaging. Any execution
//! context (L2 manager builder, peer agents, future layers) can use this to
//! deliver messages to target sessions. Messages are wrapped in `<agent_message>`
//! XML tags, stored with `source_type = "agent"`, and broadcast via WebSocket.

use anyhow::{Context, Result};
use chrono::Utc;
use uuid::Uuid;

use crate::server::state::{AppState, ConsumerMessage};
use crate::server::ws::events::{SessionEvent, SessionEventKind};
use crate::types::UserId;

mod tests;

// ============================================================================
// Types
// ============================================================================

/// Parameters for sending an agent message to a session.
pub struct SendMessageInput {
    /// Target session ID.
    pub session_id: Uuid,
    /// User who owns the session (DB foreign key).
    pub user_id: UserId,
    /// Display name of the sending agent (e.g. "Manager", "Collector").
    pub from_agent: String,
    /// Message type for the XML tag (e.g. "initial_instruction", "update", "coordination").
    pub message_type: String,
    /// Raw content of the message (before XML wrapping).
    pub content: String,
    /// Optional short reference ID for the XML ref attribute.
    pub ref_id: Option<String>,
    /// Whether to trigger the node assistant to auto-respond.
    pub trigger_response: bool,
}

/// Result of sending an agent message.
pub struct SendMessageResult {
    pub message_id: Uuid,
    /// The response message ID (if auto-response was triggered).
    pub response_message_id: Option<Uuid>,
}

// ============================================================================
// XML Wrapping
// ============================================================================

/// Wrap content in the standard `<agent_message>` XML format.
///
/// Produces:
/// ```xml
/// <agent_message from="Manager" type="update" ref="c8f2">
/// content here
/// </agent_message>
/// ```
pub fn wrap_agent_xml(
    from: &str,
    message_type: &str,
    ref_id: Option<&str>,
    content: &str,
) -> String {
    let ref_attr = match ref_id {
        Some(r) => format!(r#" ref="{r}""#),
        None => String::new(),
    };
    format!(
        r#"<agent_message from="{from}" type="{message_type}"{ref_attr}>
{content}
</agent_message>"#
    )
}

// ============================================================================
// Core Service
// ============================================================================

/// Send an agent message to a target session.
///
/// 1. Wraps content in `<agent_message>` XML tags.
/// 2. Inserts into session with `role="user"`, `source_type="agent"`.
/// 3. Broadcasts a WebSocket event for real-time frontend display.
/// 4. Optionally triggers the node assistant to auto-respond via the chat consumer.
///
/// The message is stored with role "user" because the LLM (the node assistant)
/// should treat it as incoming input and respond to it on its next turn.
pub async fn send_message(state: &AppState, input: SendMessageInput) -> Result<SendMessageResult> {
    let message_id = Uuid::new_v4();
    let xml_content = wrap_agent_xml(
        &input.from_agent,
        &input.message_type,
        input.ref_id.as_deref(),
        &input.content,
    );

    // Store with role="user" so the assistant responds, source_type="agent" for UI distinction.
    state
        .repos()
        .sessions
        .insert_agent_message(
            input.user_id,
            input.session_id,
            message_id,
            "user".to_string(),
            xml_content.clone(),
            "agent".to_string(),
        )
        .await
        .context("Failed to insert agent message")?;

    // Broadcast real-time event.
    let content_preview = if xml_content.len() > 200 {
        format!("{}...", &xml_content[..200])
    } else {
        xml_content.clone()
    };

    state.broadcast(crate::server::ws::events::ServerEvent::Session(
        SessionEvent {
            session_id: input.session_id,
            user_id: Some(input.user_id.0),
            kind: SessionEventKind::AgentMessage {
                message_id,
                from_agent: input.from_agent,
                message_type: input.message_type,
                content_preview,
            },
        },
    ));

    // Trigger auto-response if requested.
    let response_message_id = if input.trigger_response {
        let response_id = Uuid::new_v4();
        state.ensure_response_stream(response_id);

        // Look up the session to get agent_id for the consumer.
        let agent_id = state
            .repos()
            .sessions
            .get_session(input.session_id)
            .await
            .ok()
            .flatten()
            .and_then(|s| s.agent_id);

        let _ = state
            .chat_tx()
            .send(ConsumerMessage {
                id: response_id,
                user_id: input.user_id,
                session_id: Some(input.session_id),
                agent_id,
                content: xml_content,
                timestamp: Utc::now(),
            })
            .await;

        Some(response_id)
    } else {
        None
    };

    Ok(SendMessageResult {
        message_id,
        response_message_id,
    })
}

// ============================================================================
// Tool Execution Handler
// ============================================================================

/// Execute the `dispatch_to_nodes` batch tool call.
///
/// Resolves each node ref → step → session, sends messages concurrently,
/// and returns aggregated per-node results.
pub async fn execute_dispatch_to_nodes_tool(
    state: &AppState,
    input: &serde_json::Value,
    from_agent: &str,
    user_id: UserId,
    workflow_id: Uuid,
) -> serde_json::Value {
    let Some(messages) = input["messages"].as_array() else {
        return serde_json::json!({ "error": "Missing required parameter: messages" });
    };

    if messages.is_empty() {
        return serde_json::json!({ "error": "messages array must not be empty" });
    }

    let mut handles = tokio::task::JoinSet::new();

    for msg in messages {
        let Some(node_ref) = msg["node"].as_str() else {
            continue;
        };
        let Some(message_type) = msg["message_type"].as_str() else {
            continue;
        };
        let Some(content) = msg["content"].as_str() else {
            continue;
        };

        let state = state.clone();
        let from_agent = from_agent.to_string();
        let node_ref = node_ref.to_string();
        let message_type = message_type.to_string();
        let content = content.to_string();

        handles.spawn(async move {
            // Resolve ref → step
            let step = match state
                .repos()
                .workflows
                .find_step_by_ref_id(workflow_id, &node_ref)
                .await
            {
                Ok(Some(s)) => s,
                Ok(None) => {
                    return serde_json::json!({
                        "node": node_ref,
                        "status": "error",
                        "error": format!("No step found for ref \"{node_ref}\""),
                    });
                }
                Err(e) => {
                    return serde_json::json!({
                        "node": node_ref,
                        "status": "error",
                        "error": format!("DB error: {e}"),
                    });
                }
            };

            // Resolve step → session
            let session = match state
                .repos()
                .sessions
                .find_session_by_step_id(step.id)
                .await
            {
                Ok(Some(s)) => s,
                Ok(None) => {
                    return serde_json::json!({
                        "node": node_ref,
                        "status": "error",
                        "error": format!("No session for node \"{node_ref}\". The node may not have been opened yet."),
                    });
                }
                Err(e) => {
                    return serde_json::json!({
                        "node": node_ref,
                        "status": "error",
                        "error": format!("DB error: {e}"),
                    });
                }
            };

            let ref_id = Uuid::new_v4().to_string()[..8].to_string();

            match send_message(
                &state,
                SendMessageInput {
                    session_id: session.id,
                    user_id,
                    from_agent,
                    message_type,
                    content,
                    ref_id: Some(ref_id),
                    trigger_response: true,
                },
            )
            .await
            {
                Ok(result) => serde_json::json!({
                    "node": node_ref,
                    "status": "delivered",
                    "message_id": result.message_id.to_string(),
                }),
                Err(e) => serde_json::json!({
                    "node": node_ref,
                    "status": "error",
                    "error": format!("Failed to send: {e}"),
                }),
            }
        });
    }

    let mut results = Vec::new();
    while let Some(result) = handles.join_next().await {
        match result {
            Ok(value) => results.push(value),
            Err(e) => results.push(serde_json::json!({
                "status": "error",
                "error": format!("Task join error: {e}"),
            })),
        }
    }

    serde_json::json!({
        "dispatched": results.len(),
        "results": results,
    })
}

/// Execute the `send_message` tool call.
///
/// Resolves the target session from `step_id`, wraps content in XML, and
/// delivers the message with auto-response enabled.
pub async fn execute_send_message_tool(
    state: &AppState,
    input: &serde_json::Value,
    from_agent: &str,
    user_id: UserId,
) -> serde_json::Value {
    let Some(step_id_str) = input["step_id"].as_str() else {
        return serde_json::json!({ "error": "Missing required parameter: step_id" });
    };
    let Ok(step_id) = Uuid::parse_str(step_id_str) else {
        return serde_json::json!({ "error": format!("Invalid step_id UUID: {step_id_str}") });
    };
    let Some(message_type) = input["message_type"].as_str() else {
        return serde_json::json!({ "error": "Missing required parameter: message_type" });
    };
    let Some(content) = input["content"].as_str() else {
        return serde_json::json!({ "error": "Missing required parameter: content" });
    };

    // Resolve session from step_id.
    let session = match state
        .repos()
        .sessions
        .find_session_by_step_id(step_id)
        .await
    {
        Ok(Some(s)) => s,
        Ok(None) => {
            return serde_json::json!({
                "error": format!("No session found for step {step_id}. The node may not have been opened yet.")
            });
        }
        Err(e) => {
            return serde_json::json!({ "error": format!("DB error: {e}") });
        }
    };

    let ref_id = Uuid::new_v4().to_string()[..8].to_string();

    match send_message(
        state,
        SendMessageInput {
            session_id: session.id,
            user_id,
            from_agent: from_agent.to_string(),
            message_type: message_type.to_string(),
            content: content.to_string(),
            ref_id: Some(ref_id),
            trigger_response: true,
        },
    )
    .await
    {
        Ok(result) => serde_json::json!({
            "status": "delivered",
            "message_id": result.message_id.to_string(),
            "session_id": session.id.to_string(),
        }),
        Err(e) => serde_json::json!({ "error": format!("Failed to send: {e}") }),
    }
}
