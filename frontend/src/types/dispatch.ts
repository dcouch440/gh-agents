// ── Dispatch Trace API Response Types ──────────────────────────────────────

// Raw API trace events use snake_case (from Rust serde)
type ApiTraceEvent =
  | { type: 'token'; content: string; ts: string }
  | { type: 'tool_start'; tool_name: string; tool_id: string; input: Record<string, unknown>; ts: string }
  | { type: 'tool_end'; tool_name: string; tool_id: string; result: unknown; ts: string }
  | { type: 'error'; error: string; ts: string }
  | { type: 'system_prompt'; content: string; agent_name: string | null; ts: string }

export type DispatchTraceResponse = {
  execution_id: string
  step_id: string
  workflow_id: string
  status: string
  instruction: string
  trace: ApiTraceEvent[]
  result: string | null
}

export type DispatchTaskSummary = {
  execution_id: string
  step_id: string
  status: string
  instruction: string
  result: string | null
  trace_len: number
  created_at: string
}

export type DispatchTasksResponse = {
  tasks: DispatchTaskSummary[]
}

export type DispatchSendRequest = {
  instruction: string
  workflow_id: string
}

export type DispatchActionResponse = {
  execution_id: string
  status: string
}

export type DispatchSessionResponse = {
  session_id: string
}

export type { ApiTraceEvent }
