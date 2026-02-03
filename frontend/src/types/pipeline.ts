type PipelineStage = {
  stage_number: number
  agent_id: string | null
  cluster_id: string | null
  role: string | null
  approval_required: boolean | null
  fan_out: boolean | null
  stage_name: string
  input_definitions: Record<string, string> | null
  output_description: string | null
  output_schema: unknown
}

type StageMember = {
  id: string
  pipeline_id: string
  stage_number: number
  workflow_id: string
  display_order: number
}

type CreateStageMemberRequest = {
  workflow_id: string
  display_order?: number
}

type UpdateStageMemberRequest = {
  display_order: number
}

type Pipeline = {
  id: string
  name: string
  stages: PipelineStage[]
}

type PipelineRun = {
  id: string
  pipeline_id: string
  status: string
  initial_task: string
  stage_outputs: Record<string, unknown> | null
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
  structured_output: unknown
  user_input: string | null
  input_tokens: number
  output_tokens: number
  started_at: string
  completed_at: string | null
  duration_ms: number
}

type ApproveGateRequest = {
  user_input?: string
}

type CreateSideTaskRequest = {
  title: string
  description: string
}

export type { Pipeline, PipelineStage, PipelineRun, StageExecution, StageMember, ApproveGateRequest, CreateSideTaskRequest, CreateStageMemberRequest, UpdateStageMemberRequest }
