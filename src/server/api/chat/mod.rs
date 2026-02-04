//! Chat endpoints

use std::convert::Infallible;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::sse::{Event, Sse},
    Json,
};
use chrono::{DateTime, Utc};
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::constants::MAX_CHAT_MESSAGE_LENGTH;
use crate::server::auth as auth_utils;
use crate::server::state::{AppState, ConsumerMessage, StreamChunk};

/// Request body for sending a chat message
#[derive(Deserialize, utoipa::ToSchema)]
pub struct ChatRequest {
    pub message: String,
}

/// Response for sending a chat message
#[derive(Serialize, utoipa::ToSchema)]
pub struct ChatResponse {
    pub message_id: Uuid,
    pub status: String,
}

/// Query parameters for chat history
#[derive(Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct HistoryQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// A chat message in the response
#[derive(Serialize, utoipa::ToSchema)]
pub struct ChatMessage {
    pub id: Uuid,
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

/// Send a chat message to the orchestrator
///
/// Returns 202 Accepted with the message ID.
/// The message is queued for processing by the orchestrator.
#[utoipa::path(
    post,
    path = "/api/chat",
    tag = "Chat",
    security(("bearer_auth" = [])),
    request_body = ChatRequest,
    responses(
        (status = 202, description = "Message queued", body = ChatResponse),
        (status = 400, description = "Invalid message")
    )
)]
pub async fn send_chat(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Json(request): Json<ChatRequest>,
) -> Result<(StatusCode, Json<ChatResponse>), StatusCode> {
    if request.message.trim().is_empty() || request.message.len() > MAX_CHAT_MESSAGE_LENGTH {
        return Err(StatusCode::BAD_REQUEST);
    }

    let message_id = Uuid::new_v4();

    // Pre-create the buffered stream so chunks are captured even before
    // the SSE client connects
    state.ensure_response_stream(message_id).await;

    // Store the user message in the database
    state
        .repo
        .insert_chat_message(
            auth.user_id,
            message_id,
            "user".to_string(),
            request.message.clone(),
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Queue message to chat consumer
    state
        .chat_tx
        .send(ConsumerMessage {
            id: message_id,
            user_id: auth.user_id,
            session_id: None,
            agent_id: None,
            content: request.message,
            timestamp: Utc::now(),
        })
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((
        StatusCode::ACCEPTED,
        Json(ChatResponse {
            message_id,
            status: "queued".to_string(),
        }),
    ))
}

/// Get chat history with pagination
///
/// Returns messages in chronological order.
#[utoipa::path(
    get,
    path = "/api/chat/history",
    tag = "Chat",
    security(("bearer_auth" = [])),
    params(HistoryQuery),
    responses(
        (status = 200, description = "Chat history", body = Vec<ChatMessage>)
    )
)]
pub async fn get_chat_history(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<Vec<ChatMessage>>, StatusCode> {
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);

    let rows = state
        .repo
        .get_chat_history(auth.user_id, limit, offset)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let messages: Vec<ChatMessage> = rows
        .into_iter()
        .map(|row| ChatMessage {
            id: row.id,
            role: row.role,
            content: row.content,
            timestamp: row.timestamp,
        })
        .collect();

    Ok(Json(messages))
}

/// Stream chat response via Server-Sent Events
///
/// Subscribes to the response stream for a specific message and
/// streams tokens as they are generated.
#[utoipa::path(
    get,
    path = "/api/chat/{message_id}/stream",
    tag = "Chat",
    params(("message_id" = Uuid, Path, description = "Message ID")),
    responses(
        (status = 200, description = "SSE event stream")
    )
)]
pub async fn chat_stream(
    State(state): State<AppState>,
    Path(message_id): Path<Uuid>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    chat_stream_inner(state, message_id)
}

/// Stream chat response for session-scoped messages.
///
/// Same as `chat_stream` but extracts both session_id and message_id
/// from the path (only message_id is used for stream lookup).
#[utoipa::path(
    get,
    path = "/api/sessions/{session_id}/chat/{message_id}/stream",
    tag = "Sessions",
    params(
        ("session_id" = Uuid, Path, description = "Session ID"),
        ("message_id" = Uuid, Path, description = "Message ID")
    ),
    responses(
        (status = 200, description = "SSE event stream")
    )
)]
pub async fn session_chat_stream(
    State(state): State<AppState>,
    Path((_session_id, message_id)): Path<(Uuid, Uuid)>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    chat_stream_inner(state, message_id)
}

fn chat_stream_inner(
    state: AppState,
    message_id: Uuid,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = async_stream::stream! {
        let (buffered, mut rx, already_done) = state.get_response_stream(message_id).await;

        // Replay any buffered chunks that arrived before we connected
        for chunk in buffered {
            match chunk {
                StreamChunk::Token(text) => {
                    yield Ok(Event::default().event("token").data(serde_json::to_string(&text).unwrap_or(text)));
                }
                StreamChunk::ToolStart { name, tool_id } => {
                    let data = format!(r#"{{"name":"{}","id":"{}"}}"#, name, tool_id);
                    yield Ok(Event::default().event("tool_start").data(data));
                }
                StreamChunk::ToolEnd { name, tool_id } => {
                    let data = format!(r#"{{"name":"{}","id":"{}"}}"#, name, tool_id);
                    yield Ok(Event::default().event("tool_end").data(data));
                }
                StreamChunk::DocUpdate { doc_id, title } => {
                    let data = format!(r#"{{"doc_id":"{}","title":"{}"}}"#, doc_id, title);
                    yield Ok(Event::default().event("doc_update").data(data));
                }
                StreamChunk::Done => {
                    yield Ok(Event::default().event("done").data(""));
                    return;
                }
                StreamChunk::Error(e) => {
                    yield Ok(Event::default().event("error").data(e));
                    return;
                }
            }
        }

        if already_done {
            yield Ok(Event::default().event("done").data(""));
            return;
        }

        // Listen for new chunks from the orchestrator
        loop {
            match rx.recv().await {
                Ok(chunk) => {
                    match chunk {
                        StreamChunk::Token(text) => {
                            yield Ok(Event::default().event("token").data(serde_json::to_string(&text).unwrap_or(text)));
                        }
                        StreamChunk::ToolStart { name, tool_id } => {
                            let data = format!(r#"{{"name":"{}","id":"{}"}}"#, name, tool_id);
                            yield Ok(Event::default().event("tool_start").data(data));
                        }
                        StreamChunk::ToolEnd { name, tool_id } => {
                            let data = format!(r#"{{"name":"{}","id":"{}"}}"#, name, tool_id);
                            yield Ok(Event::default().event("tool_end").data(data));
                        }
                        StreamChunk::DocUpdate { doc_id, title } => {
                            let data = format!(r#"{{"doc_id":"{}","title":"{}"}}"#, doc_id, title);
                            yield Ok(Event::default().event("doc_update").data(data));
                        }
                        StreamChunk::Done => {
                            yield Ok(Event::default().event("done").data(""));
                            break;
                        }
                        StreamChunk::Error(e) => {
                            yield Ok(Event::default().event("error").data(e));
                            break;
                        }
                    }
                }
                Err(broadcast::error::RecvError::Closed) => {
                    break;
                }
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    continue;
                }
            }
        }
    };

    Sse::new(stream)
}

/// Clear all chat history
///
/// Returns 204 No Content on success.
#[utoipa::path(
    delete,
    path = "/api/chat/history",
    tag = "Chat",
    security(("bearer_auth" = [])),
    responses(
        (status = 204, description = "Chat history cleared")
    )
)]
pub async fn clear_chat_history(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
) -> StatusCode {
    match state.repo.clear_chat_history(auth.user_id).await {
        Ok(_) => StatusCode::NO_CONTENT,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
#[cfg(test)]
mod tests;
