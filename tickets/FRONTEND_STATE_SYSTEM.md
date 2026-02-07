# Plan: Frontend State System — Full App Rebuild

## Context

The Nexor frontend is being rebuilt from scratch. The existing contexts/hooks are outdated and will be deleted. The backend has 141 API endpoints across 25 resource groups with 40+ database tables. We need a state system that maps 1:1 to the backend, built on pure functions, subscriber components, and scalable store patterns.

**Key decisions:**
- Custom store on `useSyncExternalStore` (no external state libraries)
- `createResourceStore<T>` factory eliminates CRUD boilerplate
- `NormalizedMap<T>` for O(1) lookups with memoized arrays
- Stores are modules (not contexts) — singleton instances, imported directly
- WebSocket events routed to stores via `WsStoreRouter` component
- Existing `api/api.ts` typed client + `api/sse.ts` SSE factory reused as-is

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    React Components                      │
│  (subscribe via useStore(store, selector) — only         │
│   re-render when their slice changes)                    │
├─────────┬──────────┬──────────┬──────────┬─────────────┤
│  Auth   │ Agents   │ Workflow │  Rooms   │  ...16 more │
│  Store  │  Store   │  Store   │  Store   │   stores    │
├─────────┴──────────┴──────────┴──────────┴─────────────┤
│      createStore() + useStore() + NormalizedMap          │
│      createResourceStore() — CRUD factory                │
├──────────────────────┬─────────────────────────────────┤
│   WsStoreRouter      │         api/ (typed client)      │
│   (WS → store sync)  │    api/sse.ts (SSE streams)      │
└──────────────────────┴─────────────────────────────────┘
```

---

## Part 1: Core Infrastructure

### Files (all in `stores/lib/`)

| File | Purpose |
|------|---------|
| `createStore.ts` | `createStore<T>(creator)` → `StoreApi<T>` with `getState`, `setState`, `subscribe`, `destroy` |
| `useStore.ts` | `useStore(store, selector, equalityFn?)` via `useSyncExternalStore`. Also `useStoreActions()` for non-subscribing action access |
| `NormalizedMap.ts` | `NormalizedMap<T>` — `Map<string, T>` wrapper with `toArray()` memoization, `set`, `delete`, `fromArray` |
| `shallow.ts` | Shallow object equality for multi-field selectors |
| `batch.ts` | `batch(fn)` — coalesce multiple `setState` into single listener notification |
| `createResourceStore.ts` | Higher-order factory generating CRUD stores from API config |
| `types.ts` | `StoreApi<T>`, `SetState<T>`, `GetState<T>`, `StateCreator<T>`, `Listener` |
| `index.ts` | Barrel export |

### `createResourceStore<T>` Factory

Generates standard CRUD stores from config. Eliminates boilerplate for ~9 stores:

```typescript
type ResourceStoreConfig<T extends { id: string }, TCreate, TUpdate> = {
  name: string
  api: {
    list: () => Promise<unknown>
    get: (id: string) => Promise<T>
    create: (body: TCreate) => Promise<T>
    update: (id: string, body: TUpdate) => Promise<T>
    delete: (id: string) => Promise<void>
  }
  unwrapList: (response: unknown) => T[]
}
```

Returns: `{ store, selectAll, selectById, selectLoading, selectError, fetchAll, fetchOne, create, update, remove, upsert, removeById, setAll }`

**Uses factory:** ToolStore, TaskStore, DocumentStore (extends), OutputSchemaStore, PromptTemplateStore, AgentStore (extends), ToolRouterStore (extends), CollectionStore (extends), ResultStore (extends)

**Hand-written:** AuthStore, SessionStore, WorkflowStore, AgentExecutionStore, RoomStore, CostStore, ConfigStore

---

## Part 2: Domain Stores (16 stores)

### Store 1: `auth/authStore.ts` — Hand-written

```typescript
type AuthState = {
  user: User | null
  token: string | null
  status: 'idle' | 'loading' | 'authenticated' | 'unauthenticated'
  error: string | null
}
```

Actions: `login(email, password)`, `register(email, password)`, `logout()`, `refresh()`, `fetchMe()`
Selectors: `selectUser`, `selectIsAuthenticated`, `selectToken`
API: `api.auth.*`
WS: None. Persists token to localStorage.

### Store 2: `agents/agentStore.ts` — Extends factory

```typescript
type AgentState = {
  agents: NormalizedMap<Agent>
  stats: AgentPoolStats | null
  toolsByAgent: Record<string, Tool[]>
  contextByAgent: Record<string, DocumentListItem[]>
  loading: boolean
  error: string | null
}
```

Base CRUD from factory. Extended with:
- `fetchTools(agentId)` / `setTools(agentId, toolIds)` — `api.agents.getTools/setTools`
- `fetchContext(agentId)` / `setContext(agentId, docIds)` — `api.agents.getContext/setContext`

### Store 3: `tools/toolStore.ts` — Pure factory

```typescript
type ToolState = { items: NormalizedMap<Tool>; loading: boolean; error: string | null }
```

Pure `createResourceStore` usage. API: `api.tools.*`

### Store 4: `tasks/taskStore.ts` — Extends factory

```typescript
type TaskState = { items: NormalizedMap<Task>; loading: boolean; error: string | null }
```

Extended with `updateStatus(id, status)` — PATCH `/tasks/:id/status`

### Store 5: `documents/documentStore.ts` — Extends factory

```typescript
type DocumentState = {
  items: NormalizedMap<DocumentListItem>
  detail: Record<string, Document>
  searchResults: DocumentSearchResult[]
  loading: boolean
  error: string | null
}
```

Extended with:
- `search(query)` — `api.documents.search`
- `upload(formData)` — `api.documents.upload` (if endpoint exists)
- `fetchDetail(id)` → populates `detail` map (full content)

### Store 6: `sessions/sessionStore.ts` — Hand-written

```typescript
type SessionState = {
  sessions: NormalizedMap<Session>
  messagesBySession: Record<string, ChatMessage[]>
  activeStreams: Record<string, (() => void) | null>
  loading: boolean
  error: string | null
}
```

Actions: CRUD + `fetchMessages(sessionId)`, `sendMessage(sessionId, body)`, `startStream(sessionId, messageId)` via `createSSEStream`, `stopStream(sessionId)`, `clearMessages(sessionId)`, `updateConfig(id, draftConfig)`, `saveAgent(id, body)`

WS: Subscribes to `session` topic — `created` inserts, `updated` patches, `deleted` removes.

### Store 7: `workflows/workflowStore.ts` — Hand-written

```typescript
type WorkflowState = {
  workflows: NormalizedMap<Workflow>
  // Active workflow context (single workflow loaded for editing)
  activeWorkflowId: string | null
  steps: NormalizedMap<WorkflowStep>
  edges: NormalizedMap<WorkflowStepEdge>
  inputPorts: Record<string, StepInput[]>
  outputPorts: Record<string, StepOutput[]>
  routingRules: Record<string, RoutingRule[]>
  stepDocuments: Record<string, StepDocument[]>
  runs: WorkflowRun[]
  loading: boolean
  error: string | null
  dirty: boolean
}
```

Actions: CRUD for workflows + `loadWorkflow(id)` (fetches steps/edges/ports/rules), CRUD for steps/edges, port management, routing rule management, step document management, `executeWorkflow(id)`, `resumeWorkflow(id)`, `fetchRuns(id)`, position updates.

WS: Subscribes to `workflow` topic for execution state updates.

New types needed in `stores/workflows/types.ts`:
- `StepInput` — `{ id, port_name, port_type, required, default_value, description, json_schema }`
- `StepOutput` — `{ id, port_name, port_type, json_path, description, json_schema }`
- `RoutingRule` — `{ id, label_value, description, agent_id, display_order }`
- `WorkflowRun` — `{ id, workflow_id, status, started_at, completed_at, error }`

### Store 8: `outputSchemas/outputSchemaStore.ts` — Pure factory

API: `api.outputSchemas.*`. Unwrap: `(res) => res.items`

### Store 9: `promptTemplates/promptTemplateStore.ts` — Pure factory

API: `api.promptTemplates.*`. Unwrap: `(res) => res.items`

### Store 10: `executions/executionStore.ts` — Hand-written

```typescript
type ExecutionState = {
  executions: NormalizedMap<AgentExecution>
  messagesByExecution: Record<string, ExecutionMessage[]>
  activeStreams: Record<string, (() => void) | null>
  loading: boolean
  error: string | null
}
```

Actions: `fetchAll(params?)`, `fetchOne(id)`, `fetchMessages(id)`, `sendMessage(id, body)`, `startStream(executionId, streamId)` via `createSSEStream`, `stopStream(executionId)`, `approve(id, body?)`

Selectors: `selectExecutions`, `selectExecution(id)`, `selectMessages(execId)`, `selectAwaitingApproval`

### Store 11: `results/resultStore.ts` — Extends factory

Extended with `fetchByExecution(executionId)` → `api.results.byExecution(eid)`

Secondary index: `resultsByExecution: Record<string, string[]>`

### Store 12: `costs/costStore.ts` — Hand-written singleton

```typescript
type CostState = {
  summary: CostResponse | null
  byAgent: unknown[] | null
  byModel: unknown[] | null
  recent: unknown[] | null
  loading: boolean
  error: string | null
  lastFetched: number | null
}
```

Actions: `fetchSummary()`, `fetchByAgent()`, `fetchByModel()`, `fetchRecent()`
Selector: `selectIsStale(state)` — true if lastFetched > threshold

### Store 13: `toolRouters/toolRouterStore.ts` — Extends factory

```typescript
type ToolRouterState = {
  items: NormalizedMap<ToolRouter>
  toolsByRouter: Record<string, Tool[]>
  modesByRouter: Record<string, RouterMode[]>
  toolsByMode: Record<string, Tool[]>
  loading: boolean
  error: string | null
}
```

Extended with: `fetchRouterTools/setRouterTools`, `fetchModes/createMode/updateMode/deleteMode`, `fetchModeTools/setModeTools`

### Store 14: `rooms/roomStore.ts` — Hand-written

```typescript
type RoomState = {
  rooms: NormalizedMap<Room>
  membersByRoom: Record<string, RoomMember[]>
  sessionsByRoom: Record<string, RoomSession[]>
  activeSessionId: string | null
  transcript: RoomTranscriptEntry[]
  outputs: RoomOutput[]
  loading: boolean
  error: string | null
}
```

Actions: Room CRUD + member management + session lifecycle (create, close, transcript, outputs, send message).

WS: Subscribes to `room` topic — `speaker_start/token/end`, `turn_complete`, `session_complete`

New types needed in `stores/rooms/types.ts`:
- `Room`, `RoomMember`, `RoomSession`, `RoomTranscriptEntry`, `RoomOutput`

### Store 15: `collections/collectionStore.ts` — Extends factory

```typescript
type CollectionState = {
  items: NormalizedMap<Collection>
  runsByCollection: Record<string, CollectionRun[]>
  loading: boolean
  error: string | null
}
```

Extended with: `execute(id)`, `fetchRunStatus(runId)`, `fetchRuns(collectionId)`

New types in `stores/collections/types.ts`: `Collection`, `CollectionRun`

### Store 16: `config/configStore.ts` — Hand-written singleton

```typescript
type ConfigState = {
  config: Config | null
  health: { status: string } | null
  systemStats: UsageSummary | null
  loading: boolean
  error: string | null
}
```

Actions: `fetchConfig()`, `updateConfig(body)`, `fetchHealth()`, `fetchStats()`

---

## Part 3: Workflow Editor Stores (4 additional stores)

These serve the React Flow workflow editor specifically.

### `canvas/canvasStore.ts`

```typescript
type CanvasState = {
  selectedNodeIds: ReadonlySet<string>
  selectedEdgeIds: ReadonlySet<string>
  hoveredNodeId: string | null
  panel: PanelKind        // 'closed' | 'step-config' | 'edge-config' | ...
  panelTargetId: string | null
  interactionMode: InteractionMode  // 'select' | 'connect' | 'pan'
  dragItem: DragItem | null
  minimapVisible: boolean
}
```

### `execution/workflowExecutionStore.ts`

Live execution overlay — per-step status driven by WS events:

```typescript
type WorkflowExecutionState = {
  runId: string | null
  isRunning: boolean
  stepStates: Map<string, StepExecutionState>
  totalTokensIn: number
  totalTokensOut: number
  startedAt: string | null
}

type StepExecutionState = {
  status: 'idle' | 'pending' | 'running' | 'success' | 'error' | 'skipped' | 'paused'
  envelope: unknown | null
  streamContent: string | null
  startedAt: string | null
  completedAt: string | null
}
```

Selector: `selectStepStatus(stepId)` — each node subscribes to only its own status.

### `history/historyStore.ts`

Undo/redo via command pattern:

```typescript
type Command = { type: string; description: string; execute: () => void; undo: () => void }
type HistoryState = { past: Command[]; future: Command[]; maxSize: number }
```

Actions: `push(cmd)`, `undo()`, `redo()`, `clear()`
Command factories: `moveNodesCommand`, `addStepCommand`, `removeStepCommand`, `updateStepConfigCommand`, `addEdgeCommand`

### `ui/uiStore.ts`

```typescript
type UIState = {
  theme: 'light' | 'dark'
  sidebarCollapsed: boolean
  toasts: Toast[]
  commandPaletteOpen: boolean
}
```

Persists theme + sidebar to localStorage.

---

## Part 4: WebSocket Integration

### `stores/ws/WsStoreRouter.tsx`

A headless component mounted inside the app root that wires WS events to stores:

```typescript
function WsStoreRouter() {
  const { subscribe } = useWebSocket()

  useEffect(() => {
    const unsubs = [
      subscribe('workflow', workflowExecutionStore.handleWsEvent),
      subscribe('room', roomStore.handleWsEvent),
      subscribe('session', sessionStore.handleWsEvent),
    ]
    return () => unsubs.forEach(fn => fn())
  }, [subscribe])

  return null
}
```

Each store that receives WS events exposes a `handleWsEvent(msg: WsWireMessage)` method that switches on `msg.event`.

---

## Part 5: React Flow Bridge (workflow editor)

### Files (in `bridge/`)

| File | Purpose |
|------|---------|
| `transforms.ts` | `WorkflowStep` ↔ RF `Node<StepNodeData>`, `WorkflowStepEdge` ↔ RF `Edge` |
| `useFlowNodes.ts` | Store → RF nodes (memoized). Combines workflow + execution + catalog stores |
| `useFlowEdges.ts` | Store → RF edges (memoized) |
| `useFlowSync.ts` | RF callbacks → store mutations (`onNodeDragStop`, `onConnect`, `onSelectionChange`) |
| `nodeTypes.ts` | Custom node registry: `singleStep`, `forEachStep`, `roomStep` |
| `edgeTypes.ts` | Custom edge registry: `dataFlow`, `conditional` |
| `types.ts` | `StepNodeData`, `EdgeData` |

---

## File Structure

```
frontend/src/stores/
  index.ts                              # Barrel export
  lib/
    createStore.ts                      # Store factory
    useStore.ts                         # Selector hook
    NormalizedMap.ts                    # Normalized collection
    shallow.ts                          # Equality comparator
    batch.ts                            # Batch updates
    createResourceStore.ts              # CRUD factory
    types.ts                            # Core types
    index.ts
  auth/       { authStore, selectors, index }.ts
  agents/     { agentStore, selectors, index }.ts
  tools/      { toolStore, selectors, index }.ts
  tasks/      { taskStore, selectors, index }.ts
  documents/  { documentStore, selectors, index }.ts
  sessions/   { sessionStore, selectors, index }.ts
  workflows/  { workflowStore, selectors, types, index }.ts
  outputSchemas/    { outputSchemaStore, selectors, index }.ts
  promptTemplates/  { promptTemplateStore, selectors, index }.ts
  executions/ { executionStore, selectors, index }.ts
  results/    { resultStore, selectors, index }.ts
  costs/      { costStore, selectors, index }.ts
  toolRouters/{ toolRouterStore, selectors, index }.ts
  rooms/      { roomStore, selectors, types, index }.ts
  collections/{ collectionStore, selectors, types, index }.ts
  config/     { configStore, selectors, index }.ts
  canvas/     { canvasStore, selectors, index }.ts
  execution/  { workflowExecutionStore, selectors, wsHandlers, index }.ts
  history/    { historyStore, commands, types, index }.ts
  ui/         { uiStore, selectors, index }.ts
  ws/         { WsStoreRouter.tsx, index }.ts

frontend/src/bridge/
  transforms.ts
  useFlowNodes.ts
  useFlowEdges.ts
  useFlowSync.ts
  nodeTypes.ts
  edgeTypes.ts
  types.ts
  index.ts
```

**Total: ~80 files (including tests colocated per convention)**

---

## Existing Files to Reuse (keep as-is)

| File | What |
|------|------|
| `frontend/src/api/api.ts` | Typed API client — all store actions call this |
| `frontend/src/api/client.ts` | HTTP client with retry, dedup, interceptors |
| `frontend/src/api/sse.ts` | SSE stream factory for SessionStore + ExecutionStore |
| `frontend/src/types/*.ts` | 20 type files already match backend (Agent, Task, Tool, Document, Session, Workflow, Execution, Result, Cost, Config, etc.) |
| `frontend/src/types/ws.ts` | WS event types + constants |
| `frontend/src/contexts/WebSocketContext.tsx` | WS connection + topic subscriptions |
| `frontend/src/hooks/useWebSocket.ts` | WS hook |
| `frontend/src/constants.ts` | App constants |

---

## Existing Files to Delete

All existing context files and their hooks (replaced by stores):
- `contexts/AgentContext.tsx`, `TaskContext.tsx`, `ChatContext.tsx`, `FeedContext.tsx`, `ReviewQueueContext.tsx`, `AuthContext.tsx`
- `hooks/useAgents.ts`, `hooks/useTaskContext.ts`, `hooks/useChatContext.ts`, `hooks/useFeed.ts`, `hooks/useReviewQueue.ts`, `hooks/useAuth.ts`
- `test/fixtures.ts` (rebuild with store-compatible fixtures)

**Keep:** `contexts/WebSocketContext.tsx`, `hooks/useWebSocket.ts`

---

## Build Order

### Phase 1: Core Infrastructure
1. `stores/lib/types.ts`
2. `stores/lib/createStore.ts` + tests
3. `stores/lib/useStore.ts` + tests
4. `stores/lib/NormalizedMap.ts` + tests
5. `stores/lib/shallow.ts` + tests
6. `stores/lib/batch.ts` + tests
7. `stores/lib/createResourceStore.ts` + tests
8. `stores/lib/index.ts`

**Depends on:** Nothing. Pure library code.

### Phase 2: Auth Store
1. `stores/auth/authStore.ts` + tests
2. Delete `contexts/AuthContext.tsx` + `hooks/useAuth.ts`

**Depends on:** Phase 1.

### Phase 3: Simple CRUD Stores (parallel)
Build all 5 in parallel — they are independent:
1. `stores/tools/toolStore.ts` + tests
2. `stores/outputSchemas/outputSchemaStore.ts` + tests
3. `stores/promptTemplates/promptTemplateStore.ts` + tests
4. `stores/tasks/taskStore.ts` + tests
5. `stores/documents/documentStore.ts` + tests

Delete corresponding contexts + hooks.

**Depends on:** Phase 1 (factory).

### Phase 4: Extended Resource Stores (parallel)
1. `stores/agents/agentStore.ts` + tests (extends with tools/context)
2. `stores/results/resultStore.ts` + tests (extends with fetchByExecution)
3. `stores/toolRouters/toolRouterStore.ts` + tests (extends with modes)
4. `stores/collections/collectionStore.ts` + tests (extends with execute/runs)

**Depends on:** Phase 1.

### Phase 5: Session + Execution Stores
1. `stores/sessions/sessionStore.ts` + tests (chat, SSE streaming)
2. `stores/executions/executionStore.ts` + tests (messages, SSE, approval)

**Depends on:** Phase 1. Uses `api/sse.ts`.

### Phase 6: Workflow Store
1. `stores/workflows/types.ts` (StepInput, StepOutput, RoutingRule, WorkflowRun)
2. `stores/workflows/workflowStore.ts` + tests
3. `stores/workflows/selectors.ts`

**Depends on:** Phase 1. May need API endpoint additions for step ports/routing rules.

### Phase 7: Room Store
1. `stores/rooms/types.ts` (Room, RoomMember, RoomSession, RoomTranscriptEntry, RoomOutput)
2. `stores/rooms/roomStore.ts` + tests

**Depends on:** Phase 1. May need API endpoint additions for rooms.

### Phase 8: Singleton Stores
1. `stores/costs/costStore.ts` + tests
2. `stores/config/configStore.ts` + tests

**Depends on:** Phase 1.

### Phase 9: WebSocket Integration
1. `stores/ws/WsStoreRouter.tsx` + tests
2. Add `handleWsEvent` to SessionStore, RoomStore, WorkflowExecutionStore

**Depends on:** Phases 5, 6, 7.

### Phase 10: UI Store
1. `stores/ui/uiStore.ts` + tests (theme, toasts, sidebar)

**Depends on:** Phase 1.

### Phase 11: Workflow Editor Stores
1. `stores/canvas/canvasStore.ts` + tests
2. `stores/execution/workflowExecutionStore.ts` + tests + wsHandlers
3. `stores/history/historyStore.ts` + commands + tests

**Depends on:** Phases 1, 6.

### Phase 12: React Flow Bridge
1. `bridge/transforms.ts` + tests
2. `bridge/useFlowNodes.ts`, `useFlowEdges.ts`, `useFlowSync.ts`
3. `bridge/nodeTypes.ts`, `edgeTypes.ts`

**Depends on:** Phases 6, 11. Install: `@xyflow/react`, `framer-motion`.

---

## Verification

Per phase:
```bash
npx vitest run src/stores/lib/       # Phase 1: core infrastructure
npx vitest run src/stores/auth/      # Phase 2: auth
npx vitest run src/stores/           # Phase 3-8: all domain stores
npx vitest run src/stores/ws/        # Phase 9: WS integration
npx vitest run src/bridge/           # Phase 12: RF bridge
npx vitest run                       # Full suite
npx tsc --noEmit                     # Type check
npx eslint .                         # Lint (zero warnings)
```

Integration: Render a page that uses `useStore(agentStore, selectAgents)` — verify:
- Initial fetch populates store
- Component re-renders only when agents change
- CRUD operations update store + API
- Multiple components sharing same store see consistent state
