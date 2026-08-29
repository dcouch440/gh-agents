type ExecutionType =
  | 'dispatch'
  | 'manager_dispatch'
  | 'dag_step'
  | 'agent_designer'
  | 'pipeline_agent'
  | 'interactive_review'
  | 'debate_verification'

type AgentExecution = {
  id: string
  execution_type: ExecutionType
  stage_execution_id: string
  agent_id: string
  workflow_step_id: string | null
  is_interactive: boolean
  parent_agent_execution_id: string | null
  system_prompt_rendered: string
  input: string
  output: string | null
  structured_output: Record<string, unknown> | null
  selected_mode_id: string | null
  status: AgentExecutionStatus
  started_at: string
  completed_at: string | null
}

type AgentExecutionStatus = 'pending' | 'running' | 'completed' | 'awaiting_user' | 'failed' | 'cancelled'

type ExecutionMessage = {
  id: string
  agent_execution_id: string
  role: 'system' | 'user' | 'assistant' | 'tool'
  content: string
  reasoning: string | null
  tool_call_id: string | null
  input_tokens: number
  output_tokens: number
  created_at: string
}

type TreeAgentExecution = {
  id: string
  agent_name: string
  workflow_step_id: string | null
  is_interactive: boolean
  status: AgentExecutionStatus
  structured_output: Record<string, unknown> | null
  input_tokens: number
  output_tokens: number
  cost_usd: number
  started_at: string
  completed_at: string | null
  for_each_index: number | null
  for_each_label: string | null
  interactive_review: TreeAgentExecution | null
}

type SendExecutionMessageRequest = {
  content: string
}

type ApproveExecutionRequest = {
  structured_output?: Record<string, unknown>
}

type SendMessageResponse = {
  message: ExecutionMessage
  stream_id: string
}

export type {
  ExecutionType,
  AgentExecution,
  AgentExecutionStatus,
  ExecutionMessage,
  TreeAgentExecution,
  SendExecutionMessageRequest,
  ApproveExecutionRequest,
  SendMessageResponse,
}
