import type { Agent } from '@/types/agent'
import type { Task } from '@/types/task'
import type { Pipeline, PipelineRun, StageExecution } from '@/types/pipeline'
import type { FeedItem } from '@/types/feed'
import type { UsageSummary } from '@/types/stats'
import type { ChatMessage, Session, Mode } from '@/types/session'
import type { RoutingEvent } from '@/types/routing'
import type { Document } from '@/types/document'
import type { Tool } from '@/types/tool'
import type { Config } from '@/types/config'

export const mockAgent: Agent = {
  id: 'agent-001',
  tier: 'worker',
  persona: { name: 'TestBot', system_prompt: 'You are a test agent.', style: 'concise' },
  model_config: { provider: 'anthropic', model_id: 'claude-sonnet-4-20250514', max_tokens: 8192, temperature: 0.7 },
  status: 'idle',
  current_task: null,
  router_mode: false,
}

export const mockAgentUpdated: Agent = {
  ...mockAgent,
  status: 'working',
  current_task: 'task-001',
}

export const mockTask: Task = {
  id: 'task-001',
  slice_id: null,
  title: 'Test task',
  description: 'A task for testing',
  assigned_tier: 'worker',
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

export const mockPipeline: Pipeline = {
  id: 'pipeline-001',
  name: 'Test pipeline',
  stages: [],
}

export const mockPipelineRun: PipelineRun = {
  id: 'run-001',
  pipeline_id: 'pipeline-001',
  user_id: 'user-001',
  status: 'running',
  initial_task: 'Do the thing',
  stage_outputs: {},
  current_stage: 1,
  started_at: '2025-01-01T00:00:00Z',
  completed_at: null,
  total_input_tokens: 0,
  total_output_tokens: 0,
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
  tier: 'worker',
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
  session_id: 'session-001',
  router_agent_id: 'agent-001',
  cluster_agent_id: 'agent-002',
  cluster_name: 'codebase',
  tool_name: 'search_files',
  request: 'Find the auth module',
  parameters: {},
  response: null,
  error: null,
  status: 'pending',
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
  cluster_id: null,
  is_builtin: true,
}

export const mockStageExecution: StageExecution = {
  id: 'exec-001',
  run_id: 'run-001',
  stage_number: 1,
  stage_name: 'Planning',
  agent_id: 'agent-001',
  status: 'completed',
  rendered_prompt: 'Plan the task',
  output: 'Here is the plan',
  structured_output: null,
  user_input: null,
  input_tokens: 500,
  output_tokens: 200,
  started_at: '2025-01-01T00:00:00Z',
  completed_at: '2025-01-01T00:00:01Z',
  duration_ms: 1000,
}

const mockModelConfig = { provider: 'anthropic', model_id: 'claude-sonnet-4-20250514', max_tokens: 8192, temperature: 0.7 }

export const mockConfig: Config = {
  verbosity: 'normal',
  models: {
    orchestrator: { ...mockModelConfig, model_id: 'claude-opus-4-5-20251101', max_tokens: 16384 },
    worker: mockModelConfig,
    utility: { ...mockModelConfig, model_id: 'claude-3-5-haiku-20241022', max_tokens: 4096 },
  },
  pool: { max_orchestrators: 1, max_workers: 3, max_utilities: 2 },
  autonomy: 'supervised',
  git_strategy: 'branch',
  sandbox_mode: 'docker',
}
