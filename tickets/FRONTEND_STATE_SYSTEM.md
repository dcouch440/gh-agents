# Plan: Frontend State System Architecture

## Context

The Nexor frontend is being rebuilt from scratch. The backend has a complete workflow execution engine (DAG executor with ports, routing, for-each, rooms, cavernous routing) but the frontend has no visual workflow editor — just CRUD via API. We need a state management system that powers a React Flow-based workflow builder capable of handling 200+ nodes with fluid animations, inline editing, and live execution visualization.

**Decisions made:**
- Custom store built on `useSyncExternalStore` (no external state libraries)
- Split ownership: React Flow owns visual state (positions, viewport, selection during drag), our stores own business data
- Full undo/redo via command pattern
- Framer Motion for animations
- Normalized collections (`Map<string, T>`) for O(1) lookups at scale
- `@xyflow/react` v12+ for the canvas

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────┐
│                    React Components                  │
│  (subscribe to stores via selectors — only re-render │
│   when their specific slice changes)                 │
├──────────┬──────────┬───────────┬──────────┬────────┤
│ Workflow │  Canvas  │ Execution │ Catalog  │   UI   │
│  Store   │  Store   │   Store   │  Store   │ Store  │
├──────────┴──────────┴───────────┴──────────┴────────┤
│              createStore() + useStore()               │
│         (useSyncExternalStore + selectors)            │
├──────────────────────┬──────────────────────────────┤
│    History Store     │      API Sync Layer           │
│  (undo/redo cmds)    │  (debounced persistence)      │
├──────────────────────┴──────────────────────────────┤
│              React Flow (visual layer)               │
│  owns: drag, viewport, selection, edge routing       │
│  bridge: onNodeDragStop → store, onConnect → store   │
└─────────────────────────────────────────────────────┘
```

**Data flow:**
1. User drags node → React Flow handles drag animation → `onNodeDragStop` → store persists final position
2. User edits step config → store updates → React Flow node re-renders (via `React.memo` + data prop change)
3. Execution starts → WebSocket events → execution store updates → node status overlays re-render
4. User clicks undo → history store pops command → command.undo() mutates workflow store → RF re-renders

---

## File Structure

```
frontend/src/
├── stores/
│   ├── core/
│   │   ├── createStore.ts          # Store factory (useSyncExternalStore)
│   │   ├── useStore.ts             # Selector hook + useStoreActions
│   │   ├── batch.ts                # Batch update utility
│   │   ├── shallow.ts              # Shallow equality comparator
│   │   ├── normalized.ts           # NormalizedMap<T> with memoized arrays
│   │   ├── middleware.ts           # Logging, devtools, persistence middleware
│   │   ├── types.ts                # Core store types
│   │   └── index.ts
│   │
│   ├── workflow/
│   │   ├── workflowStore.ts        # Steps, edges, ports, routing rules, documents
│   │   ├── selectors.ts            # Memoized selectors (stepById, stepArray, etc.)
│   │   ├── actions.ts              # Mutation functions (addStep, updateStep, etc.)
│   │   ├── types.ts                # WorkflowState, StepData, PortData, etc.
│   │   ├── tests.ts                # Store unit tests
│   │   └── index.ts
│   │
│   ├── canvas/
│   │   ├── canvasStore.ts          # Panel state, drag-from-palette, interaction mode
│   │   ├── selectors.ts
│   │   ├── types.ts
│   │   └── index.ts
│   │
│   ├── execution/
│   │   ├── executionStore.ts       # Run state, per-step status, streaming, envelopes
│   │   ├── selectors.ts
│   │   ├── wsHandlers.ts           # WebSocket event → store update mapping
│   │   ├── types.ts
│   │   └── index.ts
│   │
│   ├── catalog/
│   │   ├── catalogStore.ts         # Agents, schemas, templates, documents, tools
│   │   ├── selectors.ts
│   │   ├── loaders.ts              # API fetch + hydrate functions
│   │   ├── types.ts
│   │   └── index.ts
│   │
│   ├── ui/
│   │   ├── uiStore.ts              # Theme, sidebar, toasts, modals, command palette
│   │   ├── selectors.ts
│   │   ├── types.ts
│   │   └── index.ts
│   │
│   ├── history/
│   │   ├── historyStore.ts         # Undo/redo stack
│   │   ├── commands.ts             # Command factories (moveNode, addStep, etc.)
│   │   ├── types.ts                # Command type, HistoryState
│   │   └── index.ts
│   │
│   ├── sync/
│   │   ├── apiSync.ts              # Debounced API persistence
│   │   ├── optimistic.ts           # Optimistic update + rollback utilities
│   │   ├── conflict.ts             # Version-based conflict detection
│   │   └── index.ts
│   │
│   └── index.ts                    # Barrel: all store + hook exports
│
├── bridge/
│   ├── transforms.ts               # WorkflowStep ↔ RF Node, Edge ↔ RF Edge
│   ├── useFlowSync.ts              # Hook: RF callbacks → store mutations
│   ├── useFlowNodes.ts             # Hook: store → RF nodes (memoized)
│   ├── useFlowEdges.ts             # Hook: store → RF edges (memoized)
│   ├── nodeTypes.ts                # Custom node type registry
│   ├── edgeTypes.ts                # Custom edge type registry
│   ├── types.ts                    # StepNodeData, EdgeData, etc.
│   └── index.ts
│
├── types/
│   ├── workflow.ts                 # Updated: full step/edge/port types matching backend
│   ├── execution.ts                # Execution status, envelope, metadata
│   ├── agent.ts                    # Agent type
│   └── ... (existing type files)
```

---

## Constants

All string literals live in const objects — no magic strings in stores or components. Follows the existing `ACTION`, `WS_CHANNEL` pattern.

```typescript
// stores/constants.ts

// Step execution modes (maps to backend WorkflowStepRow.execution_mode)
export const STEP_MODE = {
  SINGLE: 'single',
  FOR_EACH: 'for_each',
  ROOM: 'room',
} as const

export type StepMode = (typeof STEP_MODE)[keyof typeof STEP_MODE]

// Panel kinds for the canvas editor sidebar
export const PANEL = {
  CLOSED: 'closed',
  STEP_CONFIG: 'step-config',
  EDGE_CONFIG: 'edge-config',
  AGENT_PICKER: 'agent-picker',
  PORT_EDITOR: 'port-editor',
  ROUTING_RULES: 'routing-rules',
  DOCUMENT_PICKER: 'document-picker',
  EXECUTION_DETAIL: 'execution-detail',
} as const

export type PanelKind = (typeof PANEL)[keyof typeof PANEL]

// Canvas interaction modes
export const INTERACTION = {
  SELECT: 'select',
  CONNECT: 'connect',
  PAN: 'pan',
} as const

export type InteractionMode = (typeof INTERACTION)[keyof typeof INTERACTION]

// Drag item kinds (palette → canvas)
export const DRAG_KIND = {
  NEW_STEP: 'new-step',
} as const

export type DragKind = (typeof DRAG_KIND)[keyof typeof DRAG_KIND]

// Step execution status (live execution state)
export const STEP_STATUS = {
  IDLE: 'idle',
  PENDING: 'pending',
  RUNNING: 'running',
  SUCCESS: 'success',
  ERROR: 'error',
  SKIPPED: 'skipped',
  PAUSED: 'paused',
} as const

export type StepExecutionStatus = (typeof STEP_STATUS)[keyof typeof STEP_STATUS]

// React Flow custom node type keys
export const NODE_TYPE = {
  SINGLE_STEP: 'singleStep',
  FOR_EACH_STEP: 'forEachStep',
  ROOM_STEP: 'roomStep',
} as const

export type NodeTypeKey = (typeof NODE_TYPE)[keyof typeof NODE_TYPE]

// React Flow custom edge type keys
export const EDGE_TYPE = {
  DATA_FLOW: 'dataFlow',
  CONDITIONAL: 'conditional',
} as const

export type EdgeTypeKey = (typeof EDGE_TYPE)[keyof typeof EDGE_TYPE]

// Toast severity levels
export const TOAST_SEVERITY = {
  INFO: 'info',
  SUCCESS: 'success',
  WARNING: 'warning',
  ERROR: 'error',
} as const

export type ToastSeverity = (typeof TOAST_SEVERITY)[keyof typeof TOAST_SEVERITY]

// Theme modes
export const THEME = {
  LIGHT: 'light',
  DARK: 'dark',
} as const

export type ThemeMode = (typeof THEME)[keyof typeof THEME]

// Command types for undo/redo history
export const COMMAND = {
  MOVE_NODES: 'MOVE_NODES',
  ADD_STEP: 'ADD_STEP',
  REMOVE_STEP: 'REMOVE_STEP',
  UPDATE_STEP_CONFIG: 'UPDATE_STEP_CONFIG',
  ADD_EDGE: 'ADD_EDGE',
  REMOVE_EDGE: 'REMOVE_EDGE',
  UPDATE_PORTS: 'UPDATE_PORTS',
  UPDATE_ROUTING_RULES: 'UPDATE_ROUTING_RULES',
} as const

export type CommandType = (typeof COMMAND)[keyof typeof COMMAND]

// Collection execution modes
export const COLLECTION_MODE = {
  PARALLEL: 'parallel',
  SEQUENTIAL: 'sequential',
} as const

export type CollectionMode = (typeof COLLECTION_MODE)[keyof typeof COLLECTION_MODE]

// Routing modes
export const ROUTING_MODE = {
  LABEL: 'label',
} as const

// Port types
export const PORT_TYPE = {
  STRING: 'string',
  JSON: 'json',
  ARRAY: 'array',
  NUMBER: 'number',
  BOOLEAN: 'boolean',
} as const

export type PortType = (typeof PORT_TYPE)[keyof typeof PORT_TYPE]

// Sync status for API persistence layer
export const SYNC_STATUS = {
  IDLE: 'idle',
  SYNCING: 'syncing',
  ERROR: 'error',
  CONFLICT: 'conflict',
} as const

export type SyncStatus = (typeof SYNC_STATUS)[keyof typeof SYNC_STATUS]

// Auto-save debounce timing
export const SYNC_DEBOUNCE_MS = 1500

// Undo/redo stack max size
export const HISTORY_MAX_SIZE = 100
```

All domain types then reference these constants:
```typescript
// Example: CanvasStoreState uses PanelKind not raw strings
type CanvasStoreState = {
  panel: PanelKind           // typeof PANEL values
  interactionMode: InteractionMode  // typeof INTERACTION values
  // ...
}
```

---

## Phase 1: Store Infrastructure

**Goal:** Build the core `createStore` factory and `useStore` hook that everything else depends on.

### 1A: `createStore<T>` — Store Factory

```typescript
// stores/core/types.ts

type Listener = () => void

type SetState<T> = {
  (partial: Partial<T>): void
  (updater: (state: T) => Partial<T>): void
}

type GetState<T> = () => T

type Subscribe = (listener: Listener) => () => void

type StoreApi<T> = {
  getState: GetState<T>
  setState: SetState<T>
  subscribe: Subscribe
  destroy: () => void
}

type StateCreator<T> = (
  set: SetState<T>,
  get: GetState<T>,
) => T
```

```typescript
// stores/core/createStore.ts

const createStore = <T>(creator: StateCreator<T>): StoreApi<T> => {
  let state: T
  const listeners = new Set<Listener>()

  const getState: GetState<T> = () => state

  const setState: SetState<T> = (partial) => {
    const nextPartial = typeof partial === 'function'
      ? (partial as (s: T) => Partial<T>)(state)
      : partial
    const prev = state
    state = Object.assign({}, state, nextPartial)
    if (state !== prev) {
      listeners.forEach((listener) => listener())
    }
  }

  const subscribe: Subscribe = (listener) => {
    listeners.add(listener)
    return () => { listeners.delete(listener) }
  }

  const destroy = () => { listeners.clear() }

  state = creator(setState, getState)

  return { getState, setState, subscribe, destroy }
}
```

### 1B: `useStore` — Selector Hook

```typescript
// stores/core/useStore.ts

const useStore = <T, S>(
  store: StoreApi<T>,
  selector: (state: T) => S,
  equalityFn: (a: S, b: S) => boolean = Object.is,
): S => {
  // Cache the latest selected value for equality comparison
  const selectedRef = useRef<S>(selector(store.getState()))
  const selectorRef = useRef(selector)
  const equalityRef = useRef(equalityFn)
  selectorRef.current = selector
  equalityRef.current = equalityFn

  const getSnapshot = useCallback(() => {
    const next = selectorRef.current(store.getState())
    if (equalityRef.current(selectedRef.current, next)) {
      return selectedRef.current  // Return same reference → no re-render
    }
    selectedRef.current = next
    return next
  }, [store])

  return useSyncExternalStore(store.subscribe, getSnapshot)
}

// Convenience: get actions without subscribing to state changes
const useStoreActions = <T, A>(
  store: StoreApi<T>,
  actionsSelector: (set: SetState<T>, get: GetState<T>) => A,
): A => {
  return useMemo(
    () => actionsSelector(store.setState, store.getState),
    [store, actionsSelector],
  )
}
```

### 1C: `batch` — Batch Updates

Multiple `setState` calls → single listener notification:

```typescript
// stores/core/batch.ts

let batchDepth = 0
let pendingStores = new Set<StoreApi<unknown>>()

const batch = (fn: () => void): void => {
  batchDepth++
  try {
    fn()
  } finally {
    batchDepth--
    if (batchDepth === 0) {
      const stores = pendingStores
      pendingStores = new Set()
      stores.forEach((store) => {
        // Notify listeners once per store
        store.subscribe // trigger via internal mechanism
      })
    }
  }
}
```

Implementation note: batch works by patching `setState` to defer listener notification while `batchDepth > 0`. The createStore's setState checks batchDepth before notifying.

### 1D: `shallow` — Shallow Equality

```typescript
// stores/core/shallow.ts

const shallow = <T>(a: T, b: T): boolean => {
  if (Object.is(a, b)) return true
  if (typeof a !== 'object' || typeof b !== 'object') return false
  if (a === null || b === null) return false

  const keysA = Object.keys(a)
  const keysB = Object.keys(b)
  if (keysA.length !== keysB.length) return false

  return keysA.every(
    (key) => Object.hasOwn(b, key) && Object.is(a[key as keyof T], b[key as keyof T]),
  )
}
```

### 1E: `NormalizedMap<T>` — Memoized Normalized Collection

This is the key performance primitive for 200+ nodes:

```typescript
// stores/core/normalized.ts

type NormalizedMap<T> = {
  byId: Map<string, T>
  // Memoized array — only recalculated when byId changes
  _array: T[] | null
  _version: number
}

const createNormalizedMap = <T>(): NormalizedMap<T> => ({
  byId: new Map(),
  _array: null,
  _version: 0,
})

// Returns memoized array — same reference if map hasn't changed
const toArray = <T>(nm: NormalizedMap<T>): T[] => {
  if (nm._array === null) {
    nm._array = Array.from(nm.byId.values())
  }
  return nm._array
}

// Mutation helpers — invalidate _array cache
const nmSet = <T>(nm: NormalizedMap<T>, id: string, item: T): NormalizedMap<T> => ({
  byId: new Map(nm.byId).set(id, item),
  _array: null,
  _version: nm._version + 1,
})

const nmDelete = <T>(nm: NormalizedMap<T>, id: string): NormalizedMap<T> => {
  const next = new Map(nm.byId)
  next.delete(id)
  return { byId: next, _array: null, _version: nm._version + 1 }
}

const nmFromArray = <T>(items: T[], getId: (item: T) => string): NormalizedMap<T> => ({
  byId: new Map(items.map((item) => [getId(item), item])),
  _array: items, // Already have the array
  _version: 0,
})
```

### 1F: Middleware

```typescript
// stores/core/middleware.ts

// Logging middleware (dev only)
const withLogging = <T>(creator: StateCreator<T>): StateCreator<T> =>
  (set, get) => {
    const loggedSet: SetState<T> = (partial) => {
      const prev = get()
      set(partial)
      if (import.meta.env.DEV) {
        console.groupCollapsed('[store] state update')
        console.log('prev:', prev)
        console.log('next:', get())
        console.groupEnd()
      }
    }
    return creator(loggedSet, get)
  }

// localStorage persistence middleware
const withPersistence = <T>(
  key: string,
  pick: (state: T) => Partial<T>,
): (creator: StateCreator<T>) => StateCreator<T> =>
  (creator) => (set, get) => {
    const stored = localStorage.getItem(key)
    const initial = creator(set, get)
    const hydrated = stored
      ? { ...initial, ...JSON.parse(stored) }
      : initial

    // Subscribe to persist on change (debounced)
    let timeout: ReturnType<typeof setTimeout>
    const originalSet = set
    const persistSet: SetState<T> = (partial) => {
      originalSet(partial)
      clearTimeout(timeout)
      timeout = setTimeout(() => {
        localStorage.setItem(key, JSON.stringify(pick(get())))
      }, 500)
    }

    return hydrated
  }
```

### Phase 1 Files
- `stores/constants.ts` — all string constants + derived types
- `stores/core/types.ts`
- `stores/core/createStore.ts`
- `stores/core/useStore.ts`
- `stores/core/batch.ts`
- `stores/core/shallow.ts`
- `stores/core/normalized.ts`
- `stores/core/middleware.ts`
- `stores/core/index.ts`

### Phase 1 Tests
- createStore: basic get/set, listener notification, unsubscribe
- useStore: selector isolation (component A doesn't re-render when component B's slice changes)
- batch: multiple setState → single notification
- shallow: equality edge cases
- NormalizedMap: set/delete/toArray memoization, version tracking

---

## Phase 2: Domain Stores

### 2A: WorkflowStore

The central store for the workflow being edited.

```typescript
// stores/workflow/types.ts

type WorkflowStoreState = {
  // Current workflow metadata
  workflowId: string | null
  name: string
  description: string | null

  // Normalized collections
  steps: NormalizedMap<WorkflowStep>
  edges: NormalizedMap<WorkflowStepEdge>

  // Per-step nested data (keyed by step ID)
  inputPorts: Map<string, StepInput[]>
  outputPorts: Map<string, StepOutput[]>
  routingRules: Map<string, RoutingRule[]>
  stepDocuments: Map<string, StepDocument[]>

  // Loading/error
  loading: boolean
  error: string | null
  dirty: boolean        // Has unsaved changes
  version: number       // Workflow version for conflict detection
}
```

**Key selectors:**
```typescript
// stores/workflow/selectors.ts

// Per-node selector — each node component uses this
const selectStepById = (id: string) =>
  (state: WorkflowStoreState) => state.steps.byId.get(id) ?? null

// Array selectors (memoized via NormalizedMap)
const selectStepArray = (state: WorkflowStoreState) => toArray(state.steps)
const selectEdgeArray = (state: WorkflowStoreState) => toArray(state.edges)

// Derived: steps with their ports (for node rendering)
const selectStepWithPorts = (id: string) => (state: WorkflowStoreState) => {
  const step = state.steps.byId.get(id)
  if (!step) return null
  return {
    step,
    inputs: state.inputPorts.get(id) ?? [],
    outputs: state.outputPorts.get(id) ?? [],
    rules: state.routingRules.get(id) ?? [],
  }
}

// Topology: entry steps (no incoming edges)
const selectEntryStepIds = (state: WorkflowStoreState): string[] => {
  const hasIncoming = new Set<string>()
  for (const edge of state.edges.byId.values()) {
    hasIncoming.add(edge.to_step_id)
  }
  return toArray(state.steps)
    .filter((s) => !hasIncoming.has(s.id))
    .map((s) => s.id)
}
```

**Actions (mutation functions):**
```typescript
// stores/workflow/actions.ts
// These are called by command factories (for undo support)
// and directly for non-undoable operations (load, sync)

const workflowActions = (set: SetState<WorkflowStoreState>, get: GetState<WorkflowStoreState>) => ({
  // Hydration (from API, not undoable)
  hydrate: (workflow, steps, edges, ports, rules, docs) => { ... },
  clear: () => { ... },

  // Step mutations
  addStep: (step: WorkflowStep) => set((s) => ({
    steps: nmSet(s.steps, step.id, step),
    dirty: true,
  })),
  updateStep: (id: string, patch: Partial<WorkflowStep>) => set((s) => {
    const existing = s.steps.byId.get(id)
    if (!existing) return {}
    return { steps: nmSet(s.steps, id, { ...existing, ...patch }), dirty: true }
  }),
  removeStep: (id: string) => set((s) => ({
    steps: nmDelete(s.steps, id),
    // Also remove edges connected to this step
    edges: removeEdgesForStep(s.edges, id),
    // Clean up nested data
    inputPorts: mapDelete(s.inputPorts, id),
    outputPorts: mapDelete(s.outputPorts, id),
    routingRules: mapDelete(s.routingRules, id),
    stepDocuments: mapDelete(s.stepDocuments, id),
    dirty: true,
  })),

  // Edge mutations
  addEdge: (edge: WorkflowStepEdge) => set((s) => ({
    edges: nmSet(s.edges, edge.id, edge),
    dirty: true,
  })),
  removeEdge: (id: string) => set((s) => ({
    edges: nmDelete(s.edges, id),
    dirty: true,
  })),

  // Port mutations
  setInputPorts: (stepId: string, ports: StepInput[]) => set((s) => ({
    inputPorts: new Map(s.inputPorts).set(stepId, ports),
    dirty: true,
  })),
  setOutputPorts: (stepId: string, ports: StepOutput[]) => set((s) => ({
    outputPorts: new Map(s.outputPorts).set(stepId, ports),
    dirty: true,
  })),

  // Position update (from React Flow drag end)
  updateStepPosition: (id: string, x: number, y: number) => set((s) => {
    const existing = s.steps.byId.get(id)
    if (!existing) return {}
    return { steps: nmSet(s.steps, id, { ...existing, position_x: x, position_y: y }), dirty: true }
  }),

  // Batch position update (multi-select drag)
  updateStepPositions: (updates: Array<{ id: string; x: number; y: number }>) => set((s) => {
    let next = s.steps
    for (const u of updates) {
      const existing = next.byId.get(u.id)
      if (existing) {
        next = nmSet(next, u.id, { ...existing, position_x: u.x, position_y: u.y })
      }
    }
    return { steps: next, dirty: true }
  }),

  markClean: () => set({ dirty: false }),
})
```

### 2B: CanvasStore

UI interaction state for the editor (NOT positions — RF owns those).

```typescript
// stores/canvas/types.ts
// PanelKind, InteractionMode, DragKind, StepMode imported from stores/constants

type DragItem = {
  kind: typeof DRAG_KIND.NEW_STEP
  stepType: StepMode
  agentId: string | null
}

type CanvasStoreState = {
  // Selection (mirrored from RF for panel logic)
  selectedNodeIds: ReadonlySet<string>
  selectedEdgeIds: ReadonlySet<string>
  hoveredNodeId: string | null

  // Panel
  panel: PanelKind
  panelTargetId: string | null  // Which step/edge the panel is editing

  // Interaction
  interactionMode: InteractionMode
  dragItem: DragItem | null     // For palette → canvas drag

  // Minimap
  minimapVisible: boolean
}
```

### 2C: ExecutionStore

Live execution state, driven by WebSocket events.

```typescript
// stores/execution/types.ts
// StepExecutionStatus imported from stores/constants (typeof STEP_STATUS values)

type StepExecutionState = {
  status: StepExecutionStatus
  envelope: StepExecutionEnvelope | null
  streamContent: string | null    // For currently streaming step
  startedAt: string | null
  completedAt: string | null
}

type ExecutionStoreState = {
  runId: string | null
  workflowExecutionId: string | null
  isRunning: boolean
  stepStates: Map<string, StepExecutionState>

  // Aggregate
  totalTokensIn: number
  totalTokensOut: number
  totalCostUsd: number
  startedAt: string | null
}
```

**Key selector: per-step status** (each node subscribes to only its own execution state):
```typescript
const selectStepStatus = (stepId: string) =>
  (state: ExecutionStoreState): StepExecutionStatus =>
    state.stepStates.get(stepId)?.status ?? STEP_STATUS.IDLE
```

### 2D: CatalogStore

Shared reference data loaded once, used across the app.

```typescript
// stores/catalog/types.ts

type CatalogStoreState = {
  agents: NormalizedMap<Agent>
  outputSchemas: NormalizedMap<OutputSchema>
  promptTemplates: NormalizedMap<PromptTemplate>
  documents: NormalizedMap<Document>
  tools: NormalizedMap<Tool>
  loading: boolean
}
```

### 2E: UIStore

Global UI state with localStorage persistence.

```typescript
// stores/ui/types.ts
// ToastSeverity, ThemeMode imported from stores/constants

type Toast = {
  id: string
  message: string
  severity: ToastSeverity
  duration: number
}

type UIStoreState = {
  theme: ThemeMode
  sidebarCollapsed: boolean
  toasts: Toast[]
  commandPaletteOpen: boolean
}
```

Uses `withPersistence` middleware for theme + sidebar.

### Phase 2 Files
- `stores/workflow/{workflowStore,selectors,actions,types,tests}.ts`
- `stores/canvas/{canvasStore,selectors,types}.ts`
- `stores/execution/{executionStore,selectors,types}.ts`
- `stores/catalog/{catalogStore,selectors,loaders,types}.ts`
- `stores/ui/{uiStore,selectors,types}.ts`

### Phase 2 Tests
- WorkflowStore: addStep, removeStep (cascading edge delete), position updates, port management
- NormalizedMap memoization: toArray returns same reference when unchanged
- Per-step selector: updating step A doesn't trigger re-render for step B component
- CatalogStore: hydrate from API, lookup by ID

---

## Phase 3: React Flow Bridge

The thin layer that transforms our store data into React Flow format and syncs RF events back.

### 3A: Transforms

```typescript
// bridge/transforms.ts

type StepNodeData = {
  step: WorkflowStep
  inputs: StepInput[]
  outputs: StepOutput[]
  rules: RoutingRule[]
  executionStatus: StepExecutionStatus
  agent: Agent | null
}

const stepToNode = (
  step: WorkflowStep,
  ports: { inputs: StepInput[]; outputs: StepOutput[] },
  rules: RoutingRule[],
  status: StepExecutionStatus,
  agent: Agent | null,
): Node<StepNodeData> => ({
  id: step.id,
  type: stepModeToNodeType(step.execution_mode),  // maps StepMode → NodeTypeKey via constants
  position: { x: step.position_x, y: step.position_y },
  data: { step, inputs: ports.inputs, outputs: ports.outputs, rules, executionStatus: status, agent },
})

const edgeToReactFlowEdge = (edge: WorkflowStepEdge): Edge => ({
  id: edge.id,
  source: edge.from_step_id,
  target: edge.to_step_id,
  sourceHandle: edge.from_output_port ?? 'default',
  targetHandle: edge.to_input_port ?? 'default',
  type: edge.condition_type ? EDGE_TYPE.CONDITIONAL : EDGE_TYPE.DATA_FLOW,
  data: { edge },
  animated: false,  // Set to true during execution
})
```

### 3B: `useFlowNodes` — Memoized node derivation

```typescript
// bridge/useFlowNodes.ts

// Each node component gets its own selector — only re-renders when ITS data changes
const useFlowNodes = (): Node<StepNodeData>[] => {
  const steps = useStore(workflowStore, selectStepArray)
  const inputPorts = useStore(workflowStore, (s) => s.inputPorts, shallow)
  const outputPorts = useStore(workflowStore, (s) => s.outputPorts, shallow)
  const routingRules = useStore(workflowStore, (s) => s.routingRules, shallow)
  const stepStates = useStore(executionStore, (s) => s.stepStates, shallow)
  const agents = useStore(catalogStore, (s) => s.agents)

  return useMemo(
    () => steps.map((step) => stepToNode(
      step,
      { inputs: inputPorts.get(step.id) ?? [], outputs: outputPorts.get(step.id) ?? [] },
      routingRules.get(step.id) ?? [],
      stepStates.get(step.id)?.status ?? STEP_STATUS.IDLE,
      agents.byId.get(step.agent_id ?? '') ?? null,
    )),
    [steps, inputPorts, outputPorts, routingRules, stepStates, agents],
  )
}
```

### 3C: `useFlowSync` — RF events → store

```typescript
// bridge/useFlowSync.ts

const useFlowSync = () => {
  const { executeCommand } = useHistory()

  // Position commit on drag end (not during drag — RF handles that)
  const onNodeDragStop: NodeDragHandler = useCallback((_event, node, nodes) => {
    // If multi-select drag, batch all position updates
    const updates = nodes.map((n) => ({ id: n.id, x: n.position.x, y: n.position.y }))
    executeCommand(moveNodesCommand(updates))
  }, [executeCommand])

  // New connection → add edge
  const onConnect: OnConnect = useCallback((connection) => {
    const edge = connectionToEdge(connection)
    executeCommand(addEdgeCommand(edge))
  }, [executeCommand])

  // Selection sync → canvas store (for panel management)
  const onSelectionChange: OnSelectionChangeFunc = useCallback(({ nodes, edges }) => {
    canvasStore.setState({
      selectedNodeIds: new Set(nodes.map((n) => n.id)),
      selectedEdgeIds: new Set(edges.map((e) => e.id)),
    })
    // Auto-open panel for single selection
    if (nodes.length === 1) {
      canvasStore.setState({ panel: PANEL.STEP_CONFIG, panelTargetId: nodes[0].id })
    } else if (edges.length === 1) {
      canvasStore.setState({ panel: PANEL.EDGE_CONFIG, panelTargetId: edges[0].id })
    }
  }, [])

  // Node deletion
  const onNodesDelete: OnNodesDelete = useCallback((deleted) => {
    for (const node of deleted) {
      executeCommand(removeStepCommand(node.id))
    }
  }, [executeCommand])

  return { onNodeDragStop, onConnect, onSelectionChange, onNodesDelete }
}
```

### 3D: Custom Node Types

```typescript
// bridge/nodeTypes.ts
// Registry of custom node components for React Flow

const nodeTypes = {
  [NODE_TYPE.SINGLE_STEP]: SingleStepNode,   // Standard LLM execution node
  [NODE_TYPE.FOR_EACH_STEP]: ForEachStepNode, // Array iteration node with special handles
  [NODE_TYPE.ROOM_STEP]: RoomStepNode,       // Multi-agent room node
} as const
```

Each custom node uses `React.memo` with data-version comparator:
```typescript
const SingleStepNode = memo(function SingleStepNode({ data, selected }: NodeProps<StepNodeData>) {
  // Inline prompt editing, port handles, status indicator, agent badge
  // Uses `nodrag` class on input elements
  // Framer Motion for expand/collapse of details
}, (prev, next) =>
  prev.data.step === next.data.step &&
  prev.data.executionStatus === next.data.executionStatus &&
  prev.selected === next.selected
)
```

### Phase 3 Files
- `bridge/{transforms,useFlowNodes,useFlowEdges,useFlowSync,nodeTypes,edgeTypes,types}.ts`
- `bridge/index.ts`

### Phase 3 Tests
- transforms: stepToNode maps all fields correctly, edgeToReactFlowEdge maps ports to handles
- useFlowNodes: memoization (returns same array reference when nothing changed)

---

## Phase 4: Undo/Redo System

### 4A: Command Type

```typescript
// stores/history/types.ts
// CommandType imported from stores/constants

type Command = {
  type: CommandType
  description: string     // For UI display: "Move 3 nodes", "Delete step Research"
  execute: () => void
  undo: () => void
}

type HistoryStoreState = {
  past: Command[]
  future: Command[]
  maxSize: number         // Default: HISTORY_MAX_SIZE from constants
}
```

### 4B: HistoryStore

```typescript
// stores/history/historyStore.ts

// Actions:
// push(cmd) — execute + add to past, clear future
// undo() — pop past, call cmd.undo(), push to future
// redo() — pop future, call cmd.execute(), push to past
// clear() — reset both stacks

const historyStore = createStore<HistoryStoreState>((set, get) => ({
  past: [],
  future: [],
  maxSize: 100,
}))

// Hook for components
const useHistory = () => {
  const canUndo = useStore(historyStore, (s) => s.past.length > 0)
  const canRedo = useStore(historyStore, (s) => s.future.length > 0)

  const executeCommand = useCallback((cmd: Command) => {
    cmd.execute()
    historyStore.setState((s) => ({
      past: [...s.past.slice(-(s.maxSize - 1)), cmd],
      future: [],
    }))
  }, [])

  const undo = useCallback(() => { /* pop past, call undo, push future */ }, [])
  const redo = useCallback(() => { /* pop future, call execute, push past */ }, [])

  return { canUndo, canRedo, executeCommand, undo, redo }
}
```

### 4C: Command Factories

```typescript
// stores/history/commands.ts

// Each factory captures before/after state for reversibility

const moveNodesCommand = (
  updates: Array<{ id: string; x: number; y: number }>
): Command => {
  // Capture current positions BEFORE move
  const before = updates.map(({ id }) => {
    const step = workflowStore.getState().steps.byId.get(id)
    return { id, x: step?.position_x ?? 0, y: step?.position_y ?? 0 }
  })
  return {
    type: COMMAND.MOVE_NODES,
    description: updates.length === 1 ? 'Move node' : `Move ${updates.length} nodes`,
    execute: () => workflowStore.getState().updateStepPositions(updates),
    undo: () => workflowStore.getState().updateStepPositions(before),
  }
}

const addStepCommand = (step: WorkflowStep): Command => ({
  type: COMMAND.ADD_STEP,
  description: `Add step "${step.name}"`,
  execute: () => workflowStore.getState().addStep(step),
  undo: () => workflowStore.getState().removeStep(step.id),
})

const removeStepCommand = (stepId: string): Command => {
  // Capture full step data + connected edges + ports for restore
  const state = workflowStore.getState()
  const step = state.steps.byId.get(stepId)!
  const connectedEdges = toArray(state.edges).filter(
    (e) => e.from_step_id === stepId || e.to_step_id === stepId,
  )
  const inputs = state.inputPorts.get(stepId) ?? []
  const outputs = state.outputPorts.get(stepId) ?? []
  const rules = state.routingRules.get(stepId) ?? []

  return {
    type: COMMAND.REMOVE_STEP,
    description: `Delete step "${step.name}"`,
    execute: () => workflowStore.getState().removeStep(stepId),
    undo: () => {
      const actions = workflowStore.getState()
      actions.addStep(step)
      connectedEdges.forEach((e) => actions.addEdge(e))
      actions.setInputPorts(stepId, inputs)
      actions.setOutputPorts(stepId, outputs)
      // ... restore rules, docs
    },
  }
}

const updateStepConfigCommand = (
  stepId: string,
  patch: Partial<WorkflowStep>,
): Command => {
  const before = workflowStore.getState().steps.byId.get(stepId)!
  const prevPatch = Object.fromEntries(
    Object.keys(patch).map((k) => [k, before[k as keyof WorkflowStep]]),
  )
  return {
    type: COMMAND.UPDATE_STEP_CONFIG,
    description: `Update step "${before.name}"`,
    execute: () => workflowStore.getState().updateStep(stepId, patch),
    undo: () => workflowStore.getState().updateStep(stepId, prevPatch as Partial<WorkflowStep>),
  }
}

const addEdgeCommand = (edge: WorkflowStepEdge): Command => ({
  type: COMMAND.ADD_EDGE,
  description: 'Add connection',
  execute: () => workflowStore.getState().addEdge(edge),
  undo: () => workflowStore.getState().removeEdge(edge.id),
})
```

### 4D: Keyboard Bindings

```typescript
// Registered via useEffect in the editor page component:
useEffect(() => {
  const handler = (e: KeyboardEvent) => {
    if ((e.metaKey || e.ctrlKey) && e.key === 'z' && !e.shiftKey) {
      e.preventDefault()
      undo()
    }
    if ((e.metaKey || e.ctrlKey) && e.key === 'z' && e.shiftKey) {
      e.preventDefault()
      redo()
    }
  }
  window.addEventListener('keydown', handler)
  return () => window.removeEventListener('keydown', handler)
}, [undo, redo])
```

### Phase 4 Files
- `stores/history/{historyStore,commands,types}.ts`
- `stores/history/index.ts`

### Phase 4 Tests
- push/undo/redo cycle
- Undo restores exact previous state
- Future stack clears on new command
- Remove step undo restores step + connected edges + ports
- Max size truncation

---

## Phase 5: API Sync Layer

### 5A: Debounced Auto-Save

```typescript
// stores/sync/apiSync.ts

// Subscribe to workflowStore.dirty — when true, debounce API save
const startAutoSync = (workflowId: string) => {
  let timeout: ReturnType<typeof setTimeout>

  return workflowStore.subscribe(() => {
    const { dirty } = workflowStore.getState()
    if (!dirty) return

    clearTimeout(timeout)
    timeout = setTimeout(async () => {
      await syncToApi(workflowId)
      workflowStore.setState({ dirty: false })
    }, SYNC_DEBOUNCE_MS)
  })
}
```

### 5B: Sync Strategy

```
syncToApi(workflowId):
  1. Diff current store state vs last synced state
  2. For each changed step: PUT /api/workflows/:id/steps/:sid
  3. For each added edge: POST /api/workflows/:id/edges
  4. For each removed edge: DELETE /api/workflows/:id/edges
  5. For each changed port set: PUT step ports
  6. Update lastSyncedState snapshot
```

### 5C: Optimistic Updates

All mutations happen instantly in the store. API sync runs in the background. If the API call fails:
1. Show toast notification with error
2. Mark store as dirty (will retry on next change)
3. For critical failures (409 conflict), prompt user to reload

Version-based conflict detection: each step/workflow has a `version` field. If the API returns 409, another client changed the data.

### Phase 5 Files
- `stores/sync/{apiSync,optimistic,conflict}.ts`
- `stores/sync/index.ts`

---

## Phase 6: Execution + WebSocket

### 6A: WebSocket Event Handlers

```typescript
// stores/execution/wsHandlers.ts

// Maps WS events to store mutations
const handleExecutionEvent = (event: WsExecutionEvent) => {
  switch (event.type) {
    case WS_EVENT.STAGE_EXECUTION_UPDATE:
      executionStore.setState((s) => ({
        stepStates: new Map(s.stepStates).set(event.step_id, {
          status: mapStatus(event.status),
          envelope: event.envelope ?? s.stepStates.get(event.step_id)?.envelope ?? null,
          streamContent: null,
          startedAt: event.started_at,
          completedAt: event.completed_at,
        }),
      }))
      break

    case WS_EVENT.EXECUTION_MESSAGE:
      // Streaming content for currently executing step
      executionStore.setState((s) => ({
        stepStates: new Map(s.stepStates).set(event.step_id, {
          ...s.stepStates.get(event.step_id)!,
          streamContent: (s.stepStates.get(event.step_id)?.streamContent ?? '') + event.chunk,
        }),
      }))
      break

    case WS_EVENT.PIPELINE_RUN_UPDATE:
      executionStore.setState({
        isRunning: event.status === STEP_STATUS.RUNNING,
        ...(event.status === STEP_STATUS.SUCCESS ? { totalTokensIn: event.tokens_in, ... } : {}),
      })
      break
  }
}
```

### Phase 6 Files
- `stores/execution/{executionStore,selectors,wsHandlers,types}.ts`

---

## Phase 7: Animation Layer

Framer Motion integration with store state changes.

### 7A: Node Status Animations

```typescript
// Custom node wraps content in motion.div
// Status changes trigger color/scale transitions

<motion.div
  animate={{
    borderColor: statusColor(data.executionStatus),
    scale: data.executionStatus === STEP_STATUS.RUNNING ? 1.02 : 1,
  }}
  transition={{ duration: 0.2 }}
>
  {/* Node content */}
</motion.div>
```

### 7B: Panel Transitions

```typescript
// Side panel open/close with AnimatePresence
<AnimatePresence mode="wait">
  {panel !== PANEL.CLOSED && (
    <motion.aside
      key={panel}
      initial={{ x: 320, opacity: 0 }}
      animate={{ x: 0, opacity: 1 }}
      exit={{ x: 320, opacity: 0 }}
      transition={{ type: 'spring', damping: 25, stiffness: 300 }}
    >
      <PanelContent kind={panel} targetId={panelTargetId} />
    </motion.aside>
  )}
</AnimatePresence>
```

### 7C: Execution Flow Animation

During execution, edges animate to show data flow:
```typescript
// Edge becomes animated when source step completes
const animated = executionStatus === STEP_STATUS.RUNNING || justCompleted
```

### Phase 7 Dependencies
- Install: `framer-motion`
- No new store files — animation lives in component layer, driven by store selectors

---

## Implementation Phases (Build Order)

| Phase | What | Depends On | Est. Files |
|-------|------|-----------|------------|
| **1** | Store infrastructure (createStore, useStore, batch, shallow, NormalizedMap) | Nothing | 8 |
| **2** | Domain stores (Workflow, Canvas, Execution, Catalog, UI) + types | Phase 1 | 20 |
| **3** | React Flow bridge (transforms, sync hooks, node/edge types) | Phase 1+2 | 8 |
| **4** | Undo/redo (HistoryStore, command factories) | Phase 2+3 | 4 |
| **5** | API sync layer (debounced save, optimistic updates, conflict detection) | Phase 2 | 4 |
| **6** | Execution + WebSocket (execution store integration, WS handlers) | Phase 2 | 4 |
| **7** | Animation layer (Framer Motion integration, status animations) | Phase 3 | 0 (in components) |

**Total: ~48 files across 7 phases**

Each phase is independently testable. Phase 1 is pure library code. Phase 2 can be tested with unit tests before any UI exists. Phase 3 needs React Flow installed but can be tested with `renderHook`. Phases 4-7 build incrementally.

---

## Dependencies to Install

```bash
npm install @xyflow/react framer-motion
```

React Flow v12+ and Framer Motion. No state management libraries.

---

## Verification

Per phase:
```bash
# Phase 1: Core store tests
npx vitest run src/stores/core/

# Phase 2: Domain store tests
npx vitest run src/stores/

# Phase 3: Bridge tests
npx vitest run src/bridge/

# Phase 4: History tests
npx vitest run src/stores/history/

# Full suite
npx vitest run
npx tsc --noEmit    # Type check
npx eslint .        # Lint
```

Integration test: render a `<ReactFlow>` with 100 nodes, verify:
- Drag a node → position persists in store
- Add edge → store updated
- Undo → edge removed
- Update step config → only that node re-renders (React DevTools Profiler)
- Start execution → status overlays animate correctly

---

## Key Type Alignment (Backend ↔ Frontend)

The frontend `WorkflowStep` type needs updating to match the backend `WorkflowStepRow`:

```typescript
// New WorkflowStep type (to be updated in types/workflow.ts)
type WorkflowStep = {
  id: string
  workflow_id: string
  name: string
  agent_id: string
  execution_mode: string         // 'single' | 'for_each' | 'room'
  for_each_ref: string | null
  prompt_template_id: string | null
  prompt_template: string
  output_schema_id: string | null
  output_variable_name: string | null
  interactive_agent_id: string | null
  for_each_label_field: string | null
  room_id: string | null
  routing_mode: string | null
  routing_field: string | null
  display_order: number
  position_x: number
  position_y: number
  version: number
}

type StepInput = {
  id: string
  workflow_step_id: string
  port_name: string
  port_type: string
  required: boolean
  default_value: unknown | null
  description: string | null
  json_schema: unknown | null
}

type StepOutput = {
  id: string
  workflow_step_id: string
  port_name: string
  port_type: string
  json_path: string
  description: string | null
  json_schema: unknown | null
}

type RoutingRule = {
  id: string
  workflow_step_id: string
  label_value: string
  description: string | null
  agent_id: string
  display_order: number
}
```
