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
  width: number | null
  height: number | null
  name: string | null
  room_id: string | null
  system_prompt_suffix: string | null
  description: string
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
  width?: number
  height?: number
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
  description?: string
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

type WorkflowRunResponse = {
  execution_id: string
  workflow_id: string
  status: string
}

type WorkflowExecutionSummary = {
  id: string
  workflow_id: string
  status: string
  started_at: string | null
  completed_at: string | null
  outputs: Record<string, unknown> | null
  error: string | null
  execution_mode: string
  template_id: string | null
}

type DocumentDef = {
  id: string
  step_id: string
  name: string
  description: string
  target_length: number
  display_order: number
  created_at: string
  document_id: string | null
}

type CreateDocumentDefRequest = {
  name: string
  description?: string
  target_length?: number
  display_order?: number
}

type UpdateDocumentDefRequest = {
  name?: string
  description?: string
  target_length?: number
}

type RosterAgent = {
  id: string
  name: string
  role_description: string
  capabilities: string[]
  execution_order: number
  created_at: string
}

type CreateRosterAgentRequest = {
  name: string
  role_description?: string
  capabilities?: string[]
  execution_order?: number
}

type RoomStepMember = {
  id: string
  name: string
  role: string
  perspective: string
  display_order: number
}

type StepChatDebugResponse = {
  system_prompt: string
  messages: { role: string; content: string }[]
}

type PhaseExecution = {
  id: string
  phase: string
  document_name: string | null
  status: string
  output_content: string | null
  input_tokens: number | null
  output_tokens: number | null
  cost_usd: number | null
  model: string | null
  started_at: string
  completed_at: string | null
  error_message: string | null
}

type StepLastRunResponse = {
  execution_id: string
  workflow_execution_id: string
  status: string
  started_at: string | null
  completed_at: string | null
  duration_ms: number | null
  output: string | null
  structured_output: Record<string, unknown> | null
  input_tokens: number | null
  output_tokens: number | null
  cost_usd: number | null
  error: string | null
  phases: PhaseExecution[] | null
}

type RunStepResult = {
  step_id: string
  step_name: string | null
  execution_mode: string
  execution_id: string | null
  status: string
  started_at: string | null
  completed_at: string | null
  duration_ms: number | null
  output: string | null
  structured_output: Record<string, unknown> | null
  input_tokens: number | null
  output_tokens: number | null
  cost_usd: number | null
  error: string | null
  phases: PhaseExecution[] | null
}

type RunDetailResponse = {
  execution: WorkflowExecutionSummary
  steps: RunStepResult[]
  total_input_tokens: number
  total_output_tokens: number
  total_cost_usd: number
  duration_ms: number | null
  template_name: string | null
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
  WorkflowRunResponse,
  WorkflowExecutionSummary,
  DocumentDef,
  CreateDocumentDefRequest,
  UpdateDocumentDefRequest,
  RosterAgent,
  CreateRosterAgentRequest,
  RoomStepMember,
  StepChatDebugResponse,
  PhaseExecution,
  StepLastRunResponse,
  RunStepResult,
  RunDetailResponse,
}
