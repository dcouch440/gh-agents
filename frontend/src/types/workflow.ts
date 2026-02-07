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
  name: string
  description: string | null
  step_type: string
  agent_id: string | null
  prompt_template_id: string | null
  output_schema_id: string | null
  for_each_label_field: string | null
  config: Record<string, unknown> | null
  position_x: number
  position_y: number
  created_at: string
  updated_at: string
}

type WorkflowStepEdge = {
  id: string
  workflow_id: string
  from_step_id: string
  to_step_id: string
  condition: string | null
  created_at: string
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
  name: string
  description?: string
  step_type: string
  agent_id?: string
  prompt_template_id?: string
  output_schema_id?: string
  config?: Record<string, unknown>
  position_x?: number
  position_y?: number
}

type UpdateStepRequest = Partial<CreateStepRequest>

type EdgeRequest = {
  from_step_id: string
  to_step_id: string
  condition?: string
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
