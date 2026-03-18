# Assistant's Notes — Frontend (Part B)

## Summary

Add a **Notes Node** to the workflow canvas — a red, auto-generated, read-only node that displays a step's assistant notes. When the assistant calls `update_notes`, the backend broadcasts a WebSocket event and the Notes Node updates in real-time. Follows the exact same pattern as DocumentNode (auto-generated, undetachable, positioned above the parent step, connected by a flowing animated edge).

**Depends on:** Part A (tickets/assistant/ASSISTANT_NOTES_AND_CONTEXT_SIMPLIFICATION.md) — specifically Parts 3-4 (database + tool handler).

---

## Part 1: Backend — WebSocket Event

### 1A. Event Variant

**File:** `src/server/ws/events.rs`

Add a new variant to `WorkflowEventKind` (after `StepNameUpdated`):

```rust
/// Assistant notes were updated for a step.
AssistantNotesUpdated {
    step_id: Uuid,
    content: String,
},
```

Content is included directly in the event (like `StepNameUpdated` includes `name`) so the frontend gets the notes instantly without a refetch round-trip.

Add the event name mapping in `WorkflowEvent::event_name()`:

```rust
WorkflowEventKind::AssistantNotesUpdated { .. } => "assistant_notes_updated",
```

### 1B. Broadcast Integration

**File:** `src/server/hub/strategies/chat/broadcast.rs`

Add to the `ToolEffect` enum:

```rust
enum ToolEffect {
    // ... existing variants ...
    NotesUpdated,
}
```

Add to `ToolEffect::from_tool_name()`:

```rust
"update_notes" => Some(Self::NotesUpdated),
```

Add to `ToolEffect::into_event_kind()`:

```rust
Self::NotesUpdated => {
    let content = result["content"]
        .as_str()
        .or_else(|| input["content"].as_str())
        .unwrap_or("")
        .to_string();
    WorkflowEventKind::AssistantNotesUpdated { step_id, content }
}
```

**Note:** The `update_notes` tool handler (Part A, Part 4B) returns `"Notes updated."` as a string, not a JSON object with a `content` field. The handler needs to either:
- Return `json!({"content": content, "message": "Notes updated."})` so the broadcast can extract `content` from `result`, OR
- The broadcast reads `content` from `input` (the tool call arguments), which always has the content.

The `input` approach is simpler since `input` always has `{"content": "..."}` — the tool arguments. Use `input["content"]` as the primary source:

```rust
Self::NotesUpdated => {
    let content = input["content"]
        .as_str()
        .unwrap_or("")
        .to_string();
    WorkflowEventKind::AssistantNotesUpdated { step_id, content }
}
```

### 1C. REST Endpoint — Fetch Notes

**File:** `src/server/api/` (new handler, or add to existing workflow step handlers)

The frontend needs to fetch notes on initial canvas load (before any WS events arrive).

```
GET /api/workflows/:workflow_id/steps/:step_id/notes
→ { "content": "..." }  (or 204 No Content if no notes)
```

Also add a batch endpoint to fetch all notes for a workflow in one call (avoids N+1 on canvas mount):

```
GET /api/workflows/:workflow_id/notes
→ { "notes": [{ "step_id": "...", "content": "..." }, ...] }
```

This batch endpoint uses `get_all_assistant_notes_for_workflow()` from Part A (Part 3B).

---

## Part 2: Frontend — WebSocket Types + Store

### 2A. Event Constant + Data Type

**File:** `frontend/src/types/ws.ts`

Add to `WORKFLOW_EVENT`:

```typescript
export const WORKFLOW_EVENT = {
  // ... existing ...
  ASSISTANT_NOTES_UPDATED: 'assistant_notes_updated',
} as const
```

Add the data type:

```typescript
export type AssistantNotesUpdatedData = {
  workflow_id: string
  step_id: string
  content: string
}
```

### 2B. Store — Notes State

**File:** `frontend/src/stores/workflowStore/_store.ts` (or wherever the store state type is defined)

Add notes storage to the workflow store state:

```typescript
// In WorkflowStoreState:
notesByStep: NormMap<string>  // step_id → notes content string
```

Initialize as empty: `notesByStep: emptyNormMap()`

Add a selector:

```typescript
const selectNotesByStep = (s: WorkflowStoreState): NormMap<string> => s.notesByStep
```

If `NormMap` is too heavy for simple strings, use a plain `Record<string, string>`:

```typescript
notesByStep: Record<string, string>  // step_id → notes content
```

### 2C. Store — Fetch Action

**File:** `frontend/src/stores/workflowStore/` (new file `notes.ts` or add to existing)

```typescript
/** Fetch all assistant notes for the active workflow. Called on canvas mount. */
const fetchAllNotes = async (workflowId: string): Promise<void> => {
  try {
    const response = await api.workflows.getAllNotes(workflowId)
    const lookup: Record<string, string> = {}
    for (const entry of response.notes) {
      lookup[entry.step_id] = entry.content
    }
    store.setState({ notesByStep: lookup })
  } catch (err) {
    console.error('[workflowStore] Failed to fetch notes:', err)
  }
}
```

### 2D. Store — WS Handler

**File:** `frontend/src/stores/workflowStore/wsHandler.ts`

Add a case for the new event:

```typescript
case WORKFLOW_EVENT.ASSISTANT_NOTES_UPDATED: {
  const d = msg.data as AssistantNotesUpdatedData
  if (d.workflow_id !== activeId) break
  store.setState((s) => ({
    notesByStep: { ...s.notesByStep, [d.step_id]: d.content },
  }))
  break
}
```

This is an **inline update** (not a refetch) because the WS event already carries the full content. The node re-renders instantly.

### 2E. API Client Method

**File:** `frontend/src/api/api.ts`

Add to the workflows section:

```typescript
getAllNotes: (workflowId: string) =>
  api.get<{ notes: Array<{ step_id: string; content: string }> }>(
    `/workflows/${workflowId}/notes`
  ),
```

---

## Part 3: Frontend — Notes Node Component

### 3A. Constants

**File:** `frontend/src/components/canvas/NotesNode/constants.ts` (new)

```typescript
export const NOTES_NODE = {
  DEFAULT_WIDTH: 360,
  DEFAULT_HEIGHT: 300,
  MIN_WIDTH: 300,
  MIN_HEIGHT: 240,
  MAX_WIDTH: 1200,
  MAX_HEIGHT: 1200,
  HEADER_HEIGHT: 44,
  ACCENT_COLOR: '#f85149',  // Red — matches PROTOCOL_TYPE_COLORS.review
} as const
```

Use `#f85149` — the existing red in the color palette (`PROTOCOL_TYPE_COLORS.review`). This keeps the color system consistent.

### 3B. Type Definition

**File:** `frontend/src/components/canvas/NotesNode/types.ts` (new)

```typescript
import type { CanvasNodeKind } from '../canvasKinds'

type NotesNodeData = {
  kind: CanvasNodeKind
  label: string         // Step name (e.g., "Security Scanner")
  stepName: string      // Parent step display name
  content: string       // The notes markdown content
  protocolStepId: string | null  // For protocol highlight linking
}

export type { NotesNodeData }
```

### 3C. Header Component

**File:** `frontend/src/components/canvas/NotesNode/NotesNodeHeader.tsx` (new)

Follow the `DocumentNodeHeader` pattern exactly:

```typescript
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { ProtocolBadge } from '../ProtocolBadge'
import { NOTES_NODE } from './constants'

// Use an appropriate MUI icon — NotesOutlined or StickyNote2Outlined
import StickyNote2Outlined from '@mui/icons-material/StickyNote2Outlined'

type NotesNodeHeaderProps = {
  stepName: string
  accentColor?: string
}

function NotesNodeHeader({
  stepName,
  accentColor = NOTES_NODE.ACCENT_COLOR,
}: NotesNodeHeaderProps) {
  return (
    <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, px: 1.5, width: '100%' }}>
      {/* Icon */}
      <Box
        sx={{
          width: 28,
          height: 28,
          borderRadius: '6px',
          backgroundColor: `${accentColor}20`,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          flexShrink: 0,
        }}
      >
        <StickyNote2Outlined sx={{ fontSize: 18, color: accentColor }} />
      </Box>

      {/* Title + subtitle */}
      <Box sx={{ flex: 1, minWidth: 0 }}>
        <Typography
          sx={{
            fontSize: 13,
            fontWeight: 600,
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
          }}
        >
          Agent Notes
        </Typography>
        <Typography
          sx={{
            fontSize: 10,
            color: 'text.disabled',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
          }}
        >
          {stepName}
        </Typography>
      </Box>

      {/* Badge */}
      <Box sx={{ mr: 0.5 }}>
        <ProtocolBadge color={accentColor} label="Notes" />
      </Box>
    </Box>
  )
}

export { NotesNodeHeader }
```

### 3D. Content Component

**File:** `frontend/src/components/canvas/NotesNode/NotesNodeContent.tsx` (new)

Follow `DocumentNodeContent` but simpler — notes are always markdown, no raw/md toggle needed:

```typescript
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { MarkdownPreview } from '@/components/shared/MarkdownPreview'
import { NOTES_NODE } from './constants'

type NotesNodeContentProps = {
  content: string
  accentColor?: string
}

function NotesNodeContent({
  content,
  accentColor = NOTES_NODE.ACCENT_COLOR,
}: NotesNodeContentProps) {
  const isEmpty = !content.trim()

  return (
    <Box
      className="nowheel nodrag nopan"
      sx={{ height: '100%', overflow: 'hidden', display: 'flex', flexDirection: 'column' }}
    >
      {isEmpty ? (
        <Box
          sx={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            height: '100%',
          }}
        >
          <Typography
            sx={{
              fontSize: 12,
              color: 'text.disabled',
              fontStyle: 'italic',
            }}
          >
            Notes will appear as the assistant records them.
          </Typography>
        </Box>
      ) : (
        <Box sx={{ px: 1.5, py: 1, overflow: 'auto', height: '100%' }}>
          <MarkdownPreview content={content} />
        </Box>
      )}
    </Box>
  )
}

export { NotesNodeContent }
```

### 3E. Main Node Component

**File:** `frontend/src/components/canvas/NotesNode/NotesNode.tsx` (new)

Follows `DocumentNode.tsx` structure exactly:

```typescript
import { memo, useState } from 'react'
import { Position, NodeResizer } from '@xyflow/react'
import type { NodeProps } from '@xyflow/react'
import Box from '@mui/material/Box'
import { useTheme } from '@mui/material/styles'
import { CanvasHandle } from '../CanvasHandle'
import { useNodeScale } from '../useNodeScale'
import { NOTES_NODE } from './constants'
import { NotesNodeHeader } from './NotesNodeHeader'
import { NotesNodeContent } from './NotesNodeContent'
import type { NotesNodeData } from './types'
import { nodeDataEqual } from '../mappers'
import { CanvasNodeKind } from '../canvasKinds'
import { useProtocolHighlight } from '../useProtocolHighlight'
import { getNodeHighlightStyles } from '../nodeHighlightStyles'

function NotesNodeComponent({ id, data, selected }: NodeProps) {
  const theme = useTheme()
  const nodeData = data as NotesNodeData
  const highlightMode = useProtocolHighlight(
    CanvasNodeKind.NOTES,   // New kind — see Part 4A
    id,
    nodeData.protocolStepId,
  )
  const accentColor = NOTES_NODE.ACCENT_COLOR
  const [hovered, setHovered] = useState(false)
  const { containerRef, scaleFactor } = useNodeScale()
  const highlight = getNodeHighlightStyles({
    selected: selected === true,
    accentColor,
    highlightMode,
    themeMode: theme.palette.mode,
    variant: 'resizable',
  })

  return (
    <Box
      ref={containerRef}
      onMouseEnter={() => { setHovered(true) }}
      onMouseLeave={() => { setHovered(false) }}
      sx={{
        width: '100%',
        height: '100%',
        borderRadius: '12px',
        backgroundColor:
          theme.palette.mode === 'light'
            ? theme.palette.custom.cavityBg
            : 'background.paper',
        border: 2,
        borderColor: highlight.borderColor,
        boxShadow: highlight.boxShadow,
        transition: 'border-color 150ms ease, box-shadow 150ms ease',
        overflow: 'hidden',
        display: 'flex',
        flexDirection: 'column',
        cursor: 'default',
      }}
    >
      {/* Input handle — receives edge from parent step */}
      <CanvasHandle
        type="target"
        position={Position.Bottom}
        id="notes-input"
        color={accentColor}
        variant="passive"
      />

      <NodeResizer
        isVisible={hovered || selected === true}
        minWidth={NOTES_NODE.MIN_WIDTH}
        minHeight={NOTES_NODE.MIN_HEIGHT}
        maxWidth={NOTES_NODE.MAX_WIDTH}
        maxHeight={NOTES_NODE.MAX_HEIGHT}
        lineStyle={{ borderColor: 'transparent', borderWidth: 0 }}
        handleStyle={{
          width: 10,
          height: 10,
          borderRadius: 2,
          backgroundColor: accentColor,
          borderColor: accentColor,
          opacity: 0.6,
        }}
      />

      {/* Zoomed inner container */}
      <Box
        sx={{
          flex: 1,
          minHeight: 0,
          display: 'flex',
          flexDirection: 'column',
          overflow: 'hidden',
          zoom: scaleFactor,
        }}
      >
        {/* Header — draggable */}
        <Box
          sx={{
            height: NOTES_NODE.HEADER_HEIGHT,
            overflow: 'hidden',
            borderBottom: 1,
            borderColor: 'divider',
            display: 'flex',
            alignItems: 'center',
            backgroundColor: theme.palette.custom.bgHeader,
            flexShrink: 0,
            cursor: 'grab',
            '&:active': { cursor: 'grabbing' },
          }}
        >
          <NotesNodeHeader
            stepName={nodeData.stepName}
            accentColor={accentColor}
          />
        </Box>

        {/* Content — read-only markdown */}
        <Box
          className="nowheel nodrag nopan"
          sx={{ flex: 1, minHeight: 0, overflow: 'hidden', position: 'relative' }}
        >
          <NotesNodeContent
            content={nodeData.content}
            accentColor={accentColor}
          />
        </Box>
      </Box>
    </Box>
  )
}

const notesNodeEqual = (prev: NodeProps, next: NodeProps): boolean =>
  prev.selected === next.selected &&
  prev.id === next.id &&
  nodeDataEqual(prev.data, next.data)

const NotesNode = memo(NotesNodeComponent, notesNodeEqual)

export { NotesNode }
```

### 3F. Barrel Export

**File:** `frontend/src/components/canvas/NotesNode/index.ts` (new)

```typescript
export { NotesNode } from './NotesNode'
export { NOTES_NODE } from './constants'
export type { NotesNodeData } from './types'
```

---

## Part 4: Frontend — Canvas Integration

### 4A. Canvas Node Kind

**File:** `frontend/src/components/canvas/canvasKinds/index.ts`

Add `NOTES` to the `CanvasNodeKind` enum:

```typescript
const CanvasNodeKind = {
  CONTEXT: 'context',
  DOCUMENT: 'document',
  NOTES: 'notes',       // NEW
  PROTOCOL: 'protocol',
  STEP: 'step',
} as const
```

Add `'notes'` to `HOVER_ELIGIBLE_KINDS` so the notes node participates in protocol group hover highlighting:

```typescript
const HOVER_ELIGIBLE_KINDS = Collections.toSet<CanvasNodeKind>(['context', 'document', 'notes'])
```

### 4B. Node Type Registration

**File:** `frontend/src/components/canvas/nodeTypes.ts`

```typescript
import { NotesNode } from './NotesNode'

const nodeTypes: NodeTypes = {
  stepNode: StepNode,
  dynamicNode: DynamicNode,
  contextNode: ContextNode,
  documentNode: DocumentNode,
  notesNode: NotesNode,       // NEW
}
```

### 4C. Edge Type — Notes Edge

**File:** `frontend/src/components/canvas/NotesEdge.tsx` (new)

Follows `DocumentEdge.tsx` exactly, but red:

```typescript
import { memo } from 'react'
import { getBezierPath } from '@xyflow/react'
import type { EdgeProps } from '@xyflow/react'
import { PIPE } from './constants'
import { PipeEdgePath } from './PipeEdgePath'
import { NOTES_NODE } from './NotesNode'

function NotesEdgeComponent(props: EdgeProps) {
  const { sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition } = props

  const [edgePath] = getBezierPath({
    sourceX,
    sourceY,
    targetX,
    targetY,
    sourcePosition,
    targetPosition,
  })

  return (
    <PipeEdgePath
      edgePath={edgePath}
      color={NOTES_NODE.ACCENT_COLOR}   // Red
      selected={false}
      isProtocol={true}                 // Full opacity, not dimmed
      animationDirection="reverse"       // Flow from step → notes (upward)
      interactionWidth={PIPE.INTERACTION_WIDTH}
    />
  )
}

const NotesEdge = memo(NotesEdgeComponent)

export { NotesEdge }
```

### 4D. Edge Type Registration

**File:** `frontend/src/components/canvas/edgeTypes.ts`

```typescript
import { NotesEdge } from './NotesEdge'

const edgeTypes: EdgeTypes = {
  stepEdge: StepEdge,
  documentEdge: DocumentEdge,
  notesEdge: NotesEdge,       // NEW
}
```

### 4E. Lookups — Notes Data

**File:** `frontend/src/components/canvas/mappers/types.ts`

Add to `StepNodeLookups`:

```typescript
type StepNodeLookups = {
  // ... existing fields ...
  notesByStep: Readonly<Record<string, string>>  // step_id → notes content
}
```

**File:** `frontend/src/components/canvas/useCanvasLookups.ts`

Pass `notesByStep` from the workflowStore into the lookups object. Add it to the hook's inputs:

```typescript
// In useCanvasLookups:
const notesByStep = useStore(workflowStore.store, workflowStore.selectNotesByStep)

return useMemo(() => ({
  lookups: {
    // ... existing ...
    notesByStep,
  },
  // ...
}), [/* ... existing deps ..., notesByStep */])
```

### 4F. Node Auto-Generation

**File:** `frontend/src/components/canvas/mappers/nodes.ts`

After the document node generation block (lines 107-138), add notes node generation:

```typescript
import { NOTES_NODE } from '../NotesNode'
import type { NotesNodeData } from '../NotesNode'

// ... inside toRFNodes, after documentNodes generation ...

// Auto-generate notes nodes for steps that have assistant notes
const notesNodes: Node[] = []
for (const step of steps) {
  // Skip context nodes — they don't have assistants
  if (step.execution_mode === 'context') continue

  const notes = lookups.notesByStep[step.id]
  if (!notes) continue  // No notes → no node

  const notesData: NotesNodeData = {
    kind: CanvasNodeKind.NOTES,
    label: 'Agent Notes',
    stepName: step.name ?? 'Step',
    content: notes,
    protocolStepId: step.id,
  }
  notesNodes.push({
    id: `notes-${step.id}`,
    type: 'notesNode',
    position: {
      // Position to the LEFT of the parent step, offset upward
      x: (step.position_x ?? 0) - NOTES_NODE.DEFAULT_WIDTH - 40,
      y: (step.position_y ?? 0),
    },
    style: {
      width: NOTES_NODE.DEFAULT_WIDTH,
      height: NOTES_NODE.DEFAULT_HEIGHT,
    },
    draggable: true,
    connectable: false,  // Cannot connect to/from notes nodes
    data: notesData,
  })
}

return [...stepNodes, ...documentNodes, ...notesNodes]
```

**Positioning:** The notes node is placed to the LEFT of the parent step (`x - width - 40px`). DocumentNodes go ABOVE. This avoids overlap. If both exist, the notes node is to the left and the document nodes are above.

**Appearance trigger:** The node appears only when `notesByStep[step.id]` exists (i.e., the assistant has called `update_notes` at least once). Before any notes, no node is shown.

### 4G. Edge Auto-Generation

**File:** `frontend/src/components/canvas/mappers/edges.ts`

Add a new function alongside `toDocumentEdges`:

```typescript
import { NOTES_NODE } from '../NotesNode'

const toNotesEdges = (steps: WorkflowStep[], lookups: StepNodeLookups): Edge[] => {
  const edges: Edge[] = []
  for (const step of steps) {
    if (step.execution_mode === 'context') continue

    const notes = lookups.notesByStep[step.id]
    if (!notes) continue

    edges.push({
      id: `notes-edge-${step.id}`,
      type: 'notesEdge',
      source: step.id,
      sourceHandle: 'notes',          // New handle on the parent step
      target: `notes-${step.id}`,
      targetHandle: 'notes-input',
      selectable: false,
      deletable: false,
    })
  }
  return edges
}

export { toRFEdges, toDocumentEdges, toNotesEdges }
```

### 4H. Source Handle on DynamicNode

**File:** `frontend/src/components/canvas/DynamicNode/DynamicNode.tsx`

The parent DynamicNode needs a source handle for the notes edge. Add a `'notes'` handle similar to how documenter steps have a `'documents'` handle. Place it on the LEFT side of the node:

```typescript
{/* Notes handle — LEFT side, only when notes exist */}
<CanvasHandle
  type="source"
  position={Position.Left}
  id="notes"
  color={NOTES_NODE.ACCENT_COLOR}
  variant="passive"
/>
```

Check how the `'documents'` handle is conditionally rendered on DocumenterNode. The notes handle should always be present on DynamicNodes (it's invisible when no edge connects to it, so no visual cost). Alternatively, conditionally render it only when `notesByStep[stepId]` exists — but the unconditional approach is simpler since unused handles are invisible.

### 4I. Canvas Integration

**File:** `frontend/src/components/canvas/WorkflowCanvas.tsx`

**Import and merge notes edges:**

```typescript
import { toRFNodes, toRFEdges, toDocumentEdges, toNotesEdges } from './mappers'

// Line 87 — add toNotesEdges to the edge merge:
const rfEdges = useMemo(
  () => [
    ...toRFEdges(edges, protocolGroups, protocolsByStepLookup, steps),
    ...toDocumentEdges(steps, lookups),
    ...toNotesEdges(steps, lookups),   // NEW
  ],
  [edges, protocolGroups, protocolsByStepLookup, steps, lookups],
)
```

**Deletion protection — skip auto-generated notes nodes and edges:**

```typescript
// onNodesDelete (line 152):
const onNodesDelete: OnNodesDelete = useCallback((deleted) => {
  for (const node of deleted) {
    if (node.id.startsWith('doc-artifact-')) continue
    if (node.id.startsWith('notes-')) continue        // NEW
    void workflowStore.deleteStep(node.id)
  }
}, [])

// onEdgesDelete (line 160):
const onEdgesDelete: OnEdgesDelete = useCallback((deleted) => {
  for (const edge of deleted) {
    if (edge.id.startsWith('doc-edge-')) continue
    if (edge.id.startsWith('notes-edge-')) continue   // NEW
    void workflowStore.removeEdge(edge.id)
  }
}, [])
```

**Connection validation — block connections from notes handle:**

```typescript
// isValidConnection (line 117):
const isValidConnection = useCallback(
  (connection: Connection) => {
    if (connection.sourceHandle === 'documents') return false
    if (connection.sourceHandle === 'notes') return false    // NEW
    // ... rest unchanged
  },
  [stepsById],
)
```

**Share mode — notes nodes are non-selectable:**

```typescript
// onNodeClick (line 208):
const onNodeClick = useCallback((_event: React.MouseEvent, node: { id: string }) => {
  if (shareActive) {
    if (node.id.startsWith('doc-artifact-')) return
    if (node.id.startsWith('notes-')) return           // NEW
    shareStore.commitShare(node.id)
    return
  }
  setContextMenu(null)
}, [])
```

**Fetch notes on canvas mount:**

```typescript
// Add alongside the existing fetch effects (lines 54-66):
const fetchedNotesRef = useRef(false)

useEffect(() => {
  const activeId = workflowStore.store.getState().activeWorkflowId
  if (activeId && !fetchedNotesRef.current) {
    fetchedNotesRef.current = true
    void workflowStore.fetchAllNotes(activeId)
  }
}, [steps])  // Trigger when steps load
```

---

## Part 5: Visual Behavior Summary

### What the User Sees

1. **No notes yet:** No notes node on canvas. The step looks normal.

2. **Assistant calls `update_notes` for the first time:**
   - Backend broadcasts `assistant_notes_updated` via WS
   - Frontend store updates `notesByStep[stepId]`
   - `toRFNodes` generates a new `notesNode` to the LEFT of the parent step
   - `toNotesEdges` generates a red flowing edge from the step to the notes node
   - The notes node fades in with the notes content rendered as markdown

3. **Assistant updates notes again:**
   - WS event arrives with new content
   - Store updates → node re-renders with new content
   - No position change, no flicker — just content swap

4. **Visual appearance:**
   - Red border (`#f85149`)
   - Red animated flowing edge (same pipe animation as document edges, but red)
   - Header: sticky note icon + "Agent Notes" title + step name subtitle + red "Notes" badge
   - Body: read-only markdown rendering of the notes content
   - Resizable (drag corners), draggable (drag header)
   - Cannot be deleted, disconnected, or connected to other nodes

### Example Layout

```
                    [Doc: API Spec]    [Doc: README]
                         |                  |
                         └────────┬─────────┘
                                  |
[Agent Notes] ─── red ─── [Security Scanner (task_force)] ─── blue ─── [Next Step]
                                  |
                         orange   |
                                  |
                    [Doc: Report]
```

---

## Implementation Order

1. **Backend WS event** (Part 1A-B) — add variant, wire broadcast
2. **Backend REST endpoint** (Part 1C) — batch notes fetch
3. **Frontend WS types + store** (Part 2) — event constant, store state, handler, fetch action
4. **Notes Node component** (Part 3) — constants, types, header, content, main component
5. **Canvas integration** (Part 4) — node kind, registration, edge, mapper, handles, canvas wiring

Steps 1-2 are backend. Steps 3-5 are frontend. They can be developed in parallel once Part A's database + tool handler are in place.

---

## Testing

### Backend
- **Unit test:** `update_notes` tool call produces `AssistantNotesUpdated` WS event with correct `step_id` and `content`
- **Unit test:** `AssistantNotesUpdated` serializes to wire format with `event: "assistant_notes_updated"` and data containing `step_id`, `content`
- **Unit test:** Batch notes endpoint returns all notes for a workflow
- **Unit test:** Batch endpoint returns empty array when no notes exist

### Frontend
- **Unit test:** `NotesNodeContent` — renders markdown when content present, placeholder when empty
- **Unit test:** `NotesNodeHeader` — displays step name and "Agent Notes" title
- **Unit test:** `toRFNodes` — generates notes node when `notesByStep` has entry, skips when empty
- **Unit test:** `toRFNodes` — notes node positioned to the left of parent step
- **Unit test:** `toNotesEdges` — generates edge when notes exist, skips context nodes
- **Unit test:** WS handler — `ASSISTANT_NOTES_UPDATED` event updates `notesByStep` in store
- **Integration test:** Notes node appears on canvas after first `update_notes` WS event
- **Integration test:** Notes node content updates when second WS event arrives
- **Integration test:** Notes node cannot be deleted (delete key skips it)
- **Integration test:** Notes edge cannot be deleted
- **Integration test:** Cannot drag a connection from the notes handle
