# Frontend Build Guide

This document covers the current frontend implementation: routes, the API client, the WebSocket protocol, the canvas/rendering architecture, the node type system, the workflow-agent chat, state management, and the Dispatch system. It reflects the code as it exists today, not the product's earlier design.

For product framing (Phase 0 structural build + async agent design), see `visions/vision-visual-dispatch.md`.

---

## Pages Overview

Routes are declared in `frontend/src/constants.ts` (`ROUTES`) and wired in `frontend/src/router.tsx`. Everything below `AuthGuard` renders inside `AppLayout`.

| Page | Route | Purpose |
|------|-------|---------|
| Dashboard | `/` | Overview of your agents and tasks |
| Chat | `/chat` (`/chat/:sessionId`) | Start/continue a conversation with an agent |
| Agents | `/agents` | List and manage agent templates |
| Agent Workshop | `/agents/workshop/:sessionId?` | Iterate on a single agent's system prompt/config against a live test session (split-pane editor + chat) |
| Agent Detail | `/agents/:id` | Edit an agent's config and attached context documents |
| Workflows | `/workflows` | List and manage workflows |
| Workflow Editor | `/workflows/:id` | The hand-rolled Board canvas + sidebar (tree/chat) — see below |
| Workflow Runs | `/workflows/:id/runs` | Execution history for a workflow |
| Workflow Run Detail | `/workflows/:id/runs/:runId` | Per-step results for one run |
| Review Queue | `/review-queue` | Interactive agent executions awaiting human approval, with inline chat |
| Documents | `/documents` | Browse and manage context documents |
| Settings | `/settings` | App/account settings |

`ROUTES` also defines `SCHEMAS`, `SCHEMA_DETAIL`, `PROMPTS`, `PROMPT_DETAIL`, `RESULTS`, `COSTS`, and `SHOWCASE` constants, but **none of these are registered in `router.tsx`** — there is no route that renders them. Output schemas and prompt templates are still real backend/API concepts (steps reference them by ID), but there is currently no page or panel in the live app for browsing or editing them directly; the components that once did (`frontend/src/components/panels/SchemasBrowserPanel.tsx`, `PromptsBrowserPanel.tsx`, `PropertiesPanel.tsx`, and the rest of `components/panels/`) are orphaned — grep confirms nothing outside that directory imports them. They were the properties-panel UI for the legacy React-Flow canvas (see below) and were not ported when the Board replaced it.

---

## Canvas / Rendering Architecture — `components/board/` vs `components/canvas/`

This is the most important thing to get right, and it is **not** "board is the drawing surface, canvas is what's drawn inside it." They are two different generations of the workflow editor's canvas, confirmed by tracing actual imports and git history:

- **`frontend/src/components/board/` is the current, active canvas.** `WorkflowEditorPage` (`frontend/src/pages/Workflows/WorkflowEditorPage.tsx`) imports and renders `<Board workflowId={id} />` from `@/components/board`, nothing else. `Board.tsx` is a hand-rolled, Excalidraw-inspired canvas: raw HTML5 Canvas 2D rendering (`components/board/canvas/Canvas2D.tsx` + `renderer.ts`), a generic element model of `BoxElement` (a rectangle with plain `text`), `ArrowElement`, and `PenElement` (`components/board/elements/types.ts`), pan/zoom/selection/drag interactions (`components/board/interactions/`), and serialization to an Excalidraw-compatible JSON array (`elements/serialize.ts`) that's POSTed to the backend's board endpoints. Steps are not rendered as rich per-type node components on the canvas — they're boxes with text; the backend's Phase 0 processing maps board elements to workflow steps/edges (`boardStore.elementStepMap`) and returns diffs.
- **`frontend/src/components/canvas/` is a legacy, largely orphaned canvas.** It's a `@xyflow/react` (React Flow) implementation: `WorkflowCanvas.tsx` renders a `ReactFlow` graph with a custom node type `canvasNode` → `CanvasNode` (`components/canvas/nodeTypes.ts`, `CanvasNode/CanvasNode.tsx`), driven by a `NodeVariant` registry (`CanvasNode/registry.ts`) that maps `step.execution_mode` to a variant (`workforce`, `manager`, `room`, `blank`, `agent`, `context`, `input`, `step`) with per-variant layout (`tabbed`, `editor`, `card`), and mappers including `mappers/nodes.ts` and `mappers/agentArtifactNodes.ts` (which spawns roster agents as their own child nodes). **This whole tree was replaced on 2026-02-24** (commit `5c65f73e`, "WorkflowEditorPage now renders Board instead of WorkflowCanvas"). Confirmed by grep: `WorkflowCanvas` and the `@/components/canvas` barrel export are referenced only from within `components/canvas/` itself and its own tests — no page, route, or other component imports them.

**What the current Board actually reuses from the legacy `components/canvas/` tree** — this is why the two directories aren't fully independent:
- `Board.tsx` and `SubmitBar.tsx` import the `useWorkflowRun` hook from `@/components/canvas/useWorkflowRun` (run-status polling logic, no UI).
- `board/dispatch/AgentTraceCard.tsx` imports `ToolCallCard` from `@/components/canvas/CanvasNode/tabs/dispatch/ToolCallCard`.
- `board/dispatch/DispatchAccordionRow.tsx` imports `DispatchTraceView` from `@/components/canvas/CanvasNode/tabs/dispatch/DispatchTraceView`.

Those are narrow, presentational/leaf reuses (a hook and two trace-rendering components) — not evidence that the two systems share a rendering pipeline. Everything else in `components/canvas/` (`WorkflowCanvas`, the placement/auto-layout engine, `CanvasFormNode`, the `NodeVariant` registry, `mappers/agentArtifactNodes.ts`, `OptionTray`, focus mode's use of `canvasStore`) is dead weight from the pre-Board era, still present in the tree and still passing its own tests, but not reachable from the running app.

**Practical implication for anyone reading `execution_mode`-to-UI mapping:** don't look in `CanvasNode/registry.ts` for how the *current* app treats a step type — that registry describes the legacy React-Flow node system. For the current app, execution-mode-aware rendering lives in the sidebar's step tree (`frontend/src/components/sidebar/buildStepTree.ts`, `StepTreeRow.tsx`) and in `Board.tsx` itself (e.g. picking the entry step by `execution_mode === 'input' | 'context'`).

---

## Node / Execution Mode System

`ExecutionMode` (`frontend/src/types/workflow.ts`) is the authoritative set:

```typescript
type ExecutionMode = 'workforce' | 'context' | 'input' | 'manager' | 'single' | 'container'
```

Note this is **not** the same list as the legacy canvas's `NodeVariant` (`workforce, manager, room, blank, agent, context, input, step`) — there is no `room` execution mode. Rooms are a separate concept: a `WorkflowStep` can carry a `room_id`, backed by their own `rooms`/`roomSessions` API groups and `roomStore`, layered on top of a step rather than being an execution mode of its own.

Current treatment per mode, as implemented (not as the legacy registry describes it):

- **`workforce`** — the flagship/primary archetype. `StepTreeRow.tsx` special-cases it (`isWorkforce`) to show its roster. Roster agents are not separate canvas nodes in the current system (that was the legacy `mappers/agentArtifactNodes.ts` behavior) — they render as nested `AgentEntry` rows in the sidebar's step tree (`buildStepTree.ts`), fetched via `workflowStore.fetchRoster(stepId)` / `RosterAgent[]`. Workforce steps get the richest treatment: dispatch/stream activity in the `DispatchPanel` (Dispatch and Run tabs), roster progress via `WORKFORCE_DESIGNER_PROGRESS` / `WORKFORCE_AGENT_PROGRESS` / `DESIGNER_AGENT_DESIGNED` WS events.
- **`context`, `input`** — pass-through/structural steps. Hidden from the step tree entirely (`HIDDEN_MODES` in `buildStepTree.ts`). `Board.tsx` picks the workflow's entry step by checking for an `input` step first, falling back to `context`.
- **`manager`** — also in `HIDDEN_MODES`; not shown as its own row in the current tree.
- **`single`** — a plain one-agent step; default icon treatment in the tree.
- **`container`** — modeled on the backend (`ExecutionMode` includes it, `WorkflowStep`/`RunStepResult` carry it) but has **no dedicated frontend rendering** — no icon, no special tree handling, no canvas treatment found anywhere in `components/`, `pages/`, or `hooks/`. It falls back to whatever generic handling untyped steps get. Backend-only-so-far.

---

## The Workflow-Agent Chat

`frontend/src/hooks/useWorkflowAgentChat.ts` is a current, primary feature with no analog in the old doc. It's a chat assistant that talks to the backend's board-level meta-agent and keeps the hand-drawn Board in sync with what the user asks for.

- `WorkflowEditorPage` calls `useWorkflowAgentChat(id)` and passes `messages`, `sendMessage`, `streaming`, `cancelChat`, `submitPanel` down into `WorkflowSidebar`'s "Chat" tab (`components/sidebar/WorkflowSidebar.tsx`, rendered via `ChatPanel`).
- On mount it calls `api.workflows.getOrCreateAgentSession(workflowId)` to get/create a dedicated session, then `api.sessions.getHistory(sessionId)` to hydrate prior messages (reconstructing tool calls and any previously-submitted interactive "panel" messages from stored `source_type: 'tool'` entries).
- Sending a message streams the response over **SSE** (`useSendSessionMessage`/`SSEEvent`, not the WebSocket topic system) with event types `token`/`message`/`content` (assistant text), `tool_start`/`tool_end` (tool calls, with a special case that suppresses the internal `render_panel` tool from the visible tool log), and `panel_render` (the agent asking the user to pick from an inline set of options — rendered as an interactive panel message; `submitPanel(messageId, selections)` answers it by resending as a normal chat message).
- Board sync itself happens independently via WebSocket `WORKFLOW_EVENT`s (`board_elements_updated`, `step_created`, `step_deleted`, `edge_created`, `edge_deleted`) consumed by `workflowStore.handleWsEvent` and `useCanvasSync` — the chat's SSE stream and the Board's WS-driven live sync are two separate channels that both end up mutating the same `workflowStore`/`boardElementStore` state.

---

## API Client Conventions

Typed endpoints live in `frontend/src/api/api.ts`, wrapping a base client (`baseApi.get/post/patch/put/del`). **Updates use `PATCH`** (`baseApi.patch`) for most resources — `rooms`, `collections`, and `protocols` are the exceptions, using `PUT` for their update methods (`baseApi.put`). Never call `baseApi.get`/`.post` etc. directly from a component — use the typed group.

The full set of groups on the exported `api` object:

```
auth, agents, tools, documents, sessions, chat, config, stats,
agentExecutions, outputSchemas, promptTemplates, costs, results,
workflows, contextResponse, modes, rooms, roomSessions, collections,
protocols, dispatch
```

There is no `pipelines` group — there is no pipeline/stage concept in this codebase at all (the earlier version of this doc described a "Pipeline" product that doesn't exist here).

Notes on specific groups:

- **`workflows`** is the largest group — CRUD for workflows/steps/edges/step-documents, roster agents, room step members, run/execution endpoints, `generate` (kick off Phase 0/async agent design), `submitBoard`/`getBoardElements` (the Board's persistence), `getStepDispatchHistory`, `getLiveState` (single source of truth for "what's happening right now," polled by `workflowLiveStore`), `getExecutionTimeline`, `getRunDetail`, `downloadRunFiles` (blob download via `fetch` + auth header, not the typed client), workshop endpoints (`getOrCreateWorkshop`, `executeWorkshopStep`, `getWorkshopStatus` — back the Agent Workshop page), and `rebase`/`listTemplates` (run templates).
- **`costs`** and **`results`** exist as real API groups with real backing stores (`costStore`, `resultStore`), but **neither store is consumed by any page or component** — verified by grep, the only reference to either store is the `stores/index.ts` barrel export. This is dead scaffolding, not a shipped feature.
- **`dispatch`** (`trace`, `listForStep`, `send`, `cancel`, `session`) is the API surface for the Dispatch system — see its own section below.
- **`contextResponse`** and **`modes`** are thin, single-method groups (`get`/`list`) with `unknown` response types — minimal/placeholder surface, not fleshed out.

---

## WebSocket Protocol

Connects via `frontend/src/contexts/WebSocketContext.tsx` (`WebSocketProvider`) to `WS_URL`, gated on having an auth token, with exponential-backoff+jitter reconnect and a 30s ping. It exposes `subscribe(topic, handler)`, `subscribeRun(runId)`, and `send(message)` — no other component talks to the raw socket.

**Wire format** (`frontend/src/types/ws.ts`) — every server broadcast is:

```typescript
type WsWireMessage<T> = {
  topic: 'workflow' | 'room' | 'session'
  event: string
  ts: string
  run_id: string | null
  user_id: string | null
  seq: number | null
  data: T
}
```

Plus control messages (`subscribed`, `error`, `pong`, `events_missed`) sent directly to the socket rather than broadcast.

Central routing happens in `frontend/src/stores/ws/WsStoreRouter.tsx`, a headless component mounted once that subscribes each domain store to the topics it cares about:

```
WORKFLOW → workflowExecutionStore, workflowStore, stepStreamStore, agentTraceStore
SESSION  → sessionStore, dispatchStore
ROOM     → roomStore
(all)    → activityStore   — a flight recorder that logs every event regardless of topic
```

On `events_missed` (server-side lag notification), it re-fetches from REST rather than trusting the socket to have delivered everything (`sessionStore.fetchAll()`, `workflowStore.fetchIfStale()`, `workflowLiveStore.hydrateActive()`).

Real event names, grouped by topic (`WORKFLOW_EVENT`, `ROOM_EVENT`, `SESSION_EVENT` constants in `types/ws.ts`) — none of these existed in the old doc (`pipeline_run_update`, `stage_execution_update`, `agent_execution_update`, `for_each_spawned` are all fictional):

- **`WORKFLOW_EVENT`**: `started`, `step_started`, `step_completed`, `step_failed`, `step_paused`, `for_each_progress`, `completed`, `failed`, `resumed`, `step_config_updated`, `step_name_updated`, `roster_changed`, `room_members_changed`, `plan_updated`, `workforce_designer_progress`, `workforce_agent_progress`, `designer_agent_designed`, `step_pin_changed`, `step_created`, `step_deleted`, `edge_created`, `edge_deleted`, `board_elements_updated`, `step_stream_token`, `step_stream_tool_start`, `step_stream_tool_end`, `step_stream_error`, `debug_system_prompt`, `debug_user_message`, `debug_assistant_message`, `debug_tool_call`, `debug_tool_result`.
- **`ROOM_EVENT`**: `speaker_start`, `speaker_token`, `speaker_end`, `turn_complete`, `session_complete`.
- **`SESSION_EVENT`**: `created`, `updated`, `deleted`, `agent_message`, `dispatch_started`, `dispatch_progress`, `dispatch_completed`, `dispatch_failed`, `dispatch_cancelled`, `dispatch_stream_token`, `dispatch_stream_tool_start`, `dispatch_stream_tool_end`, `dispatch_stream_error`, `dispatch_stream_system_prompt`, `dispatch_stream_user_message`.

Client → server messages (`WsClientMessage`): `subscribe`/`unsubscribe` (topics), `subscribe_run`/`unsubscribe_run` (run_id), `ping`, and a small set of canvas-sync messages (`canvas_element_moved`, `canvas_text_changed`, `canvas_node_created`, `canvas_edge_created`, `canvas_node_deleted`, `canvas_edge_deleted`) used by the Board's live-sync hook (`components/board/hooks/useCanvasSync.ts`) to push local edits to the backend in real time.

---

## The Dispatch System

There is no "Tool Router" system anywhere in the current codebase (no router configs, no context-accumulation entries, no `router_request_update`/`context_update` events — none of that exists). The closest real analog is **Dispatch**, and it's shaped differently: it's per-step, ad-hoc agent task tracing, not an LLM-based tool-routing/context-injection layer.

- **Types**: `frontend/src/types/dispatch.ts` — `DispatchTraceResponse` (a step's execution trace: `ApiTraceEvent[]` of `token`/`tool_start`/`tool_end`/`error`/`system_prompt`/`user_message`), `DispatchTaskSummary`/`DispatchTasksResponse` (per-step task list), `DispatchSendRequest`/`DispatchActionResponse`/`DispatchSessionResponse`.
- **API**: the `dispatch` group — `trace(executionId)`, `listForStep(stepId)`, `send(stepId, { instruction, workflow_id })`, `cancel(executionId)`, `session(stepId)`.
- **Live events**: the `dispatch_*` and `dispatch_stream_*` `SESSION_EVENT`s above, keyed by `session_id`/`execution_id`/`step_id`, consumed by `dispatchStore.handleWsEvent`.
- **UI**: the Board's floating `DispatchPanel` (`components/board/dispatch/DispatchPanel.tsx`), a resizable overlay with two tabs — **Dispatch** (`DispatchTab.tsx`: `PhaseZeroSummary` + one `DispatchAccordionRow` per dispatch, sourced from `workflowLiveStore.selectDispatches`, exportable as JSON) and **Run** (`RunTab.tsx`). Individual trace rendering (tool calls, streamed tokens) reuses `ToolCallCard`/`DispatchTraceView` from the legacy `components/canvas/CanvasNode/tabs/dispatch/` tree, per the reuse note above.

---

## State Management

State is `stores/` — a lightweight zustand-style pattern (`frontend/src/stores/lib/createStore.ts` + `useStore(store, selector, equalityFn?)`), not React Context. Each store is a plain module exporting `store` (the `StoreApi`) plus named selector/action functions; components read with `useStore(someStore.store, someStore.selectThing)`.

Only three things are actual React Contexts, reserved for genuinely cross-cutting concerns:

| Context | Purpose |
|---------|---------|
| `CommandPaletteContext` | Global command palette (cmd-K) state |
| `ThemeModeContext` | Light/dark theme toggle |
| `WebSocketContext` | The single shared socket connection — `subscribe`/`subscribeRun`/`send` |

Notable stores (non-exhaustive; see `frontend/src/stores/index.ts` for the full barrel):

- `workflowStore` — steps, edges, roster, WS event handling for structural changes.
- `boardElementStore` / `boardStore` — the Board's own element state and last-submit/Phase-0 response (`elementStepMap`, `elementEdgeMap`).
- `workflowLiveStore` — polled "what's running right now" snapshot (`getLiveState`), generation flag, dispatches.
- `workflowExecutionStore`, `stepStreamStore`, `agentTraceStore` — execution/streaming state fed by `WORKFLOW_EVENT`s.
- `dispatchStore`, `dispatchSessionStore` — Dispatch system state fed by `SESSION_EVENT`s.
- `sidebarStore` — tree/chat tab, selected step, expand/collapse, panel width.
- `roomStore`, `collectionStore`, `protocolStore` — their respective domains.
- `costStore`, `resultStore` — real stores backing real endpoints, but currently unused by any UI (see API Client Conventions above).
- `wsConnectionStore`, `undoStore`, `activityStore` (the flight recorder), `uiStore`, `layoutStore`, `canvasStore` — the last of these, `canvasStore`, is legacy-canvas state (`PanelKind`, drag/interaction mode for React Flow) and is only consumed inside the orphaned `components/canvas/` and `components/panels/` trees.

---

## Build Tooling

- **Vite** (`frontend/vite.config.ts`): React plugin, `@` → `src` path alias, dev server on port 5173 proxying `/api` and `/ws` to `http://localhost:3000`. Vitest config is colocated in the same file (`jsdom` environment, `src/test/setup.ts`).
- **`package.json` scripts**: `dev` (vite), `build` (`tsc -b && vite build`), `lint` (eslint), `preview`, `test`/`test:watch` (vitest), `e2e`/`e2e:ui`/`e2e:headed` (Playwright).
- **TypeScript**: split config — `tsconfig.json` (references), `tsconfig.app.json` (app source), `tsconfig.node.json` (Vite config itself).
