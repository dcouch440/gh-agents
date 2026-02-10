# Phase 3: Frontend — Assistant Tab on DocumenterNode

**Scope:** Frontend only — add a 5th tab to the DocumenterNode that hosts the ChatPanel, wired to session lifecycle endpoints.

## 3.1 Assistant Tab Registration

### What

Add an "Assistant" tab (with a chat/spark icon) as the 5th tab on the DocumenterNode's `CanvasFormNode` tab bar.

### Implementation

**In `DocumenterNode.tsx`, add to the tabs array:**

```typescript
{
  icon: AutoAwesomeOutlined,  // or SmartToyOutlined — spark/AI icon
  content: <AssistantTab stepId={id} workflowId={workflowId} />,
}
```

Tab order: Prompt | Documents | Inputs | Settings | **Assistant**

The tab is always visible (not gated behind a feature flag for v1).

---

## 3.2 AssistantTab Component

### What

A new tab component that manages the chat session lifecycle and renders the existing `ChatPanel`.

### Component tree

```
AssistantTab
├── useAssistantSession(workflowId, stepId)  // hook: session lifecycle
├── AssistantHeader                           // clear button, connection status
└── ChatPanel                                 // existing reusable component
    ├── ChatMessage[]                         // existing
    └── ChatInput                             // existing
```

### `AssistantTab.tsx`

```
frontend/src/components/canvas/DocumenterNode/tabs/AssistantTab.tsx
```

**Props:**

```typescript
type AssistantTabProps = {
  stepId: string
  workflowId: string
}
```

**Behavior:**

1. On mount, call `useAssistantSession(workflowId, stepId)`
2. Hook returns `{ session, messages, isLoading, error, sendMessage, clearHistory, streaming }`
3. Render states:
   - **Loading:** Spinner while session is being created/fetched
   - **Error:** Error message with retry
   - **Ready:** `ChatPanel` with messages and input

**Layout:**

```
+-----------------------------------+
| Assistant        [Clear] [status] |  <- AssistantHeader
+-----------------------------------+
| [messages scroll area]            |
|                                   |
| user: Set up docs for this API    |
| assistant: I'll analyze...        |
|   [creating: API Reference]       |  <- tool indicator
| assistant: Done! Created 3 docs.  |
|                                   |
+-----------------------------------+
| [message input]            [send] |  <- ChatInput
+-----------------------------------+
```

Full height of the tab content area. Messages scroll, input fixed at bottom.

---

## 3.3 `useAssistantSession` Hook

### What

Manages the full session lifecycle: create/find session, load history, send messages, track streaming state.

### File

```
frontend/src/hooks/useAssistantSession.ts
```

### Interface

```typescript
type UseAssistantSessionReturn = {
  session: ChatSession | null
  messages: ChatMessageData[]
  isLoading: boolean
  error: string | null
  streaming: boolean
  sendMessage: (content: string) => void
  clearHistory: () => void
}

const useAssistantSession = (
  workflowId: string,
  stepId: string,
): UseAssistantSessionReturn
```

### State machine

```
INIT -> LOADING_SESSION -> SESSION_READY -> LOADING_HISTORY -> READY
                                                                 |
                                                   SENDING -> STREAMING -> READY
                                                                 |
                                                   CLEARING -> READY
```

### Behavior

**On mount (or when stepId changes):**

1. Set `isLoading = true`
2. Call `api.workflows.getOrCreateAssistantSession(workflowId, stepId)`
3. Store session
4. Call `api.sessions.getHistory(session.id, { limit: 100 })`
5. Store messages, set `isLoading = false`

**`sendMessage(content)`:**

1. Optimistically append user message to local state
2. Call `api.sessions.sendChat(session.id, { message: content })`
3. Get back `{ message_id }`
4. Set `streaming = true`
5. Connect to SSE: `GET /api/sessions/{session_id}/chat/{message_id}/stream`
6. Process `StreamChunk` events (Phase 4 details)
7. On `Done` chunk: set `streaming = false`, append final assistant message

**`clearHistory()`:**

1. Confirm with user (simple `window.confirm` for v1)
2. Call `api.workflows.clearAssistantMessages(workflowId, stepId)`
3. Clear local messages array

**Cleanup:**

- On unmount: close any active SSE connection
- On stepId change: close SSE, reset state, re-initialize

---

## 3.4 AssistantHeader Component

### What

Thin header bar above the chat with a clear button and optional status indicator.

### File

```
frontend/src/components/canvas/DocumenterNode/tabs/AssistantHeader.tsx
```

### Content

- Left: "Assistant" label (text, not a heading — the tab already identifies the section)
- Right: Clear button (trash icon, calls `clearHistory`)
- Right: Optional dot indicator — green when connected, gray when idle

---

## 3.5 ChatPanel Adaptations

### What

The existing `ChatPanel` should work as-is for the basic case. Minor adjustments may be needed:

1. **Height constraint** — ChatPanel needs to fill the tab content area. It may need a `className` or `style` prop for height control since it's now inside a form node tab rather than a full sidebar.

2. **Empty state** — When no messages exist, show a contextual placeholder:
   > "Ask me to set up documents for this step. I can see your prompt and incoming context sources."

3. **Disabled state** — While `isLoading` is true (session being created), input should be disabled.

Review `ChatPanel` props and determine if these are already supported or need minor additions. Prefer extending existing props over forking the component.

---

## 3.6 Extracting `workflowId` in DocumenterNode

The `DocumenterNode` currently receives `stepId` as its React Flow node `id`, but doesn't have direct access to `workflowId`. This is needed for the API calls.

**Options (pick simplest):**

1. **From store** — `workflowStore.selectActiveWorkflowId()` — the active workflow is already in the store
2. **As node data** — Add `workflowId` to `DocumenterNodeData` in mappers. Slightly more explicit.

Recommend option 1 (store) since it's already available and avoids touching the mapper data shape.

---

## 3.7 Description Field in Settings Tab

### What

The step `description` column (added in Phase 1) needs to be editable in the DocumenterNode's Settings tab. This is also where other node types (StepNode, ContextNode) should eventually expose description editing, but for this phase we only add it to the DocumenterNode.

### Implementation

In the existing `SettingsTab.tsx` for the DocumenterNode, add a text input for `description`:

- Label: "Description"
- Placeholder: shows the execution_mode default (e.g., "Document generation orchestrator...")
- On change: `workflowStore.patchStepLocal(stepId, { description: value })`
- Saved alongside other dirty fields via `saveAllDirtySteps()`

This field is also relevant for upstream nodes — users can customize the description of a Researcher or Context node in their own Settings tabs so the documenter assistant gets richer context. That's a follow-up, not this phase.

---

### Files to create/modify

| File | Change |
|------|--------|
| `frontend/src/components/canvas/DocumenterNode/tabs/AssistantTab.tsx` | **New** — tab component |
| `frontend/src/components/canvas/DocumenterNode/tabs/AssistantHeader.tsx` | **New** — header with clear button |
| `frontend/src/components/canvas/DocumenterNode/tabs/SettingsTab.tsx` | Add description field |
| `frontend/src/components/canvas/DocumenterNode/DocumenterNode.tsx` | Add 5th tab |
| `frontend/src/hooks/useAssistantSession.ts` | **New** — session lifecycle hook |
| `frontend/src/hooks/useAssistantSession.test.ts` | **New** — hook tests |
| `frontend/src/components/chat/ChatPanel.tsx` | Minor: empty state, height flexibility (if needed) |
| `frontend/src/api/api.ts` | Already done in Phase 2, but verify session history/send endpoints are typed |

### Tests

- `useAssistantSession` hook tests:
  - Creates session on mount, loads history
  - Returns cached session on re-render (no duplicate API calls)
  - `sendMessage` optimistically adds user message
  - `clearHistory` clears messages and calls API
  - Cleanup closes SSE on unmount
- `AssistantTab` render tests:
  - Shows loading state while session initializes
  - Renders ChatPanel with messages when ready
  - Shows error state on API failure
- `AssistantHeader` render tests:
  - Clear button calls clearHistory
  - Disabled during loading

## Acceptance Criteria

- [ ] 5th tab visible on DocumenterNode with appropriate icon
- [ ] Tab opens ChatPanel connected to a real session
- [ ] Session created lazily on first tab open
- [ ] History loads and displays on tab open
- [ ] User can send messages (optimistic UI)
- [ ] Clear button wipes conversation with confirmation
- [ ] Empty state shows helpful prompt referencing incoming context
- [ ] Input disabled while session is loading
- [ ] SSE connection cleaned up on unmount/tab switch
- [ ] Description field editable in Settings tab, saved via dirty step mechanism
- [ ] Hook and component tests pass
