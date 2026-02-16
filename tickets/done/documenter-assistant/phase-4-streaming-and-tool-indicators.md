# Phase 4: Streaming and Tool Indicators

**Scope:** Frontend — wire up SSE streaming for real-time token display, and surface tool execution indicators in the chat so users see what the agent is doing.

## 4.1 SSE Stream Connection

### What

When the user sends a message, the hook connects to the SSE endpoint and processes `StreamChunk` events in real time. Tokens appear character-by-character in the chat.

### Implementation

**In `useAssistantSession`, after `sendChat` returns `message_id`:**

```typescript
const streamUrl = `${API_BASE}/sessions/${sessionId}/chat/${messageId}/stream`
const eventSource = new EventSource(streamUrl)
```

**Stream chunk types (matching backend `StreamChunk` enum):**

```typescript
type StreamChunk =
  | { type: 'token'; text: string }
  | { type: 'tool_start'; name: string; tool_id: string }
  | { type: 'tool_end'; name: string; tool_id: string }
  | { type: 'doc_update'; doc_id: string; title: string }
  | { type: 'done' }
  | { type: 'error'; message: string }
```

**Processing loop:**

```typescript
eventSource.onmessage = (event) => {
  const chunk: StreamChunk = JSON.parse(event.data)

  switch (chunk.type) {
    case 'token':
      // Append to current assistant message content
      appendToStreamingMessage(chunk.text)
      break

    case 'tool_start':
      // Add tool indicator to message
      addToolIndicator(chunk.name, chunk.tool_id, 'running')
      break

    case 'tool_end':
      // Mark tool indicator as complete
      updateToolIndicator(chunk.tool_id, 'complete')
      break

    case 'done':
      // Finalize assistant message, close stream
      finalizeStreamingMessage()
      setStreaming(false)
      eventSource.close()
      break

    case 'error':
      // Show error, close stream
      setError(chunk.message)
      setStreaming(false)
      eventSource.close()
      break
  }
}
```

**Reconnection:** The backend buffers all chunks, so if the SSE connection drops and reconnects, the client gets the full history. `EventSource` handles reconnection natively. However, to avoid duplicate tokens, track a `lastEventId` or deduplicate by checking if the assistant message already contains the buffered content.

**Cleanup:** Store the `EventSource` in a ref and close it on unmount.

---

## 4.2 Streaming Message State

### What

During streaming, the assistant's response builds incrementally. We need state to track the in-progress message separately from finalized messages.

### Implementation

Add to `useAssistantSession` state:

```typescript
type StreamingState = {
  messageId: string
  content: string                    // accumulated tokens
  toolIndicators: ToolIndicator[]    // active tool calls
}

type ToolIndicator = {
  toolId: string
  toolName: string
  status: 'running' | 'complete'
}
```

**`appendToStreamingMessage(text)`:**
- Append `text` to `streamingState.content`
- This triggers a re-render with the partial message visible

**`finalizeStreamingMessage()`:**
- Move `streamingState` into the `messages` array as a complete `ChatMessageData`
- Clear `streamingState`

**Rendering in ChatPanel:**
- Pass `messages` (finalized) + the streaming message (if active) to ChatPanel
- ChatPanel already supports a `streaming` prop that shows a cursor on the last message
- Tool indicators render inline within the streaming message

---

## 4.3 Tool Indicators in Chat

### What

When the agent calls a tool (e.g., `create_doc_def`), the user should see a visual indicator in the chat showing what's happening. This bridges the gap between the agent "thinking" and documents appearing on canvas.

### Visual Design

Tool indicators appear inline within the assistant message, at the position where the tool was called:

```
assistant: I'll set up three documents for your API service.

  [sparkle icon] Creating "API Reference"...        <- running (animated)
  [check icon]   Created "API Reference"             <- complete

  [sparkle icon] Creating "Authentication Guide"...  <- running

Let me also update your prompt to reference these documents.

  [check icon]   Updated prompt                      <- complete
```

### Implementation

**New component: `ToolIndicator.tsx`**

```
frontend/src/components/chat/ToolIndicator.tsx
```

```typescript
type ToolIndicatorProps = {
  toolName: string
  status: 'running' | 'complete'
}
```

**Display mapping** (tool name -> user-friendly label):

```typescript
const TOOL_LABELS: Record<string, (status: string) => string> = {
  create_doc_def: (s) => s === 'running' ? 'Creating document...' : 'Created document',
  update_doc_def: (s) => s === 'running' ? 'Updating document...' : 'Updated document',
  delete_doc_def: (s) => s === 'running' ? 'Removing document...' : 'Removed document',
  update_prompt:  (s) => s === 'running' ? 'Updating prompt...'  : 'Updated prompt',
  read_context:   (s) => s === 'running' ? 'Reading context...'  : 'Read context',
  think:          (s) => s === 'running' ? 'Thinking...'         : 'Thought',
}
```

**Styling:**
- Compact pill/badge style, inline with message flow
- Running: subtle pulse animation, muted accent color
- Complete: static, green check icon, slightly dimmed
- Monospace font for tool names

### Rendering approach

The streaming message is a composite of text segments and tool indicators. Model it as an ordered list:

```typescript
type MessageSegment =
  | { type: 'text'; content: string }
  | { type: 'tool'; toolId: string; toolName: string; status: 'running' | 'complete' }
```

When rendering the assistant message during streaming:
1. Text tokens build up the current text segment
2. `tool_start` inserts a tool segment and starts a new text segment
3. `tool_end` updates the tool segment status
4. Final render interleaves text and tool indicators

After streaming completes, the finalized message stores only the text content (tool indicators are ephemeral — they served their purpose during the stream).

---

## 4.4 ChatPanel Enhancements

### What

Minor enhancements to ChatPanel to support tool indicators during streaming.

### Changes

1. **`renderMessage` override or slot** — Allow the AssistantTab to provide a custom message renderer that handles `MessageSegment[]` for the streaming message. Non-streaming messages render normally.

2. **Auto-scroll behavior** — Ensure new tool indicators and tokens trigger scroll-to-bottom (ChatPanel may already handle this via message count/content changes).

3. **Input disabled during streaming** — ChatPanel's `disabled` prop should be true while streaming. Only one message at a time.

**Approach:** Rather than modifying ChatPanel internals, the AssistantTab can render the streaming message outside of ChatPanel's message list (as a separate element below it), or pass a `streamingContent` prop that ChatPanel renders specially. Evaluate which is cleaner.

---

### Files to create/modify

| File | Change |
|------|--------|
| `frontend/src/hooks/useAssistantSession.ts` | SSE connection, streaming state, chunk processing |
| `frontend/src/components/chat/ToolIndicator.tsx` | **New** — tool indicator badge component |
| `frontend/src/components/chat/ToolIndicator.test.tsx` | **New** — render tests |
| `frontend/src/components/chat/ChatPanel.tsx` | Minor: streaming content slot or custom renderer support |
| `frontend/src/components/canvas/DocumenterNode/tabs/AssistantTab.tsx` | Wire streaming state to ChatPanel |
| `frontend/src/types/streaming.ts` | **New** — `StreamChunk`, `MessageSegment`, `ToolIndicator` types |

### Tests

- SSE processing: token accumulation produces correct content
- SSE processing: tool_start/tool_end produce correct indicators
- SSE processing: done finalizes message
- SSE processing: error sets error state
- ToolIndicator renders running state with animation class
- ToolIndicator renders complete state with check icon
- Tool name mapping produces user-friendly labels
- Streaming message disabled input

## Acceptance Criteria

- [ ] SSE connection established after sendChat
- [ ] Tokens stream in real-time, visible character by character
- [ ] Tool indicators appear inline during streaming
- [ ] Running tools show animated/pulsing state
- [ ] Completed tools show check icon
- [ ] All 6 tool names have user-friendly labels
- [ ] Stream completes cleanly on `done` chunk
- [ ] Error chunk shows error message in chat
- [ ] Input disabled during streaming
- [ ] SSE cleaned up on unmount
- [ ] EventSource reconnection doesn't duplicate content
- [ ] Tests for chunk processing and indicator rendering
