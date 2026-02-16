# Phase 5: Reactive Canvas — Real-Time Document Node Materialization

**Scope:** Backend + Frontend — close the loop so that when the agent creates/updates/deletes document defs via tools, the canvas updates in real time without manual refresh.

## 5.1 Backend: WS Broadcast on Doc-Def Mutation

### What

When a documenter tool (`create_doc_def`, `update_doc_def`, `delete_doc_def`) executes, broadcast a WebSocket event on the `Workflow` topic so all connected clients know to refresh.

### Implementation

**New WS event type:**

```rust
// In ws/events.rs or similar
pub fn doc_def_changed_event(workflow_id: Uuid, step_id: Uuid, action: &str) -> WsEvent {
    WsEvent {
        topic: Topic::Workflow,
        event: "doc_def_changed".into(),
        data: json!({
            "workflow_id": workflow_id.to_string(),
            "step_id": step_id.to_string(),
            "action": action,  // "created" | "updated" | "deleted"
        }),
        run_id: None,
        user_id: None,
        ts: Utc::now(),
    }
}
```

**Broadcast in tool execution handlers:**

```rust
// In src/server/tools/documenter/mod.rs

async fn execute_create_doc_def(ctx: &DocumenterToolContext, input: &Value) -> Value {
    // ... create doc def via repo ...

    // Broadcast change
    ctx.state.ws_broadcast(doc_def_changed_event(
        ctx.workflow_id,
        ctx.step_id,
        "created",
    )).await;

    // Return result to agent
    json!({ "id": def.id, "name": def.name, "created": true })
}
```

Same pattern for `update_doc_def` and `delete_doc_def`.

**Also broadcast on `update_prompt` tool:**

```rust
pub fn step_prompt_changed_event(workflow_id: Uuid, step_id: Uuid) -> WsEvent {
    WsEvent {
        topic: Topic::Workflow,
        event: "step_prompt_changed".into(),
        data: json!({
            "workflow_id": workflow_id.to_string(),
            "step_id": step_id.to_string(),
        }),
        ..
    }
}
```

This lets the Prompt tab update if it's open while the agent modifies the prompt.

---

## 5.2 Frontend: WS Listener for Doc-Def Changes

### What

The `WorkflowCanvas` (or a hook used by it) subscribes to `Workflow` topic events. When a `doc_def_changed` event arrives for the active workflow, it refetches document defs for the affected step. The existing mapper pipeline then re-generates DocumentNodes and edges.

### Implementation

**New hook: `useDocDefSync`**

```
frontend/src/hooks/useDocDefSync.ts
```

```typescript
const useDocDefSync = (workflowId: string | null) => {
  const ws = useWebSocket()

  useEffect(() => {
    if (!workflowId) return

    const unsubscribe = ws.on('workflow', (event) => {
      if (event.event === 'doc_def_changed') {
        const { step_id } = event.data
        // Refetch doc defs for the affected step
        workflowStore.fetchDocumentDefs(step_id)
      }

      if (event.event === 'step_prompt_changed') {
        const { step_id } = event.data
        // Refetch step to get updated prompt
        workflowStore.fetchStep(workflowId, step_id)
      }
    })

    return unsubscribe
  }, [workflowId, ws])
}
```

**Usage in `WorkflowCanvas.tsx`:**

```typescript
function WorkflowCanvasInner() {
  const activeWorkflowId = useStore(workflowStore.store, workflowStore.selectActiveWorkflowId)

  // Existing hooks...
  useDocDefSync(activeWorkflowId)

  // Rest of canvas...
}
```

### What happens after refetch

1. `workflowStore.fetchDocumentDefs(stepId)` updates `state.documentDefsByStep[stepId]`
2. `WorkflowCanvas` re-runs `useMemo` → `toRFNodes()` which reads `documentDefsByStep`
3. New/removed `DocumentNode` entries appear/disappear in the RF node array
4. `toDocumentEdges()` re-generates synthetic edges
5. React Flow's smart diffing updates the canvas — new nodes animate in, deleted nodes disappear
6. User sees documents materialize/vanish while chatting

### Positioning of new document nodes

The existing `toRFNodes` mapper positions document nodes above the documenter step (staggered horizontally). New nodes from the agent follow the same positioning logic. If the user has manually repositioned existing document nodes, only the *new* ones use auto-positioning.

---

## 5.3 Prompt Tab Sync

### What

If the agent updates the prompt via `update_prompt` tool while the Prompt tab is not active, the next time the user opens the Prompt tab it should show the updated prompt. If the Prompt tab IS open, it should update in real time.

### Implementation

The `step_prompt_changed` WS event triggers `workflowStore.fetchStep()`, which updates the step's `prompt_template` in the store. The Prompt tab reads from the store, so it updates automatically.

**Edge case:** If the user has unsaved edits in the Prompt tab (step is in `dirtyStepIds`), the refetch should NOT overwrite their local changes. The store's `fetchStep` should skip steps that are dirty.

```typescript
// In workflowStore.fetchStep():
fetchStep: async (workflowId: string, stepId: string) => {
  const state = get()
  // Don't overwrite dirty local edits
  if (state.dirtyStepIds.has(stepId)) return

  const step = await api.workflows.getStep(workflowId, stepId)
  // Update store...
}
```

---

## 5.4 Optimistic UI in Chat

### What

For an even snappier feel, the chat's tool indicators can optimistically show the document name being created (extracted from the `tool_start` event data) before the WS event arrives. The WS event then confirms and triggers the actual canvas update.

### Implementation

This is a nice-to-have. The SSE `tool_start` chunk includes the tool name but not the tool input (for security). If we want to show "Creating API Reference..." we'd need to either:

1. Include a `summary` field in the `ToolStart` stream chunk (backend change)
2. Or just show "Creating document..." generically

Recommend option 1 as a small backend enhancement:

```rust
pub enum StreamChunk {
    ToolStart {
        name: String,
        tool_id: String,
        summary: Option<String>,  // e.g., "API Reference"
    },
    // ...
}
```

The tool execution handler can set `summary` to the document name before calling the actual create. This makes the chat feel immediate even if the WS event takes a moment.

---

### Files to create/modify

| File | Change |
|------|--------|
| `src/server/tools/documenter/mod.rs` | Add WS broadcast after each mutation tool |
| `src/server/ws/events.rs` | New event constructors: `doc_def_changed`, `step_prompt_changed` |
| `src/server/hub/streaming/mod.rs` | Add optional `summary` to `StreamChunk::ToolStart` |
| `frontend/src/hooks/useDocDefSync.ts` | **New** — WS listener for doc-def changes |
| `frontend/src/hooks/useDocDefSync.test.ts` | **New** — tests |
| `frontend/src/components/canvas/WorkflowCanvas.tsx` | Use `useDocDefSync` hook |
| `frontend/src/stores/workflowStore.ts` | Add `fetchStep()` method, skip-if-dirty guard |

### Tests

**Backend:**
- Tool execution broadcasts WS event with correct topic/event/data
- Event includes workflow_id, step_id, action
- `update_prompt` tool broadcasts `step_prompt_changed`

**Frontend:**
- `useDocDefSync` calls `fetchDocumentDefs` on `doc_def_changed` event
- `useDocDefSync` calls `fetchStep` on `step_prompt_changed` event
- `fetchStep` skips dirty steps
- New DocumentNodes appear in canvas after refetch
- Deleted DocumentNodes disappear from canvas after refetch

## Acceptance Criteria

- [ ] `create_doc_def` tool broadcasts `doc_def_changed` WS event with action "created"
- [ ] `update_doc_def` tool broadcasts `doc_def_changed` WS event with action "updated"
- [ ] `delete_doc_def` tool broadcasts `doc_def_changed` WS event with action "deleted"
- [ ] `update_prompt` tool broadcasts `step_prompt_changed` WS event
- [ ] Frontend WS listener triggers store refetch on doc-def events
- [ ] New DocumentNodes appear on canvas in real time during chat
- [ ] Deleted DocumentNodes disappear from canvas in real time
- [ ] Prompt tab updates when agent modifies prompt (unless user has dirty edits)
- [ ] Dirty step guard prevents overwriting local edits
- [ ] Optional: `StreamChunk::ToolStart` includes summary for richer indicators
- [ ] All WS events and listener hooks tested

---

## Integration Test: Full Loop

After all 5 phases are complete, validate the end-to-end flow:

1. Open a workflow with a documenter step
2. Click the Assistant tab — session created, empty state shown
3. Type "Set up API documentation for this service"
4. Watch:
   - Agent streams response with tokens appearing in real time
   - Tool indicators appear: "Creating API Reference...", "Creating Authentication Guide..."
   - DocumentNodes materialize on the canvas as each tool completes
   - Edges auto-connect from documenter to new document nodes
5. Type "Remove the changelog and add a migration guide"
   - Agent calls delete + create
   - Canvas updates: one node disappears, another appears
6. Switch to Documents tab — see all definitions the agent created
7. Manually add a document in Documents tab, switch back to Assistant
8. Type "What documents exist now?"
   - Agent calls `read_context`, sees the manually-added one too
9. Click Clear — conversation wiped
10. Type again — agent still sees all the document defs (context from DB, not chat history)
