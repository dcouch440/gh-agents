import { api as baseApi } from './client'
import { API } from '@/constants'
import type { RequestConfig } from './client'
import type {
  Agent,
  AgentsResponse,
  CreateAgentRequest,
  UpdateAgentRequest,
  AgentToolsResponse,
  AgentContextResponse,
  Task,
  CreateTaskRequest,
  Tool,
  CreateToolRequest,
  UpdateToolRequest,
  Document,
  DocumentListItem,
  CreateDocumentRequest,
  UpdateDocumentRequest,
  Session,
  CreateSessionRequest,
  UpdateSessionRequest,
  SendMessageRequest,
  ChatMessage,
  Config,
  UpdateConfigRequest,
  UsageSummary,
  Pipeline,
  PipelineRun,
  StageMember,
  CreateStageMemberRequest,
  AgentExecution,
  ExecutionMessage,
  OutputSchema,
  CreateOutputSchemaRequest,
  UpdateOutputSchemaRequest,
  PromptTemplate,
  CreatePromptTemplateRequest,
  UpdatePromptTemplateRequest,
  CostResponse,
  Result,
  Workflow,
  CreateWorkflowRequest,
  UpdateWorkflowRequest,
  WorkflowStep,
  CreateStepRequest,
  UpdateStepRequest,
  WorkflowStepEdge,
  EdgeRequest,
} from '@/types'

// ============================================================================
// Response Types (for endpoints that return lists)
// ============================================================================

type ListResponse<T> = {
  items: T[]
  total?: number
}

type TasksResponse = ListResponse<Task>
type ToolsResponse = ListResponse<Tool>
type DocumentsResponse = ListResponse<DocumentListItem>
type SessionsResponse = ListResponse<Session>
type ChatResponse = { message_id: string; response: string }
type SessionHistoryResponse = { messages: ChatMessage[] }
type PipelinesResponse = ListResponse<Pipeline>
type PipelineRunsResponse = ListResponse<PipelineRun>
type PipelineRunTreeResponse = unknown
type ExecutionMessagesResponse = { messages: ExecutionMessage[] }
type OutputSchemasResponse = ListResponse<OutputSchema>
type PromptTemplatesResponse = ListResponse<PromptTemplate>
type CostsResponse = CostResponse
type ResultsResponse = ListResponse<Result>
type WorkflowsResponse = ListResponse<Workflow>

// ============================================================================
// Typed Endpoints
// ============================================================================

const auth = {
  login: (body: { username: string; password: string }, config?: RequestConfig) =>
    baseApi.post<{ token: string }>(API.AUTH_LOGIN, body, config),

  register: (body: { username: string; password: string }, config?: RequestConfig) =>
    baseApi.post<{ token: string }>(API.AUTH_REGISTER, body, config),

  me: (config?: RequestConfig) => baseApi.get<{ id: string; username: string }>(API.AUTH_ME, config),
}

const agents = {
  list: (config?: RequestConfig) => baseApi.get<AgentsResponse>(API.AGENTS, config),

  get: (id: string, config?: RequestConfig) => baseApi.get<Agent>(API.AGENT(id), config),

  create: (body: CreateAgentRequest, config?: RequestConfig) =>
    baseApi.post<Agent>(API.AGENTS, body, config),

  update: (id: string, body: UpdateAgentRequest, config?: RequestConfig) =>
    baseApi.patch<Agent>(API.AGENT(id), body, config),

  delete: (id: string, config?: RequestConfig) => baseApi.del<void>(API.AGENT(id), config),

  getTools: (id: string, config?: RequestConfig) =>
    baseApi.get<AgentToolsResponse>(API.AGENT_TOOLS(id), config),

  setTools: (id: string, toolIds: string[], config?: RequestConfig) =>
    baseApi.put<void>(API.AGENT_TOOLS(id), toolIds, config),

  getContext: (id: string, config?: RequestConfig) =>
    baseApi.get<AgentContextResponse>(API.AGENT_CONTEXT(id), config),

  setContext: (id: string, docIds: string[], config?: RequestConfig) =>
    baseApi.put<void>(API.AGENT_CONTEXT(id), docIds, config),
}

const tasks = {
  list: (config?: RequestConfig) => baseApi.get<TasksResponse>(API.TASKS, config),

  get: (id: string, config?: RequestConfig) => baseApi.get<Task>(API.TASK(id), config),

  create: (body: CreateTaskRequest, config?: RequestConfig) =>
    baseApi.post<Task>(API.TASKS, body, config),

  update: (id: string, body: Partial<Task>, config?: RequestConfig) =>
    baseApi.patch<Task>(API.TASK(id), body, config),

  delete: (id: string, config?: RequestConfig) => baseApi.del<void>(API.TASK(id), config),
}

const tools = {
  list: (config?: RequestConfig) => baseApi.get<ToolsResponse>(API.TOOLS, config),

  get: (id: string, config?: RequestConfig) => baseApi.get<Tool>(API.TOOL(id), config),

  create: (body: CreateToolRequest, config?: RequestConfig) =>
    baseApi.post<Tool>(API.TOOLS, body, config),

  update: (id: string, body: UpdateToolRequest, config?: RequestConfig) =>
    baseApi.patch<Tool>(API.TOOL(id), body, config),

  delete: (id: string, config?: RequestConfig) => baseApi.del<void>(API.TOOL(id), config),
}

const documents = {
  list: (config?: RequestConfig) => baseApi.get<DocumentsResponse>(API.DOCUMENTS, config),

  get: (id: string, config?: RequestConfig) => baseApi.get<Document>(API.DOCUMENT(id), config),

  create: (body: CreateDocumentRequest, config?: RequestConfig) =>
    baseApi.post<Document>(API.DOCUMENTS, body, config),

  update: (id: string, body: UpdateDocumentRequest, config?: RequestConfig) =>
    baseApi.patch<Document>(API.DOCUMENT(id), body, config),

  delete: (id: string, config?: RequestConfig) => baseApi.del<void>(API.DOCUMENT(id), config),

  search: (query: string, config?: RequestConfig) =>
    baseApi.get<DocumentsResponse>(API.DOCUMENTS_SEARCH(query), config),
}

const sessions = {
  list: (config?: RequestConfig) => baseApi.get<SessionsResponse>(API.SESSIONS, config),

  get: (id: string, config?: RequestConfig) => baseApi.get<Session>(API.SESSION(id), config),

  create: (body: CreateSessionRequest, config?: RequestConfig) =>
    baseApi.post<Session>(API.SESSIONS, body, config),

  update: (id: string, body: UpdateSessionRequest, config?: RequestConfig) =>
    baseApi.patch<Session>(API.SESSION(id), body, config),

  delete: (id: string, config?: RequestConfig) => baseApi.del<void>(API.SESSION(id), config),

  chat: (id: string, message: SendMessageRequest, config?: RequestConfig) =>
    baseApi.post<ChatResponse>(API.SESSION_CHAT(id), message, config),

  getHistory: (id: string, config?: RequestConfig) =>
    baseApi.get<SessionHistoryResponse>(API.SESSION_HISTORY(id), config),
}

const chat = {
  send: (message: SendMessageRequest, config?: RequestConfig) =>
    baseApi.post<ChatResponse>(API.CHAT, message, config),

  getHistory: (config?: RequestConfig) =>
    baseApi.get<SessionHistoryResponse>(API.CHAT_HISTORY, config),
}

const config = {
  get: (config?: RequestConfig) => baseApi.get<Config>(API.CONFIG, config),

  update: (body: UpdateConfigRequest, config?: RequestConfig) =>
    baseApi.patch<Config>(API.CONFIG, body, config),
}

const stats = {
  get: (config?: RequestConfig) => baseApi.get<UsageSummary>(API.STATS, config),
}

const pipelines = {
  list: (config?: RequestConfig) => baseApi.get<PipelinesResponse>(API.PIPELINES, config),

  get: (id: string, config?: RequestConfig) => baseApi.get<Pipeline>(API.PIPELINE(id), config),

  create: (body: Partial<Pipeline>, config?: RequestConfig) =>
    baseApi.post<Pipeline>(API.PIPELINES, body, config),

  update: (id: string, body: Partial<Pipeline>, config?: RequestConfig) =>
    baseApi.patch<Pipeline>(API.PIPELINE(id), body, config),

  delete: (id: string, config?: RequestConfig) => baseApi.del<void>(API.PIPELINE(id), config),

  renderStage: (id: string, stage: number, config?: RequestConfig) =>
    baseApi.get<unknown>(API.PIPELINE_STAGE_RENDER(id, stage), config),

  getSideTasks: (id: string, stage: number, config?: RequestConfig) =>
    baseApi.get<Task[]>(API.PIPELINE_SIDE_TASKS(id, stage), config),

  getSideTask: (id: string, stage: number, taskId: string, config?: RequestConfig) =>
    baseApi.get<Task>(API.PIPELINE_SIDE_TASK(id, stage, taskId), config),
}

const pipelineRuns = {
  list: (config?: RequestConfig) => baseApi.get<PipelineRunsResponse>(API.PIPELINE_RUNS, config),

  get: (id: string, config?: RequestConfig) =>
    baseApi.get<PipelineRun>(API.PIPELINE_RUN(id), config),

  approve: (id: string, config?: RequestConfig) =>
    baseApi.post<void>(API.PIPELINE_RUN_APPROVE(id), undefined, config),

  getTree: (runId: string, config?: RequestConfig) =>
    baseApi.get<PipelineRunTreeResponse>(API.PIPELINE_RUN_TREE(runId), config),
}

const stageMembers = {
  list: (pipelineId: string, stageNum: number, config?: RequestConfig) =>
    baseApi.get<StageMember[]>(API.STAGE_MEMBERS(pipelineId, stageNum), config),

  create: (
    pipelineId: string,
    stageNum: number,
    body: CreateStageMemberRequest,
    config?: RequestConfig
  ) => baseApi.post<StageMember>(API.STAGE_MEMBERS(pipelineId, stageNum), body, config),

  delete: (pipelineId: string, stageNum: number, memberId: string, config?: RequestConfig) =>
    baseApi.del<void>(API.STAGE_MEMBER(pipelineId, stageNum, memberId), config),
}

const agentExecutions = {
  get: (id: string, config?: RequestConfig) =>
    baseApi.get<AgentExecution>(API.AGENT_EXECUTION(id), config),

  getMessages: (id: string, config?: RequestConfig) =>
    baseApi.get<ExecutionMessagesResponse>(API.EXECUTION_MESSAGES(id), config),

  approve: (id: string, config?: RequestConfig) =>
    baseApi.post<void>(API.EXECUTION_APPROVE(id), undefined, config),
}

const outputSchemas = {
  list: (config?: RequestConfig) => baseApi.get<OutputSchemasResponse>(API.OUTPUT_SCHEMAS, config),

  get: (id: string, config?: RequestConfig) =>
    baseApi.get<OutputSchema>(API.OUTPUT_SCHEMA(id), config),

  create: (body: CreateOutputSchemaRequest, config?: RequestConfig) =>
    baseApi.post<OutputSchema>(API.OUTPUT_SCHEMAS, body, config),

  update: (id: string, body: UpdateOutputSchemaRequest, config?: RequestConfig) =>
    baseApi.patch<OutputSchema>(API.OUTPUT_SCHEMA(id), body, config),

  delete: (id: string, config?: RequestConfig) =>
    baseApi.del<void>(API.OUTPUT_SCHEMA(id), config),
}

const promptTemplates = {
  list: (config?: RequestConfig) =>
    baseApi.get<PromptTemplatesResponse>(API.PROMPT_TEMPLATES, config),

  get: (id: string, config?: RequestConfig) =>
    baseApi.get<PromptTemplate>(API.PROMPT_TEMPLATE(id), config),

  create: (body: CreatePromptTemplateRequest, config?: RequestConfig) =>
    baseApi.post<PromptTemplate>(API.PROMPT_TEMPLATES, body, config),

  update: (id: string, body: UpdatePromptTemplateRequest, config?: RequestConfig) =>
    baseApi.patch<PromptTemplate>(API.PROMPT_TEMPLATE(id), body, config),

  delete: (id: string, config?: RequestConfig) =>
    baseApi.del<void>(API.PROMPT_TEMPLATE(id), config),
}

const costs = {
  list: (config?: RequestConfig) => baseApi.get<CostsResponse>(API.COSTS, config),
}

const results = {
  list: (config?: RequestConfig) => baseApi.get<ResultsResponse>(API.RESULTS, config),

  get: (id: string, config?: RequestConfig) => baseApi.get<Result>(API.RESULT(id), config),
}

const workflows = {
  list: (config?: RequestConfig) => baseApi.get<WorkflowsResponse>(API.WORKFLOWS, config),

  get: (id: string, config?: RequestConfig) => baseApi.get<Workflow>(API.WORKFLOW(id), config),

  create: (body: CreateWorkflowRequest, config?: RequestConfig) =>
    baseApi.post<Workflow>(API.WORKFLOWS, body, config),

  update: (id: string, body: UpdateWorkflowRequest, config?: RequestConfig) =>
    baseApi.patch<Workflow>(API.WORKFLOW(id), body, config),

  delete: (id: string, config?: RequestConfig) => baseApi.del<void>(API.WORKFLOW(id), config),

  listSteps: (workflowId: string, config?: RequestConfig) =>
    baseApi.get<WorkflowStep[]>(API.WORKFLOW_STEPS(workflowId), config),

  createStep: (workflowId: string, body: CreateStepRequest, config?: RequestConfig) =>
    baseApi.post<WorkflowStep>(API.WORKFLOW_STEPS(workflowId), body, config),

  getStep: (workflowId: string, stepId: string, config?: RequestConfig) =>
    baseApi.get<WorkflowStep>(API.WORKFLOW_STEP(workflowId, stepId), config),

  updateStep: (
    workflowId: string,
    stepId: string,
    body: UpdateStepRequest,
    config?: RequestConfig
  ) => baseApi.patch<WorkflowStep>(API.WORKFLOW_STEP(workflowId, stepId), body, config),

  deleteStep: (workflowId: string, stepId: string, config?: RequestConfig) =>
    baseApi.del<void>(API.WORKFLOW_STEP(workflowId, stepId), config),

  listEdges: (workflowId: string, config?: RequestConfig) =>
    baseApi.get<WorkflowStepEdge[]>(API.WORKFLOW_EDGES(workflowId), config),

  createEdge: (workflowId: string, body: EdgeRequest, config?: RequestConfig) =>
    baseApi.post<WorkflowStepEdge>(API.WORKFLOW_EDGES(workflowId), body, config),

  listStepDocuments: (workflowId: string, stepId: string, config?: RequestConfig) =>
    baseApi.get<Document[]>(API.STEP_DOCUMENTS(workflowId, stepId), config),

  addStepDocument: (workflowId: string, stepId: string, docId: string, config?: RequestConfig) =>
    baseApi.post<void>(API.STEP_DOCUMENT(workflowId, stepId, docId), undefined, config),

  removeStepDocument: (workflowId: string, stepId: string, docId: string, config?: RequestConfig) =>
    baseApi.del<void>(API.STEP_DOCUMENT(workflowId, stepId, docId), config),
}

const contextResponse = {
  get: (config?: RequestConfig) => baseApi.get<unknown>(API.CONTEXT_RESPONSE, config),
}

const modes = {
  list: (config?: RequestConfig) => baseApi.get<unknown>(API.MODES, config),
}

// ============================================================================
// Merge base API methods with typed endpoints into single `api` export
// ============================================================================

export const api = {
  // Low-level HTTP methods
  get: baseApi.get,
  post: baseApi.post,
  patch: baseApi.patch,
  put: baseApi.put,
  del: baseApi.del,

  // Typed endpoints
  auth,
  agents,
  tasks,
  tools,
  documents,
  sessions,
  chat,
  config,
  stats,
  pipelines,
  pipelineRuns,
  stageMembers,
  agentExecutions,
  outputSchemas,
  promptTemplates,
  costs,
  results,
  workflows,
  contextResponse,
  modes,
}
