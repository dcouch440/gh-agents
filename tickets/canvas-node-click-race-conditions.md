# Canvas Node Click Race Conditions — Double-Click Bug

## Context

Users intermittently need to click canvas nodes twice before the click registers. Investigation reveals multiple interacting issues in the React Flow integration layer where state updates, re-renders, and event processing compete during click handling. The core problem is that React re-renders triggered by selection state changes can interrupt click event processing, effectively swallowing the first click.

A previous fix (commit `ab13dd4`) addressed the most severe case — `setNodes` clobbering the `selected` property during data-only updates — but several secondary race conditions remain.

**Key decisions:**
- Fix node `memo()` comparators first (highest impact, lowest risk)
- Defer layout-triggering side effects out of the click path
- Stabilize lookup references to prevent unnecessary re-renders
- Each fix is independently shippable and testable

---

## Part 1: Custom `memo()` Comparators on All Node Types

> **Risk:** HIGH — This is the most likely remaining cause of double-click behavior.
> **Effort:** 1-2 hours
> **Dependencies:** None

### Problem

All three node components use `memo()` with default shallow comparison:

- `StepNode.tsx:338`
- `DocumentNode/DocumentNode.tsx:129`
- `DocumenterNode/DocumenterNode.tsx:102`

Default `memo()` compares props by reference (`===`). React Flow recreates the `data` prop object on every render cycle, so even when the underlying values are identical, the reference changes and `memo()` lets the re-render through. If this re-render happens mid-click (between mousedown and mouseup), the component unmounts/remounts and the click event is lost.

### Fix

Add a custom comparator to each node's `memo()` call that checks only the fields that matter:

```tsx
export const StepNode = memo(StepNodeComponent, (prev, next) => {
  return prev.data === next.data && prev.selected === next.selected;
});
```

Apply the same pattern to `DocumentNode` and `DocumenterNode`. If `data` still produces unstable references, deepen the comparison to check individual data fields.

### Verification

- Use React DevTools Profiler to confirm node components do NOT re-render when clicking a different node
- Click 20+ nodes rapidly — every click should register on the first attempt
- Verify no visual regressions (node appearance unchanged)

---

## Part 2: Defer Panel Open During Selection

> **Risk:** MEDIUM — Layout shift during click can cause canvas reflow.
> **Effort:** 30 minutes
> **Dependencies:** None

### Problem

`WorkflowCanvas.tsx:190-195` — When a node is selected, three state updates fire synchronously in the `onSelectionChange` callback:

```typescript
canvasStore.selectSteps(params.nodes.map((n) => n.id));
canvasStore.selectEdges(params.edges.map((e) => e.id));
layoutStore.openRightPanelIfClosed("properties");  // <-- triggers layout shift
```

The panel opening changes the DOM layout, which can cause the canvas container to resize and React Flow to recompute node positions. If this happens before the click event fully propagates, the event target may shift out from under the cursor.

### Fix

Defer the panel open so it doesn't compete with the selection event:

```typescript
const onSelectionChange = useCallback((params: OnSelectionChangeParams) => {
  canvasStore.selectSteps(params.nodes.map((n) => n.id));
  canvasStore.selectEdges(params.edges.map((e) => e.id));
  if (params.nodes.length > 0 || params.edges.length > 0) {
    requestAnimationFrame(() => {
      layoutStore.openRightPanelIfClosed("properties");
    });
  }
}, []);
```

### Verification

- Click a node when the right panel is closed — panel should open smoothly without requiring a second click
- Click nodes rapidly — no dropped clicks
- Panel open animation should still feel instant to the user

---

## Part 3: Stabilize Lookup Map References

> **Risk:** MEDIUM — Unstable references cascade through memoization.
> **Effort:** 1-2 hours
> **Dependencies:** None

### Problem

`WorkflowCanvas.tsx:76-121` builds five lookup maps via `useMemo`:

- `agentLookup` (depends on `agents`)
- `schemaLookup` (depends on `schemas`)
- `stepNameLookup` (depends on `steps`)
- `toolsByAgentLookup` (depends on `agents`, `toolsByAgent`)
- `protocolsByStepLookup` (depends on `stepProtocols`)

These are combined into a `lookups` object at line ~113 and passed into `toRFNodes` at line ~120. If ANY of the five dependencies produces a new reference (even with identical data), the entire `rfNodes` array is recomputed, producing new node objects, which defeats `memo()` on every node component.

The store selectors feeding these dependencies may return new array/object references on every render even when the underlying data hasn't changed.

### Fix

1. Audit each store selector (`useWorkflowStore`, `useAgentStore`, etc.) to ensure they return referentially stable values when data hasn't changed. Use `Object.is` or shallow-equal selectors.
2. Alternatively, memoize the `lookups` object itself with a custom equality check that compares the Maps by size + entry equality rather than reference.
3. As a fallback, move lookup construction into `toRFNodes` and pass raw arrays, letting the mapper build Maps internally (isolates the instability).

### Verification

- Add a `console.count('rfNodes recomputed')` inside the `rfNodes` useMemo — it should NOT fire on every render
- React DevTools Profiler should show stable node components between clicks
- No visual regressions

---

## Part 4: Guard Against External Updates During Interaction

> **Risk:** LOW — Only affects users with active WebSocket data streams.
> **Effort:** 1-2 hours
> **Dependencies:** Parts 1-3 should be done first

### Problem

`WorkflowCanvas.tsx:128-167` — The `setNodes` effect synchronizes React Flow's internal node state with application state. Even though commit `ab13dd4` prevents touching the `selected` property, if `data`, `position`, `type`, or `style` change during a click (e.g., from a WebSocket push updating step status), the node still re-renders.

### Fix

Debounce external data updates while a user interaction is in progress:

1. Track an `isInteracting` ref (set `true` on `onNodeDragStart`, `onSelectionStart`, mousedown; set `false` on corresponding end events)
2. When `isInteracting` is true, queue data updates instead of applying them immediately
3. Flush the queue on interaction end

Alternatively, use React's `useTransition` to mark external data updates as low-priority, allowing click events to process first.

### Verification

- Simulate concurrent WebSocket updates while clicking nodes
- No dropped clicks during active data streaming
- Updates still apply promptly after interaction ends (no visible delay)

---

## Part 5: Context Menu State Cleanup

> **Risk:** LOW — Edge case when right-click precedes left-click.
> **Effort:** 30 minutes
> **Dependencies:** None

### Problem

`WorkflowCanvas.tsx:279-281` — `onNodeClick` only clears the context menu:

```typescript
const onNodeClick = useCallback(() => {
  setContextMenu(null);
}, []);
```

If a user right-clicks (opening context menu), then left-clicks a node, the first left-click is consumed by closing the context menu rather than selecting the node.

### Fix

Close the context menu on `onPaneClick` and `onNodeClick`, and also on any mousedown event on the canvas pane. This ensures the menu is already dismissed before click selection logic runs.

### Verification

- Right-click a node to open context menu, then left-click a different node — should select on first click
- Context menu should dismiss on pane click, node click, and scroll

---

## Files Involved

| File | Role |
|------|------|
| `components/canvas/WorkflowCanvas.tsx` | Main canvas orchestration, event handlers, node sync |
| `components/canvas/StepNode.tsx` | Step node component + memo |
| `components/canvas/DocumentNode/DocumentNode.tsx` | Document node component + memo |
| `components/canvas/DocumenterNode/DocumenterNode.tsx` | Documenter node component + memo |
| `components/canvas/mappers.ts` | `toRFNodes` — builds React Flow node array |
| `components/canvas/CanvasContextMenu.tsx` | Context menu with stopPropagation |
| `stores/canvasStore.ts` | Selection state (selectSteps, selectEdges) |
| `stores/layoutStore.ts` | Panel open/close state |

## Acceptance Criteria

- [ ] Single click reliably selects a node in all tested scenarios
- [ ] Custom `memo()` comparators on StepNode, DocumentNode, DocumenterNode
- [ ] Panel open deferred out of click event path
- [ ] Lookup map references stabilized (rfNodes recomputation only on actual data change)
- [ ] React DevTools Profiler confirms no spurious re-renders on click
- [ ] Context menu → left-click works on first click
- [ ] All existing canvas tests pass
- [ ] `npx tsc --noEmit` and `npx eslint .` pass with zero warnings
