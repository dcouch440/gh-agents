type Workflow = {
  id: string
  name: string
  description: string | null
  created_at: string
  container_enabled: boolean
  target_repo_url: string | null
  target_branch: string | null
  vpn_enabled: boolean
}

type WorkflowStep = {
  id: string
  workflow_id: string
  agent_id: string
  execution_mode: string
  for_each_ref: string | null
  prompt_template_id: string | null
  prompt_template: string
  output_schema_id: string | null
  output_variable_name: string | null
  interactive_agent_id: string | null
  for_each_label_field: string | null
  display_order: number
  version: number
  reasoning_trace: boolean
  verification_agent_ids: string[]
  position_x: number | null
  position_y: number | null
  name: string | null
  system_prompt_suffix: string | null
}

type WorkflowStepEdge = {
  id: string
  from_step_id: string
  to_step_id: string
}

type StepDocument = {
  id: string
  workflow_id: string
  step_id: string
  document_id: string
  usage: string
  created_at: string
}

type CreateWorkflowRequest = {
  name: string
  description?: string
}

type UpdateWorkflowRequest = Partial<CreateWorkflowRequest>

type CreateStepRequest = {
  agent_id?: string
  execution_mode?: string
  position_x?: number
  position_y?: number
  name?: string
  prompt_template_id?: string
  output_schema_id?: string
  for_each_label_field?: string
  for_each_ref?: string
  prompt_template?: string
  output_variable_name?: string
  interactive_agent_id?: string
  display_order?: number
  reasoning_trace?: boolean
  verification_agent_ids?: string[]
  system_prompt_suffix?: string
}

type UpdateStepRequest = Partial<CreateStepRequest>

type EdgeRequest = {
  from_step_id: string
  to_step_id: string
}

type StepDocumentRequest = {
  document_id: string
  usage: string
}

export type {
  Workflow,
  WorkflowStep,
  WorkflowStepEdge,
  StepDocument,
  CreateWorkflowRequest,
  UpdateWorkflowRequest,
  CreateStepRequest,
  UpdateStepRequest,
  EdgeRequest,
  StepDocumentRequest,
}
