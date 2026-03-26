# Deduplicate SSE Streaming Code

## Objective

Extract the shared SSE streaming logic that's copy-pasted between chat and agent execution endpoints into a single shared utility.

---

## Problem

`src/server/api/chat/mod.rs` and `src/server/api/agent_executions/mod.rs` each contain ~90 lines of identical `async_stream::stream!` code that:
- Replays buffered `StreamChunk`s
- Listens for live chunks via `broadcast::Receiver`
- Matches on `StreamChunk` variants (Token, ToolStart, ToolEnd, DocUpdate, PanelRender, Done, Error)
- Formats each variant into SSE `Event`s with JSON payloads

A shared `streaming.rs` file was previously created at `src/server/api/streaming.rs` with a `build_sse_stream()` function that does exactly this, but was never wired in (the `mod streaming;` declaration was removed). It was deleted during cleanup.

## Fix

1. Recreate `src/server/api/streaming.rs` with the shared `build_sse_stream()` function
2. Add `mod streaming;` to `src/server/api/mod.rs`
3. Replace the inline stream blocks in both `chat/mod.rs` and `agent_executions/mod.rs` with calls to `streaming::build_sse_stream(state, stream_id)`

## Impact

Eliminates ~90 lines of duplication. Future `StreamChunk` variant changes only need updating in one place.

## Verification

- `cargo check` — compiles
- Manual test: SSE streams still work for both chat and agent execution endpoints
