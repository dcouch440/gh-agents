// ── Workflow live state ─────────────────────────────────────────────────────
//
// Response of `GET /workflows/:id/live-state` — one call describing everything
// currently happening on a workflow, so a page refresh can rebuild the editor.

import type { RunStepResult, WorkflowExecutionSummary } from './workflow'

type BaselineStatus = 'error' | 'completed' | 'configured' | 'described' | 'idle'

/**
 * Which store a dispatch came from, which decides where its trace lives.
 *
 * `registry` — in-memory, fetch via `api.dispatch.trace(executionId)`.
 * `persisted` — `execution_id` is an agent_execution id, so the trace must come
 * from `api.workflows.getStepDispatchHistory(workflowId, stepId)` instead.
 */
type DispatchSource = 'registry' | 'persisted'

/** Design-time truth for one node, independent of any run. */
type LiveStepBaseline = {
  step_id: string
  name: string | null
  execution_mode: string
  baseline_status: BaselineStatus
  pinned: boolean
  has_run_summary: boolean
  is_running_in_active_run: boolean
}

type LiveDispatchInfo = {
  step_id: string
  execution_id: string
  status: string
  instruction: string
  created_at: string
  result: string | null
  trace_len: number
  source: DispatchSource
}

type WorkflowLiveStateResponse = {
  workflow_id: string
  server_time: string
  active_run: WorkflowExecutionSummary | null
  latest_run: WorkflowExecutionSummary | null
  run_steps: RunStepResult[]
  steps: LiveStepBaseline[]
  dispatches: LiveDispatchInfo[]
  generating: boolean
}

// ── Execution timeline ──────────────────────────────────────────────────────

type TimelineEntryKind =
  | 'system_prompt'
  | 'user_message'
  | 'assistant_message'
  | 'tool_call'
  | 'tool_result'

type TimelineEntry = {
  id: string
  ts: string
  kind: TimelineEntryKind
  step_id: string | null
  step_name: string | null
  agent_name: string | null
  agent_execution_id: string
  content: string
  tool_name: string | null
  tool_call_id: string | null
  input_tokens: number
  output_tokens: number
}

type TimelineResponse = {
  entries: TimelineEntry[]
  has_more: boolean
  next_cursor: string | null
}

export type {
  BaselineStatus,
  DispatchSource,
  LiveStepBaseline,
  LiveDispatchInfo,
  WorkflowLiveStateResponse,
  TimelineEntryKind,
  TimelineEntry,
  TimelineResponse,
}
