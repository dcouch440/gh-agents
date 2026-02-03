type RoutingEvent = {
  id: string
  user_id: string | null
  session_id: string | null
  task_id: string | null
  router_agent_id: string
  cluster_agent_id: string | null
  cluster_id: string | null
  cluster_name: string
  tool_name: string
  request: string
  parameters: Record<string, unknown>
  response: string | null
  error: string | null
  status: string
  model_id: string | null
  input_tokens: number
  output_tokens: number
  duration_ms: number | null
  created_at: string
  completed_at: string | null
}

export type { RoutingEvent }
