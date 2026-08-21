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
   * Run we have already asked the timeline endpoint about.
   *
   * Tracked separately from `hydratedRunId` because a run that has not produced
   * a message yet answers with nothing, leaving `order` empty — without this the
   * poller would re-ask every tick and burn rate-limit budget on an endpoint
   * that has already told us it has nothing.
   */
  timelineAttemptedRunId: string | null
}

export type { AgentTraceEvent, AgentTrace, AgentTraceState }
