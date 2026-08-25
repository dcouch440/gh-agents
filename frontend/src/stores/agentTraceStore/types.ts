type AgentTraceEvent =
  | { type: 'system_prompt'; content: string; ts: string }
  | { type: 'user_message'; content: string; ts: string }
  | { type: 'assistant_message'; content: string; ts: string }
  | { type: 'tool_call'; toolName: string; toolId: string; input: Record<string, unknown>; ts: string }
  | { type: 'tool_result'; toolName: string; toolId: string; result: string; ts: string }

type AgentTrace = {
  agentExecutionId: string
  agentName: string | null
  stepId: string
  events: AgentTraceEvent[]
}

type AgentTraceState = {
  traces: Record<string, AgentTrace>
  order: string[]
  /** Run these traces belong to. Null means nothing has run yet. */
  hydratedRunId: string | null
  /**
   * Run whose timeline has been fetched in full.
   *
   * Distinct from `hydratedRunId`, which is stamped before the fetch is made.
   * This is only set once the entries are in, so a caller can tell "we know
   * which run this is" apart from "we already have its timeline".
   */
  timelineRunId: string | null
}

export type { AgentTraceEvent, AgentTrace, AgentTraceState }
