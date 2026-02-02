# Frontend: Sync Types and Hooks with Backend Chat Hub Refactor

## Background

The backend chat system was refactored over several iterations. The old `ModeRegistry` (hardcoded modes) was replaced with **DB-backed agents** (`persisted_agents` table). A unified execution engine (`hub::ExecutionEngine`) now powers all LLM interactions. Cancellation support was added to pipelines and agent executions. The `token_ledger` table is now the single source of truth for cost/token data — `agent_executions` no longer carries `input_tokens`, `output_tokens`, or `cost_usd` columns.

The frontend types, hooks, and contexts have not been updated to reflect these changes.

---

## What Changed on the Backend

### 1. Modes are now Agents

`GET /api/modes` still works but now returns rows from `persisted_agents`. Each "mode" is just an agent. The response shape is unchanged:

```json
{ "id": "uuid", "name": "string", "description": "string" }
```

The `id` field is now a `Uuid` (serialized as string). No frontend type change needed for `Mode`.

### 2. Sessions have an `agent_id` field

`SessionResponse` now returns:

```json
{
  "id": "uuid",
  "mode_id": "string",
  "agent_id": "uuid | null",   // <-- NEW
  "title": "string",
  "created_at": "datetime",
  "updated_at": "datetime"
}
```

`CreateSessionRequest` accepts an optional `agent_id`:

```json
{ "mode_id": "string", "agent_id": "uuid | null", "title": "string" }
```

**Frontend `Session` type** is missing `agent_id`. `CreateSessionRequest` is missing `agent_id`.

### 3. `AgentExecution` no longer has token/cost fields

The backend `AgentExecutionResponse` no longer returns `input_tokens`, `output_tokens`, or `cost_usd`. These fields were removed from the API response.

**Frontend `AgentExecution` type** still has `input_tokens`, `output_tokens`, `cost_usd` — remove them.

### 4. `TreeAgentExecution` still has token fields (from token_ledger joins)

The backend `TreeAgentExecution` still returns `input_tokens`, `output_tokens`, `cost_usd` — these are populated from `token_ledger` joins in the tree query, not from `agent_executions` columns. **No change needed** for the frontend `TreeAgentExecution` type.

Similarly, `TreeRunInfo` still returns `total_input_tokens`, `total_output_tokens`, `total_cost_usd` from ledger aggregation. **No change needed.**

### 5. `AgentExecutionStatus` is missing `cancelled`

The backend now supports `cancelled` as a valid status (from the new cancellation feature). The frontend type should be:

```typescript
type AgentExecutionStatus = 'pending' | 'running' | 'completed' | 'awaiting_user' | 'failed' | 'cancelled'
```

### 6. No API endpoint changes for chat flow

The chat endpoints are unchanged:
- `POST /api/chat` — global chat
- `POST /api/sessions/{id}/chat` — session chat
- `GET /api/sessions/{id}/history` — session history
- `GET /api/sessions/{id}/chat/{messageId}/stream` — SSE stream
- `GET /api/chat/{messageId}/stream` — global SSE stream

Request/response shapes for sending messages and streaming are unchanged.

### 7. New cancel endpoints

Two new endpoints exist:

- `POST /api/pipeline-runs/{id}/cancel` — cancel a pipeline run (cascades to all stages/executions)
- `POST /api/agent-executions/{id}/cancel` — cancel a single agent execution

Both return `200` with `{"status": "cancelled"}` on success, or `404`/`409` if not found or not in a cancellable state.

---

## Required Changes

### Types (`frontend/src/types/`)

**`session.ts`:**
- Add `agent_id: string | null` to `Session`
- Add `agent_id?: string` to `CreateSessionRequest`

**`execution.ts`:**
- Remove `input_tokens`, `output_tokens`, `cost_usd` from `AgentExecution`
- Add `'cancelled'` to `AgentExecutionStatus` union
- `TreeAgentExecution` and `TreeRunInfo` — no change needed (still served from ledger)
- `ExecutionMessage` — `input_tokens` and `output_tokens` are still returned by the backend, no change needed

### Hooks (`frontend/src/hooks/`)

- If any hook reads `agentExecution.input_tokens`, `.output_tokens`, or `.cost_usd` — remove those references
- If `useSessionMutations.ts` `useCreateSession` doesn't support passing `agent_id`, add it to the mutation payload
- Add a `useCancelPipelineRun` and/or `useCancelAgentExecution` hook if cancel UI is desired (endpoints: `POST /api/pipeline-runs/{id}/cancel`, `POST /api/agent-executions/{id}/cancel`)

### Constants (`frontend/src/constants.ts`)

Add the cancel endpoint paths:
```typescript
PIPELINE_RUN_CANCEL: (id: string) => `/pipeline-runs/${id}/cancel`,
AGENT_EXECUTION_CANCEL: (id: string) => `/agent-executions/${id}/cancel`,
```

### Components

- Any component displaying `input_tokens`/`output_tokens`/`cost_usd` from an `AgentExecution` should either be removed or switched to read from the execution tree (which still has these from ledger joins)
- Any status badge/indicator should handle the `cancelled` status (color, label, icon)
- The pipeline run detail view (if it exists) could add a cancel button using the new endpoints

### Context

- `ChatContext` — no changes needed, the message shape hasn't changed
- `WebSocketContext` — no changes needed, channel names are the same

---

## Verification

```bash
npx tsc --noEmit    # Must pass with zero errors
npx eslint .        # Must pass with zero warnings
npx vitest run      # All tests must pass
```
