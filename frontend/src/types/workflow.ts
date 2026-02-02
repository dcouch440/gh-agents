type Workflow = {
  id: string
  name: string
  description: string
  created_at: string
}

type WorkflowStep = {
  id: string
  workflow_id: string
  agent_id: string
  execution_mode: 'single' | 'for_each'
  for_each_ref: string | null
  prompt_template_id: string | null
  prompt_template: string
  output_schema_id: string | null
  output_variable_name: string | null
  interactive_agent_id: string | null
  for_each_label_field: string | null
  display_order: number
}

type WorkflowStepEdge = {
  from_step_id: string
  to_step_id: string
}

type StepDocument = {
  step_id: string
  document_id: string
}

type CreateWorkflowRequest = {
  name: string
  description?: string
}

type UpdateWorkflowRequest = Partial<CreateWorkflowRequest>

type CreateStepRequest = {
  agent_id: string
  execution_mode?: string
  for_each_ref?: string
  prompt_template_id?: string
  prompt_template?: string
  output_schema_id?: string
  output_variable_name?: string
  interactive_agent_id?: string
  for_each_label_field?: string
  display_order?: number
}

type UpdateStepRequest = Partial<CreateStepRequest>

type EdgeRequest = {
  from_step_id: string
  to_step_id: string
}

type StepDocumentRequest = {
  document_id: string
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
