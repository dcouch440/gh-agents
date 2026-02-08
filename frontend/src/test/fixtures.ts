import type { Agent } from '@/types/agent'
import type { Task } from '@/types/task'
import type { FeedItem } from '@/types/feed'
import type { UsageSummary } from '@/types/stats'
import type { ChatMessage, Session, Mode } from '@/types/session'
import type { RoutingEvent } from '@/types/routing'
import type { Document } from '@/types/document'
import type { Tool } from '@/types/tool'
import type { Config } from '@/types/config'
import type { CostResponse } from '@/types/cost'
import type { Result } from '@/types/result'
import type { PromptTemplate } from '@/types/template'
import type { OutputSchema } from '@/types/schema'
import type { Workflow, WorkflowStep, WorkflowStepEdge } from '@/types/workflow'
import type { RouterMode } from '@/types/router'
import type { ToolRouter } from '@/types/toolRouter'

export const mockAgent: Agent = {
  id: 'agent-001',
  name: 'TestBot',
  system_prompt: 'You are a test agent.',
  model_provider: 'anthropic',
  model_id: 'claude-sonnet-4-20250514',
  model_max_tokens: 8192,
  model_temperature: 0.7,
  status: 'idle',
  output_schema_id: null,
  router_id: null,
  version: 1,
}

export const mockAgentUpdated: Agent = {
  ...mockAgent,
  status: 'working',
}

export const mockTask: Task = {
  id: 'task-001',
  slice_id: null,
  title: 'Test task',
  description: 'A task for testing',
  assigned_agent: null,
  status: 'pending',
  priority: 'normal',
  context_files: [],
  metadata: null,
  depends_on: [],
  retry_count: 0,
  max_retries: 3,
  last_error: null,
  created_at: '2025-01-01T00:00:00Z',
  updated_at: '2025-01-01T00:00:00Z',
}

export const mockTaskCompleted: Task = {
  ...mockTask,
  status: 'completed',
  updated_at: '2025-01-01T01:00:00Z',
}

export const mockFeedItem: FeedItem = {
  id: 'feed-001',
  agent_id: 'agent-001',
  content: 'Agent started working',
  item_type: 'task_started',
  verbosity_level: 'normal',
  timestamp: '2025-01-01T00:00:00Z',
}

export const mockUsageSummary: UsageSummary = {
  model_id: 'claude-sonnet-4-20250514',
  total_input: 5000,
  total_output: 2000,
  call_count: 10,
}

export const mockChatMessage: ChatMessage = {
  id: 'msg-001',
  role: 'user',
  content: 'Hello agent',
  timestamp: '2025-01-01T00:00:00Z',
}

export const mockAssistantMessage: ChatMessage = {
  id: 'msg-002',
  role: 'assistant',
  content: 'Hello human',
  timestamp: '2025-01-01T00:00:01Z',
}

export const mockRoutingEvent: RoutingEvent = {
  id: 'route-001',
  user_id: 'user-001',
  session_id: 'session-001',
  task_id: null,
  router_agent_id: 'agent-001',
  tool_name: 'search_files',
  request: 'Find the auth module',
  parameters: {},
  response: null,
  error: null,
  status: 'pending',
  model_id: 'claude-sonnet-4-20250514',
  input_tokens: 100,
  output_tokens: 0,
  duration_ms: null,
  created_at: '2025-01-01T00:00:00Z',
  completed_at: null,
}

export const mockRoutingEventCompleted: RoutingEvent = {
  ...mockRoutingEvent,
  status: 'completed',
  response: 'Found src/auth/mod.rs',
  output_tokens: 50,
  duration_ms: 1200,
  completed_at: '2025-01-01T00:00:01Z',
}

export const mockSession: Session = {
  id: 'session-001',
  user_id: 'user-001',
  mode_id: 'home',
  title: 'Test session',
  summary: 'A test session',
  created_at: '2025-01-01T00:00:00Z',
  updated_at: '2025-01-01T00:00:00Z',
}

export const mockMode: Mode = {
  id: 'home',
  name: 'Home',
  description: 'Default mode',
}

export const mockDocument: Document = {
  id: 'doc-001',
  user_id: 'user-001',
  session_id: null,
  title: 'Test document',
  content: 'Document content here',
  summary: 'A test doc',
  doc_type: 'note',
  ref_tag: 'test-doc',
  tags: ['test'],
  created_at: '2025-01-01T00:00:00Z',
  updated_at: '2025-01-01T00:00:00Z',
}

export const mockTool: Tool = {
  id: 'tool-001',
  name: 'search_files',
  description: 'Search for files in the codebase',
  category: 'codebase',
  parameter_schema: {},
  output_schema: {},
  enabled: true,
  is_builtin: true,
}

export const mockRouterMode: RouterMode = {
  id: 'mode-001',
  router_id: 'router-001',
  mode_key: 'planning_mode',
  display_name: 'Planning Mode',
  description: 'Mode for strategic planning tasks',
  system_prompt: 'You are a strategic planner.',
  temperature: 0.7,
  max_tokens: 8192,
  append_to_agent_system_prompt: false,
  append_to_agent_tools: true,
  display_order: 0,
  created_at: '2025-01-01T00:00:00Z',
  updated_at: '2025-01-01T00:00:00Z',
}

export const mockToolRouter: ToolRouter = {
  id: 'router-001',
  user_id: 'user-001',
  name: 'Default Router',
  description: 'Main tool router',
  system_prompt: 'You are a tool router.',
  model_id: 'claude-sonnet-4-20250514',
  is_active: true,
  parent_router_id: null,
  level: 1,
  created_at: '2025-01-01T00:00:00Z',
  updated_at: '2025-01-01T00:00:00Z',
}

export const mockPromptTemplate: PromptTemplate = {
  id: 'template-001',
  user_id: 'user-001',
  name: 'Test Template',
  description: 'A test prompt template',
  template: 'Hello {{name}}, please {{action}}',
  variables: ['name', 'action'],
  created_at: '2025-01-01T00:00:00Z',
  updated_at: '2025-01-01T00:00:00Z',
}

export const mockCostResponse: CostResponse = {
  total_spend: 0.15,
  models: [{ model_id: 'claude-sonnet-4-20250514', total_input_tokens: 10000, total_output_tokens: 5000, total_cost_usd: 0.15, call_count: 10 }],
}

const mockModelConfig = { provider: 'anthropic', model_id: 'claude-sonnet-4-20250514', max_tokens: 8192, temperature: 0.7 }

export const mockConfig: Config = {
  verbosity: 'normal',
  models: {
    orchestrator: { ...mockModelConfig, model_id: 'claude-opus-4-5-20251101', max_tokens: 16384 },
    worker: mockModelConfig,
    utility: { ...mockModelConfig, model_id: 'claude-3-5-haiku-20241022', max_tokens: 4096 },
  },
  pool: { max_agents: 12 },
  autonomy: 'supervised',
  git_strategy: 'branch',
  sandbox_mode: 'docker',
}

export const mockResult: Result = {
  id: 'result-001',
  agent_execution_id: 'exec-001',
  output_schema_id: null,
  name: 'Test Result',
  data: { status: 'success', message: 'Task completed successfully' },
  created_at: '2025-01-01T00:00:00Z',
}

export const mockWorkflow: Workflow = {
  id: 'workflow-001',
  user_id: 'user-001',
  name: 'Test Workflow',
  description: 'A test workflow',
  created_at: '2025-01-01T00:00:00Z',
  updated_at: '2025-01-01T00:00:00Z',
}

export const mockWorkflowStep: WorkflowStep = {
  id: 'step-001',
  workflow_id: 'workflow-001',
  name: 'First Step',
  description: 'The first step',
  step_type: 'llm',
  agent_id: 'agent-001',
  prompt_template_id: null,
  output_schema_id: null,
  for_each_label_field: null,
  config: null,
  position_x: 0,
  position_y: 0,
  created_at: '2025-01-01T00:00:00Z',
  updated_at: '2025-01-01T00:00:00Z',
}

export const mockWorkflowStep2: WorkflowStep = {
  id: 'step-002',
  workflow_id: 'workflow-001',
  name: 'Second Step',
  description: null,
  step_type: 'for_each',
  agent_id: null,
  prompt_template_id: null,
  output_schema_id: null,
  for_each_label_field: 'label',
  config: null,
  position_x: 400,
  position_y: 100,
  created_at: '2025-01-01T00:00:00Z',
  updated_at: '2025-01-01T00:00:00Z',
}

export const mockWorkflowEdge: WorkflowStepEdge = {
  id: 'edge-001',
  workflow_id: 'workflow-001',
  from_step_id: 'step-001',
  to_step_id: 'step-002',
  condition: null,
  created_at: '2025-01-01T00:00:00Z',
}

export const mockOutputSchema: OutputSchema = {
  id: 'schema-001',
  user_id: 'user-001',
  name: 'Test Schema',
  description: 'A test output schema',
  json_schema: { type: 'object', properties: { result: { type: 'string' } } },
  created_at: '2025-01-01T00:00:00Z',
  updated_at: '2025-01-01T00:00:00Z',
}

// ── Execution ────────────────────────────────────────────────────────────────

import type { AgentExecution, TreeAgentExecution, ExecutionMessage } from '@/types/execution'

export const mockAgentExecution: AgentExecution = {
  id: 'exec-001',
  stage_execution_id: 'stage-001',
  agent_id: 'agent-001',
  workflow_step_id: 'step-001',
  is_interactive: true,
  parent_agent_execution_id: null,
  system_prompt_rendered: 'Review the code changes',
  input: 'Please review the following changes...',
  output: 'The changes look good overall.',
  structured_output: null,
  selected_mode_id: null,
  status: 'awaiting_user',
  started_at: '2025-01-01T00:00:00Z',
  completed_at: null,
}

export const mockTreeAgentExecution: TreeAgentExecution = {
  id: 'agent-exec-001',
  agent_name: 'TestBot',
  workflow_step_id: null,
  is_interactive: false,
  status: 'running',
  structured_output: null,
  input_tokens: 500,
  output_tokens: 0,
  cost_usd: 0,
  started_at: '2025-01-01T00:00:00Z',
  completed_at: null,
  for_each_index: null,
  for_each_label: null,
  interactive_review: null,
}

export const mockExecutionMessage: ExecutionMessage = {
  id: 'msg-001',
  agent_execution_id: 'agent-exec-001',
  role: 'user',
  content: 'Please proceed with the task',
  tool_call_id: null,
  input_tokens: 0,
  output_tokens: 0,
  created_at: '2025-01-01T00:00:00Z',
}
