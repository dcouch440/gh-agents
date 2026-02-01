type PipelineStage = {
  stage_number: number
  agent_id: string | null
  cluster_id: string | null
  role: string | null
  approval_required: boolean
  fan_out: boolean
  stage_name: string
  input_definitions: Record<string, string>
  output_description: string
  output_schema: unknown
}

type Pipeline = {
  id: string
  name: string
  stages: PipelineStage[]
}

type PipelineRun = {
  id: string
  pipeline_id: string
  user_id: string
  status: string
  initial_task: string
  stage_outputs: Record<string, unknown>
  current_stage: number
  started_at: string
  completed_at: string | null
  total_input_tokens: number
  total_output_tokens: number
}

type StageExecution = {
  id: string
  run_id: string
  stage_number: number
  stage_name: string
  agent_id: string | null
  status: string
  rendered_prompt: string | null
  output: string | null
  structured_output: unknown | null
  user_input: string | null
  input_tokens: number
  output_tokens: number
  started_at: string
  completed_at: string | null
  duration_ms: number
}

export type { Pipeline, PipelineStage, PipelineRun, StageExecution }
