//! Shared SSE streaming for response streams.
//!
//! Used by both chat and agent execution endpoints.

use std::convert::Infallible;

use axum::response::sse::{Event, Sse};
use futures::stream::Stream;
use tokio::sync::broadcast;

use crate::server::state::{AppState, StreamChunk};

/// Build an SSE stream from a buffered response stream.
///
/// Replays buffered chunks first, then listens for live chunks.
/// Both `chat_stream` and `execution_message_stream` delegate to this.
pub(crate) fn build_sse_stream(
    state: AppState,
    stream_id: uuid::Uuid,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = async_stream::stream! {
        let (buffered, mut rx, already_done) = state.get_response_stream(stream_id);

        // Replay any buffered chunks that arrived before we connected
        for chunk in buffered {
            match chunk_to_event(&chunk) {
                ChunkAction::Yield(event) => yield Ok(event),
                ChunkAction::Done(event) => { yield Ok(event); return; }
            }
        }

        if already_done {
            yield Ok(Event::default().event("done").data(""));
            return;
        }

        // Listen for live chunks
        loop {
            match rx.recv().await {
                Ok(chunk) => match chunk_to_event(&chunk) {
                    ChunkAction::Yield(event) => yield Ok(event),
                    ChunkAction::Done(event) => { yield Ok(event); break; }
                },
                Err(broadcast::error::RecvError::Closed) => break,
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
            }
        }
    };

    Sse::new(stream)
}

enum ChunkAction {
    Yield(Event),
    Done(Event),
}

fn chunk_to_event(chunk: &StreamChunk) -> ChunkAction {
    match chunk {
        StreamChunk::Token(text) => {
            let data = serde_json::to_string(text).unwrap_or_else(|_| text.clone());
            ChunkAction::Yield(Event::default().event("token").data(data))
        }
        StreamChunk::ToolStart { name, tool_id } => {
            let data = format!(r#"{{"name":"{}","id":"{}"}}"#, name, tool_id);
            ChunkAction::Yield(Event::default().event("tool_start").data(data))
        }
        StreamChunk::ToolEnd { name, tool_id } => {
            let data = format!(r#"{{"name":"{}","id":"{}"}}"#, name, tool_id);
            ChunkAction::Yield(Event::default().event("tool_end").data(data))
        }
        StreamChunk::DocUpdate { doc_id, title } => {
            let data = format!(r#"{{"doc_id":"{}","title":"{}"}}"#, doc_id, title);
            ChunkAction::Yield(Event::default().event("doc_update").data(data))
        }
        StreamChunk::PanelRender {
            content,
            submit_label,
        } => {
            let data =
                serde_json::json!({ "content": content, "submit_label": submit_label })
                    .to_string();
            ChunkAction::Yield(Event::default().event("panel_render").data(data))
        }
        StreamChunk::Done => ChunkAction::Done(Event::default().event("done").data("")),
        StreamChunk::Error(e) => {
            ChunkAction::Done(Event::default().event("error").data(e.clone()))
        }
    }
}
