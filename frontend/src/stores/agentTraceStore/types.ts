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
}

export type { AgentTraceEvent, AgentTrace, AgentTraceState }
