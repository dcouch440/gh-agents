type AgentExecution = {
  id: string
  stage_execution_id: string
  agent_id: string
  workflow_step_id: string | null
  is_interactive: boolean
  parent_agent_execution_id: string | null
  system_prompt_rendered: string
  input: string
  output: string | null
  structured_output: Record<string, unknown> | null
  status: AgentExecutionStatus
  input_tokens: number
  output_tokens: number
  cost_usd: number
  started_at: string
  completed_at: string | null
}

type AgentExecutionStatus = 'pending' | 'running' | 'completed' | 'awaiting_user' | 'failed'

type ExecutionMessage = {
  id: string
  agent_execution_id: string
  role: 'system' | 'user' | 'assistant' | 'tool'
  content: string
  tool_call_id: string | null
  input_tokens: number
  output_tokens: number
  created_at: string
}

type TreeRunInfo = {
  id: string
  pipeline_id: string
  pipeline_name: string
  status: string
  initial_input: string
  current_stage: number
  started_at: string
  completed_at: string | null
  total_input_tokens: number
  total_output_tokens: number
  total_cost_usd: number
}

type TreeStage = {
  stage_number: number
  stage_name: string
  status: string
  stage_executions: TreeStageExecution[]
}

type TreeStageExecution = {
  id: string
  workflow_name: string
  status: string
  agent_executions: TreeAgentExecution[]
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

type ExecutionTree = {
  run: TreeRunInfo
  stages: TreeStage[]
}

type SendExecutionMessageRequest = {
  content: string
}

type ApproveExecutionRequest = {
  structured_output?: Record<string, unknown>
}

export type {
  AgentExecution,
  AgentExecutionStatus,
  ExecutionMessage,
  TreeRunInfo,
  TreeStage,
  TreeStageExecution,
  TreeAgentExecution,
  ExecutionTree,
  SendExecutionMessageRequest,
  ApproveExecutionRequest,
}
