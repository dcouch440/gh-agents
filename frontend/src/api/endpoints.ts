import {api} from "./client";
import {API} from "@/constants";
import type {RequestConfig} from "./client";
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
} from "@/types";

// ============================================================================
// Response Types (for endpoints that return lists)
// ============================================================================

type ListResponse<T> = {
  items: T[];
  total?: number;
};

type TasksResponse = ListResponse<Task>;
type ToolsResponse = ListResponse<Tool>;
type DocumentsResponse = ListResponse<DocumentListItem>;
type SessionsResponse = ListResponse<Session>;
type ChatResponse = {message_id: string; response: string};
type SessionHistoryResponse = {messages: ChatMessage[]};
type PipelinesResponse = ListResponse<Pipeline>;
type PipelineRunsResponse = ListResponse<PipelineRun>;
type PipelineRunTreeResponse = unknown;
type ExecutionMessagesResponse = {messages: ExecutionMessage[]};
type OutputSchemasResponse = ListResponse<OutputSchema>;
type PromptTemplatesResponse = ListResponse<PromptTemplate>;
type CostsResponse = CostResponse;
type ResultsResponse = ListResponse<Result>;
type WorkflowsResponse = ListResponse<Workflow>;

// ============================================================================
// Auth Endpoints
// ============================================================================

const auth = {
  login: (body: {username: string; password: string}, config?: RequestConfig) =>
    api.post<{token: string}>(API.AUTH_LOGIN, body, config),

  register: (
    body: {username: string; password: string},
    config?: RequestConfig,
  ) => api.post<{token: string}>(API.AUTH_REGISTER, body, config),

  me: (config?: RequestConfig) =>
    api.get<{id: string; username: string}>(API.AUTH_ME, config),
};

// ============================================================================
// Agent Endpoints
// ============================================================================

const agents = {
  list: (config?: RequestConfig) => api.get<AgentsResponse>(API.AGENTS, config),

  get: (id: string, config?: RequestConfig) =>
    api.get<Agent>(API.AGENT(id), config),

  create: (body: CreateAgentRequest, config?: RequestConfig) =>
    api.post<Agent>(API.AGENTS, body, config),

  update: (id: string, body: UpdateAgentRequest, config?: RequestConfig) =>
    api.patch<Agent>(API.AGENT(id), body, config),

  delete: (id: string, config?: RequestConfig) =>
    api.del<void>(API.AGENT(id), config),

  getTools: (id: string, config?: RequestConfig) =>
    api.get<AgentToolsResponse>(API.AGENT_TOOLS(id), config),

  setTools: (id: string, toolIds: string[], config?: RequestConfig) =>
    api.put<void>(API.AGENT_TOOLS(id), toolIds, config),

  getContext: (id: string, config?: RequestConfig) =>
    api.get<AgentContextResponse>(API.AGENT_CONTEXT(id), config),

  setContext: (id: string, docIds: string[], config?: RequestConfig) =>
    api.put<void>(API.AGENT_CONTEXT(id), docIds, config),
};

// ============================================================================
// Task Endpoints
// ============================================================================

const tasks = {
  list: (config?: RequestConfig) => api.get<TasksResponse>(API.TASKS, config),

  get: (id: string, config?: RequestConfig) =>
    api.get<Task>(API.TASK(id), config),

  create: (body: CreateTaskRequest, config?: RequestConfig) =>
    api.post<Task>(API.TASKS, body, config),

  update: (id: string, body: Partial<Task>, config?: RequestConfig) =>
    api.patch<Task>(API.TASK(id), body, config),

  delete: (id: string, config?: RequestConfig) =>
    api.del<void>(API.TASK(id), config),
};

// ============================================================================
// Tool Endpoints
// ============================================================================

const tools = {
  list: (config?: RequestConfig) => api.get<ToolsResponse>(API.TOOLS, config),

  get: (id: string, config?: RequestConfig) =>
    api.get<Tool>(API.TOOL(id), config),

  create: (body: CreateToolRequest, config?: RequestConfig) =>
    api.post<Tool>(API.TOOLS, body, config),

  update: (id: string, body: UpdateToolRequest, config?: RequestConfig) =>
    api.patch<Tool>(API.TOOL(id), body, config),

  delete: (id: string, config?: RequestConfig) =>
    api.del<void>(API.TOOL(id), config),
};

// ============================================================================
// Document Endpoints
// ============================================================================

const documents = {
  list: (config?: RequestConfig) =>
    api.get<DocumentsResponse>(API.DOCUMENTS, config),

  get: (id: string, config?: RequestConfig) =>
    api.get<Document>(API.DOCUMENT(id), config),

  create: (body: CreateDocumentRequest, config?: RequestConfig) =>
    api.post<Document>(API.DOCUMENTS, body, config),

  update: (id: string, body: UpdateDocumentRequest, config?: RequestConfig) =>
    api.patch<Document>(API.DOCUMENT(id), body, config),

  delete: (id: string, config?: RequestConfig) =>
    api.del<void>(API.DOCUMENT(id), config),

  search: (query: string, config?: RequestConfig) =>
    api.get<DocumentsResponse>(API.DOCUMENTS_SEARCH(query), config),
};

// ============================================================================
// Session Endpoints
// ============================================================================

const sessions = {
  list: (config?: RequestConfig) =>
    api.get<SessionsResponse>(API.SESSIONS, config),

  get: (id: string, config?: RequestConfig) =>
    api.get<Session>(API.SESSION(id), config),

  create: (body: CreateSessionRequest, config?: RequestConfig) =>
    api.post<Session>(API.SESSIONS, body, config),

  update: (id: string, body: UpdateSessionRequest, config?: RequestConfig) =>
    api.patch<Session>(API.SESSION(id), body, config),

  delete: (id: string, config?: RequestConfig) =>
    api.del<void>(API.SESSION(id), config),

  chat: (id: string, message: SendMessageRequest, config?: RequestConfig) =>
    api.post<ChatResponse>(API.SESSION_CHAT(id), message, config),

  getHistory: (id: string, config?: RequestConfig) =>
    api.get<SessionHistoryResponse>(API.SESSION_HISTORY(id), config),
};

// ============================================================================
// Chat Endpoints
// ============================================================================

const chat = {
  send: (message: SendMessageRequest, config?: RequestConfig) =>
    api.post<ChatResponse>(API.CHAT, message, config),

  getHistory: (config?: RequestConfig) =>
    api.get<SessionHistoryResponse>(API.CHAT_HISTORY, config),
};

// ============================================================================
// Config Endpoints
// ============================================================================

const config = {
  get: (config?: RequestConfig) => api.get<Config>(API.CONFIG, config),

  update: (body: UpdateConfigRequest, config?: RequestConfig) =>
    api.patch<Config>(API.CONFIG, body, config),
};

// ============================================================================
// Stats Endpoints
// ============================================================================

const stats = {
  get: (config?: RequestConfig) => api.get<UsageSummary>(API.STATS, config),
};

// ============================================================================
// Pipeline Endpoints
// ============================================================================

const pipelines = {
  list: (config?: RequestConfig) =>
    api.get<PipelinesResponse>(API.PIPELINES, config),

  get: (id: string, config?: RequestConfig) =>
    api.get<Pipeline>(API.PIPELINE(id), config),

  create: (body: Partial<Pipeline>, config?: RequestConfig) =>
    api.post<Pipeline>(API.PIPELINES, body, config),

  update: (id: string, body: Partial<Pipeline>, config?: RequestConfig) =>
    api.patch<Pipeline>(API.PIPELINE(id), body, config),

  delete: (id: string, config?: RequestConfig) =>
    api.del<void>(API.PIPELINE(id), config),

  renderStage: (id: string, stage: number, config?: RequestConfig) =>
    api.get<unknown>(API.PIPELINE_STAGE_RENDER(id, stage), config),

  getSideTasks: (id: string, stage: number, config?: RequestConfig) =>
    api.get<Task[]>(API.PIPELINE_SIDE_TASKS(id, stage), config),

  getSideTask: (
    id: string,
    stage: number,
    taskId: string,
    config?: RequestConfig,
  ) => api.get<Task>(API.PIPELINE_SIDE_TASK(id, stage, taskId), config),
};

// ============================================================================
// Pipeline Run Endpoints
// ============================================================================

const pipelineRuns = {
  list: (config?: RequestConfig) =>
    api.get<PipelineRunsResponse>(API.PIPELINE_RUNS, config),

  get: (id: string, config?: RequestConfig) =>
    api.get<PipelineRun>(API.PIPELINE_RUN(id), config),

  approve: (id: string, config?: RequestConfig) =>
    api.post<void>(API.PIPELINE_RUN_APPROVE(id), undefined, config),

  getTree: (runId: string, config?: RequestConfig) =>
    api.get<PipelineRunTreeResponse>(API.PIPELINE_RUN_TREE(runId), config),
};

// ============================================================================
// Pipeline Stage Member Endpoints
// ============================================================================

const stageMembers = {
  list: (pipelineId: string, stageNum: number, config?: RequestConfig) =>
    api.get<StageMember[]>(API.STAGE_MEMBERS(pipelineId, stageNum), config),

  create: (
    pipelineId: string,
    stageNum: number,
    body: CreateStageMemberRequest,
    config?: RequestConfig,
  ) =>
    api.post<StageMember>(
      API.STAGE_MEMBERS(pipelineId, stageNum),
      body,
      config,
    ),

  delete: (
    pipelineId: string,
    stageNum: number,
    memberId: string,
    config?: RequestConfig,
  ) => api.del<void>(API.STAGE_MEMBER(pipelineId, stageNum, memberId), config),
};

// ============================================================================
// Agent Execution Endpoints
// ============================================================================

const agentExecutions = {
  get: (id: string, config?: RequestConfig) =>
    api.get<AgentExecution>(API.AGENT_EXECUTION(id), config),

  getMessages: (id: string, config?: RequestConfig) =>
    api.get<ExecutionMessagesResponse>(API.EXECUTION_MESSAGES(id), config),

  approve: (id: string, config?: RequestConfig) =>
    api.post<void>(API.EXECUTION_APPROVE(id), undefined, config),
};

// ============================================================================
// Output Schema Endpoints
// ============================================================================

const outputSchemas = {
  list: (config?: RequestConfig) =>
    api.get<OutputSchemasResponse>(API.OUTPUT_SCHEMAS, config),

  get: (id: string, config?: RequestConfig) =>
    api.get<OutputSchema>(API.OUTPUT_SCHEMA(id), config),

  create: (body: CreateOutputSchemaRequest, config?: RequestConfig) =>
    api.post<OutputSchema>(API.OUTPUT_SCHEMAS, body, config),

  update: (
    id: string,
    body: UpdateOutputSchemaRequest,
    config?: RequestConfig,
  ) => api.patch<OutputSchema>(API.OUTPUT_SCHEMA(id), body, config),

  delete: (id: string, config?: RequestConfig) =>
    api.del<void>(API.OUTPUT_SCHEMA(id), config),
};

// ============================================================================
// Prompt Template Endpoints
// ============================================================================

const promptTemplates = {
  list: (config?: RequestConfig) =>
    api.get<PromptTemplatesResponse>(API.PROMPT_TEMPLATES, config),

  get: (id: string, config?: RequestConfig) =>
    api.get<PromptTemplate>(API.PROMPT_TEMPLATE(id), config),

  create: (body: CreatePromptTemplateRequest, config?: RequestConfig) =>
    api.post<PromptTemplate>(API.PROMPT_TEMPLATES, body, config),

  update: (
    id: string,
    body: UpdatePromptTemplateRequest,
    config?: RequestConfig,
  ) => api.patch<PromptTemplate>(API.PROMPT_TEMPLATE(id), body, config),

  delete: (id: string, config?: RequestConfig) =>
    api.del<void>(API.PROMPT_TEMPLATE(id), config),
};

// ============================================================================
// Cost Endpoints
// ============================================================================

const costs = {
  list: (config?: RequestConfig) => api.get<CostsResponse>(API.COSTS, config),
};

// ============================================================================
// Result Endpoints
// ============================================================================

const results = {
  list: (config?: RequestConfig) =>
    api.get<ResultsResponse>(API.RESULTS, config),

  get: (id: string, config?: RequestConfig) =>
    api.get<Result>(API.RESULT(id), config),
};

// ============================================================================
// Workflow Endpoints
// ============================================================================

const workflows = {
  list: (config?: RequestConfig) =>
    api.get<WorkflowsResponse>(API.WORKFLOWS, config),

  get: (id: string, config?: RequestConfig) =>
    api.get<Workflow>(API.WORKFLOW(id), config),

  create: (body: CreateWorkflowRequest, config?: RequestConfig) =>
    api.post<Workflow>(API.WORKFLOWS, body, config),

  update: (id: string, body: UpdateWorkflowRequest, config?: RequestConfig) =>
    api.patch<Workflow>(API.WORKFLOW(id), body, config),

  delete: (id: string, config?: RequestConfig) =>
    api.del<void>(API.WORKFLOW(id), config),

  // Steps
  listSteps: (workflowId: string, config?: RequestConfig) =>
    api.get<WorkflowStep[]>(API.WORKFLOW_STEPS(workflowId), config),

  createStep: (
    workflowId: string,
    body: CreateStepRequest,
    config?: RequestConfig,
  ) => api.post<WorkflowStep>(API.WORKFLOW_STEPS(workflowId), body, config),

  getStep: (workflowId: string, stepId: string, config?: RequestConfig) =>
    api.get<WorkflowStep>(API.WORKFLOW_STEP(workflowId, stepId), config),

  updateStep: (
    workflowId: string,
    stepId: string,
    body: UpdateStepRequest,
    config?: RequestConfig,
  ) =>
    api.patch<WorkflowStep>(
      API.WORKFLOW_STEP(workflowId, stepId),
      body,
      config,
    ),

  deleteStep: (workflowId: string, stepId: string, config?: RequestConfig) =>
    api.del<void>(API.WORKFLOW_STEP(workflowId, stepId), config),

  // Edges
  listEdges: (workflowId: string, config?: RequestConfig) =>
    api.get<WorkflowStepEdge[]>(API.WORKFLOW_EDGES(workflowId), config),

  createEdge: (workflowId: string, body: EdgeRequest, config?: RequestConfig) =>
    api.post<WorkflowStepEdge>(API.WORKFLOW_EDGES(workflowId), body, config),

  // Step Documents
  listStepDocuments: (
    workflowId: string,
    stepId: string,
    config?: RequestConfig,
  ) => api.get<Document[]>(API.STEP_DOCUMENTS(workflowId, stepId), config),

  addStepDocument: (
    workflowId: string,
    stepId: string,
    docId: string,
    config?: RequestConfig,
  ) =>
    api.post<void>(
      API.STEP_DOCUMENT(workflowId, stepId, docId),
      undefined,
      config,
    ),

  removeStepDocument: (
    workflowId: string,
    stepId: string,
    docId: string,
    config?: RequestConfig,
  ) => api.del<void>(API.STEP_DOCUMENT(workflowId, stepId, docId), config),
};

// ============================================================================
// Context Response Endpoint
// ============================================================================

const contextResponse = {
  get: (config?: RequestConfig) =>
    api.get<unknown>(API.CONTEXT_RESPONSE, config),
};

// ============================================================================
// Modes Endpoint
// ============================================================================

const modes = {
  list: (config?: RequestConfig) => api.get<unknown>(API.MODES, config),
};

// ============================================================================
// Export All Endpoints
// ============================================================================

export const endpoints = {
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
};
