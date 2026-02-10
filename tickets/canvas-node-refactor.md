# Canvas Node Refactor: Context Nodes, Auto-Appearing Documents, Edge Validation

## Context

The current canvas has three problems:
1. **"Entry" is a confusing name** — it's really a text context injection point. Rename to "Context."
2. **Document nodes are connectable like execution steps** — but they don't execute. They should be read-only visual artifacts that auto-appear when a documenter has document defs configured.
3. **No edge validation** — any node can connect to any other. Only protocols/contexts should be connectable.

User decisions:
- **Multiple Context nodes allowed** (remove the 1-per-workflow limit)
- **Documents auto-appear on the canvas** when a document def is saved on a documenter. They take the documenter's name, are read-only, and have NO handles (not connectable).

---

## Part 1: Database Migration

**File:** `migrations/0020_canvas_node_refactor.sql`

```sql
-- 1. Rename entry → context
UPDATE workflow_steps SET execution_mode = 'context' WHERE execution_mode = 'entry';

-- 2. Remove edges connected to document-mode steps
DELETE FROM workflow_step_edges
WHERE from_step_id IN (SELECT id FROM workflow_steps WHERE execution_mode = 'document')
   OR to_step_id IN (SELECT id FROM workflow_steps WHERE execution_mode = 'document');

-- 3. Remove document-mode steps (no longer connectable nodes)
DELETE FROM workflow_steps WHERE execution_mode = 'document';
```

---

## Part 2: Backend Changes

### 2A. DAG execution pass-through (`src/server/hub/dag/mod.rs` ~line 476)

Replace:
```rust
if step.execution_mode == "entry" || step.execution_mode == "document" {
```
With:
```rust
if step.execution_mode == "context" {
```

Same pass-through logic, just `"context"` only. Document mode is gone.

### 2B. Step creation validation (`src/server/api/workflows/mod.rs` ~line 440-478)

- Replace `"entry"` with `"context"` in auto-wiring block (agent=None, schema=None, reasoning=false)
- **Remove the 1-per-workflow constraint** — delete the `if execution_mode == "entry"` uniqueness check entirely
- Remove `"document"` from the auto-wiring conditions (it's just `"context"` now)

### 2C. Initial input resolution (`src/server/api/workflows/mod.rs` ~line 1033-1041)

Replace:
```rust
steps.iter().find(|s| s.execution_mode == "entry")
```
With:
```rust
steps.iter().find(|s| s.execution_mode == "context")
```

Note: With multiple context nodes, this finds the first one as a fallback for `initial_input`. Each context node's own `prompt_template` is its output regardless.

### 2D. Edge validation (`src/server/api/workflows/mod.rs` ~line 742-765)

Add after ownership check in `add_workflow_edge`:

```rust
let to_step = repo.get_step(req.to_step_id).await?
    .ok_or(AppError::not_found("Target step"))?;

// Context nodes are source-only
if to_step.execution_mode == "context" {
    return Err(AppError::bad_request("Context nodes cannot receive incoming edges"));
}
```

### 2E. No changes needed

- `find_entry_steps()` in `src/server/hub/dag/utils/mod.rs` — uses no-incoming-edges detection, not execution_mode string. Works as-is.
- `step_documents` (M:M reference doc junction) — completely unaffected, different concept.

---

## Part 3: Frontend — Rename DocumentNode to ContextNode

### 3A. New `ContextNode/` directory

Rename `frontend/src/components/canvas/DocumentNode/` → `frontend/src/components/canvas/ContextNode/`

**Files to rename and simplify:**

| Old | New | Changes |
|-----|-----|---------|
| `DocumentNode.tsx` | `ContextNode.tsx` | Remove `mode` branching. Always source-only (no target handle). Always editable. |
| `DocumentNodeContent.tsx` | `ContextNodeContent.tsx` | Remove document-mode branches. Always editable CodeEditor. Remove "Document will be generated" empty state. |
| `DocumentNodeHeader.tsx` | `ContextNodeHeader.tsx` | Badge text: "Document" → "Context" |
| `types.ts` | `types.ts` | Remove `DocumentNodeMode`. Type becomes `ContextNodeData = { label: string; content: string }` |
| `constants.ts` | `constants.ts` | Rename `DOCUMENT_NODE` → `CONTEXT_NODE`. Same dimensions. |

### 3B. Node type registration (`frontend/src/components/canvas/nodeTypes.ts`)

```typescript
import { ContextNode } from './ContextNode'
// ...
const nodeTypes: NodeTypes = {
  stepNode: StepNode,
  documenterNode: DocumenterNode,
  contextNode: ContextNode,
}
```

### 3C. Mapper update (`frontend/src/components/canvas/mappers.ts`)

Replace the entry/document block (lines 43-62):

```typescript
if (step.execution_mode === 'context') {
  const data: ContextNodeData = {
    label: step.name ?? 'Context',
    content: step.prompt_template,
  }
  return {
    id: step.id,
    type: 'contextNode',
    position: { x: step.position_x ?? 0, y: step.position_y ?? 0 },
    style: { width: CONTEXT_NODE.DEFAULT_WIDTH, height: CONTEXT_NODE.DEFAULT_HEIGHT },
    data,
  }
}
```

Remove import of `DOCUMENT_NODE` and `DocumentNodeData`. Add imports from `ContextNode`.

Update export at bottom: replace `DocumentNodeData` with `ContextNodeData`.

### 3D. Constants update (`frontend/src/components/canvas/constants.ts`)

```typescript
export const STEP_TYPE_COLORS: Record<string, string> = {
  single: '#3b82f6',
  for_each: '#2dd4bf',
  room: '#a78bfa',
  context: '#10b981',   // renamed from 'entry'
  // 'document' removed
}
```

### 3E. Context menu update (`frontend/src/components/canvas/CanvasContextMenu.tsx`)

- Remove `hasEntry` check (line 40) — no longer limited to 1
- Rename `handleAddEntry` → `handleAddContext`: change `execution_mode: 'entry'` to `'context'`, change name to `'Context'`
- Remove `handleAddDocument` function entirely (lines 113-123)
- Remove the "Document" section label and the Document menu item (lines 301-363)
- Change the "Port of Entry" label to "Context" (line 338)
- Remove the `hasEntry` disabled/opacity styling

### 3F. Edge validation in ReactFlow (`frontend/src/components/canvas/WorkflowCanvas.tsx`)

Add `isValidConnection` callback:

```typescript
const isValidConnection = useCallback((connection: Connection) => {
  const steps = workflowStore.selectSteps(workflowStore.store.getState())
  const targetStep = steps.find(s => s.id === connection.target)
  if (!targetStep) return false
  if (targetStep.execution_mode === 'context') return false  // context = source-only
  if (connection.source === connection.target) return false    // no self-loops
  return true
}, [])
```

Pass to `<ReactFlow isValidConnection={isValidConnection} ...>`.

---

## Part 4: Frontend — Auto-Appearing Document Artifacts

When a documenter has document defs configured, those documents appear as read-only visual nodes on the canvas near the documenter. They are NOT connectable.

### 4A. Document artifact node type

Keep the old `DocumentNode` component but repurpose it as `DocumentArtifactNode` — a simpler, read-only node with:
- No handles (no source, no target — not connectable)
- Read-only markdown display of content (or placeholder "Will be generated when workflow runs")
- Name derived from the document def name + documenter name
- Positioned automatically near its parent documenter node (offset right + down)

Register in `nodeTypes.ts`:
```typescript
documentArtifactNode: DocumentArtifactNode,
```

### 4B. Auto-positioning logic

In `mappers.ts`, after mapping all real steps, append document artifact nodes for each documenter's document defs:

```typescript
// For each documenter step, add artifact nodes for its document defs
const documentDefs = lookups.documentDefsByStep ?? new Map()
for (const step of steps) {
  if (step.execution_mode !== 'documenter') continue
  const defs = documentDefs.get(step.id) ?? []
  defs.forEach((def, i) => {
    nodes.push({
      id: `doc-artifact-${def.id}`,
      type: 'documentArtifactNode',
      position: {
        x: (step.position_x ?? 0) + 480,
        y: (step.position_y ?? 0) + (i * 140),
      },
      data: {
        label: def.name,
        documenterName: step.name ?? 'Documenter',
        content: null,  // populated after execution
      },
      draggable: true,
      selectable: false,
      connectable: false,
    })
  })
}
```

### 4C. Store integration

When `workflowStore.createDocumentDef()` or `workflowStore.deleteDocumentDef()` is called, the store already refetches `documentDefsByStep`. The mapper picks up the change and artifact nodes appear/disappear on the next render. No special WebSocket event needed — it's reactive through the store.

### 4D. Lookups extension

Add `documentDefsByStep` to `StepNodeLookups` type in `mappers.ts`:
```typescript
documentDefsByStep: ReadonlyMap<string, ReadonlyArray<{ id: string; name: string }>>
```

Wire this from `workflowStore.selectDocumentDefsByStep` in `WorkflowCanvas.tsx` where lookups are built.

---

## Part 5: Cleanup and Delete Old Files

- Delete `frontend/src/components/canvas/DocumentNode/` directory (replaced by ContextNode + DocumentArtifactNode)
- Remove any remaining `'entry'` or `'document'` string references in frontend
- Run `npx tsc --noEmit && npx eslint .` to verify zero errors/warnings

---

## Files Changed Summary

**Backend (new):**
- `migrations/0020_canvas_node_refactor.sql`

**Backend (modified):**
- `src/server/hub/dag/mod.rs` — execution dispatch (~line 476)
- `src/server/api/workflows/mod.rs` — step creation (~line 440), initial_input (~line 1033), edge validation (~line 742)

**Frontend (new):**
- `frontend/src/components/canvas/ContextNode/` (5 files, adapted from DocumentNode)
- `frontend/src/components/canvas/DocumentArtifactNode/` (new read-only node component)

**Frontend (modified):**
- `frontend/src/components/canvas/mappers.ts` — context mapping + artifact node generation
- `frontend/src/components/canvas/nodeTypes.ts` — register contextNode + documentArtifactNode
- `frontend/src/components/canvas/constants.ts` — color map updates
- `frontend/src/components/canvas/CanvasContextMenu.tsx` — rename entry→context, remove document item
- `frontend/src/components/canvas/WorkflowCanvas.tsx` — add isValidConnection

**Frontend (deleted):**
- `frontend/src/components/canvas/DocumentNode/` (replaced)

---

## Verification

1. `~/.cargo/bin/cargo check && ~/.cargo/bin/cargo test` — backend compiles and tests pass
2. `cd frontend && npx tsc --noEmit && npx eslint .` — frontend compiles with zero warnings
3. Run the app, open a workflow:
   - Right-click canvas: "Context" appears in menu, "Document" is gone
   - Add multiple Context nodes (no limit)
   - Context nodes have source handle only, are editable
   - Add a Documenter node, add document defs via Documents tab → artifact nodes appear on canvas
   - Delete a document def → artifact node disappears
   - Artifact nodes are not connectable (no handles, cannot drag edges from them)
   - Context nodes cannot be edge targets (dragging an edge TO a context node is rejected)
   - Existing workflows with entry nodes still work (migration renamed to context)
