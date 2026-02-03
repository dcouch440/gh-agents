type Result = {
  id: string
  agent_execution_id: string
  output_schema_id: string | null
  name: string
  data: Record<string, unknown>
  created_at: string
}

export type { Result }
