import type { DispatchTraceEvent } from '@/stores/dispatchStore'

// ── Dispatch Trace API Response Types ──────────────────────────────────────

export type DispatchTraceResponse = {
  execution_id: string
  step_id: string
  workflow_id: string
  status: string
  instruction: string
  trace: DispatchTraceEvent[]
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
