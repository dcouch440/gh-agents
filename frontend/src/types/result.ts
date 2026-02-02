type Result = {
  id: string
  pipeline_run_id: string
  stage_number: number
  agent_execution_id: string | null
  output: string | null
  structured_output: Record<string, unknown> | null
  created_at: string
}

export type { Result }
