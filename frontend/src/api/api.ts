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
  RouterMode,
  CreateRouterModeRequest,
  UpdateRouterModeRequest,
  SetModeToolsRequest,
  SendExecutionMessageRequest,
  ApproveExecutionRequest,
  SendMessageResponse,
  ToolRouter,
  CreateToolRouterRequest,
  UpdateToolRouterRequest,
  SetRouterToolsRequest,
  Room,
  RoomMember,
  RoomSession,
  RoomTranscriptEntry,
  RoomOutput,
  CreateRoomRequest,
  UpdateRoomRequest,
  AddRoomMemberRequest,
  SetRoomMembersRequest,
  RoomMessageRequest,
  Collection,
  CollectionRun,
  CreateCollectionRequest,
  UpdateCollectionRequest,
  WorkflowRunResponse,
  WorkflowExecutionSummary,
  Protocol,
  ProtocolPort,
  ProtocolTypeInfo,
  CreateProtocolRequest,
  UpdateProtocolRequest,
  CreatePortRequest,
  DocumentDef,
  CreateDocumentDefRequest,
  UpdateDocumentDefRequest,
  RosterAgent,
  CreateRosterAgentRequest,
  RoomStepMember,
  StepChatDebugResponse,
  StepLastRunResponse,
} from '@/types'

// ============================================================================
// Response Types
// ============================================================================

type LoginResponse = { token: string; expires_in: number }

type RegisterResponse = {
  token: string
  expires_in: number
  user: { id: string; email: string; github_login: string | null }
}

type MeResponse = {
  id: string
  email: string
  github_login: string | null
  authenticated: boolean
  token_expires: number
}

type ChatResponse = { message_id: string; response: string }
type ExecutionMessagesResponse = { messages: ExecutionMessage[] }
type CostsResponse = CostResponse
type ProtocolTypesResponse = { types: ProtocolTypeInfo[] }

// ============================================================================
// Freeze Helper
// ============================================================================

const freeze = <T extends Record<string, unknown>>(obj: T): Readonly<T> => Object.freeze(obj)

// ============================================================================
// Typed Endpoints
// ============================================================================

const auth = freeze({
  login: (body: { email: string; password: string }, config?: RequestConfig) =>
    baseApi.post<LoginResponse>(API.AUTH_LOGIN, body, config),

  register: (body: { email: string; password: string }, config?: RequestConfig) =>
    baseApi.post<RegisterResponse>(API.AUTH_REGISTER, body, config),

  me: (config?: RequestConfig) =>
    baseApi.get<MeResponse>(API.AUTH_ME, config),
})

const agents = freeze({
  list: (config?: RequestConfig) => baseApi.get<AgentsResponse>(API.AGENTS, config),

  get: (id: string, config?: RequestConfig) => baseApi.get<Agent>(API.AGENT(id), config),

  create: (body: CreateAgentRequest, config?: RequestConfig) => baseApi.post<Agent>(API.AGENTS, body, config),

  update: (id: string, body: UpdateAgentRequest, config?: RequestConfig) => baseApi.patch<Agent>(API.AGENT(id), body, config),

  delete: (id: string, config?: RequestConfig) => baseApi.del<void>(API.AGENT(id), config),

  getTools: (id: string, config?: RequestConfig) => baseApi.get<AgentToolsResponse>(API.AGENT_TOOLS(id), config),

  setTools: (id: string, toolIds: string[], config?: RequestConfig) => baseApi.put<void>(API.AGENT_TOOLS(id), toolIds, config),

  getContext: (id: string, config?: RequestConfig) => baseApi.get<AgentContextResponse>(API.AGENT_CONTEXT(id), config),

  setContext: (id: string, docIds: string[], config?: RequestConfig) =>
    baseApi.put<void>(API.AGENT_CONTEXT(id), { document_ids: docIds }, config),
})

const tasks = freeze({
  list: (config?: RequestConfig) => baseApi.get<Task[]>(API.TASKS, config),

  get: (id: string, config?: RequestConfig) => baseApi.get<Task>(API.TASK(id), config),

  create: (body: CreateTaskRequest, config?: RequestConfig) => baseApi.post<Task>(API.TASKS, body, config),

  update: (id: string, body: Partial<Task>, config?: RequestConfig) => baseApi.patch<Task>(API.TASK(id), body, config),

  delete: (id: string, config?: RequestConfig) => baseApi.del<void>(API.TASK(id), config),
})

const tools = freeze({
  list: (config?: RequestConfig) => baseApi.get<Tool[]>(API.TOOLS, config),

  get: (id: string, config?: RequestConfig) => baseApi.get<Tool>(API.TOOL(id), config),

  create: (body: CreateToolRequest, config?: RequestConfig) => baseApi.post<Tool>(API.TOOLS, body, config),

  update: (id: string, body: UpdateToolRequest, config?: RequestConfig) => baseApi.patch<Tool>(API.TOOL(id), body, config),

  delete: (id: string, config?: RequestConfig) => baseApi.del<void>(API.TOOL(id), config),
})

const documents = freeze({
  list: (config?: RequestConfig) => baseApi.get<DocumentListItem[]>(API.DOCUMENTS, config),

  get: (id: string, config?: RequestConfig) => baseApi.get<Document>(API.DOCUMENT(id), config),

  create: (body: CreateDocumentRequest, config?: RequestConfig) => baseApi.post<Document>(API.DOCUMENTS, body, config),

  update: (id: string, body: UpdateDocumentRequest, config?: RequestConfig) => baseApi.patch<Document>(API.DOCUMENT(id), body, config),

  delete: (id: string, config?: RequestConfig) => baseApi.del<void>(API.DOCUMENT(id), config),

  search: (query: string, config?: RequestConfig) => baseApi.get<DocumentListItem[]>(API.DOCUMENTS_SEARCH(query), config),
})

const sessions = freeze({
  list: (config?: RequestConfig) => baseApi.get<Session[]>(API.SESSIONS, config),

  get: (id: string, config?: RequestConfig) => baseApi.get<Session>(API.SESSION(id), config),

  create: (body: CreateSessionRequest, config?: RequestConfig) => baseApi.post<Session>(API.SESSIONS, body, config),

  update: (id: string, body: UpdateSessionRequest, config?: RequestConfig) => baseApi.patch<Session>(API.SESSION(id), body, config),

  delete: (id: string, config?: RequestConfig) => baseApi.del<void>(API.SESSION(id), config),

  chat: (id: string, message: SendMessageRequest, config?: RequestConfig) =>
    baseApi.post<ChatResponse>(API.SESSION_CHAT(id), message, config),

  getHistory: (id: string, config?: RequestConfig) => baseApi.get<ChatMessage[]>(API.SESSION_HISTORY(id), config),

  clearMessages: (id: string, config?: RequestConfig) => baseApi.del<void>(API.SESSION_MESSAGES(id), config),
})

const chat = freeze({
  send: (message: SendMessageRequest, config?: RequestConfig) => baseApi.post<ChatResponse>(API.CHAT, message, config),

  getHistory: (config?: RequestConfig) => baseApi.get<ChatMessage[]>(API.CHAT_HISTORY, config),
})

const appConfig = freeze({
  get: (config?: RequestConfig) => baseApi.get<Config>(API.CONFIG, config),

  update: (body: UpdateConfigRequest, config?: RequestConfig) => baseApi.patch<Config>(API.CONFIG, body, config),
})

const stats = freeze({
  get: (config?: RequestConfig) => baseApi.get<UsageSummary>(API.STATS, config),
})

const agentExecutions = freeze({
  list: (params?: { status?: string }, config?: RequestConfig) =>
    baseApi.get<AgentExecution[]>(params?.status ? `${API.AGENT_EXECUTIONS}?status=${params.status}` : API.AGENT_EXECUTIONS, config),

  get: (id: string, config?: RequestConfig) => baseApi.get<AgentExecution>(API.AGENT_EXECUTION(id), config),

  getMessages: (id: string, config?: RequestConfig) => baseApi.get<ExecutionMessagesResponse>(API.EXECUTION_MESSAGES(id), config),

  sendMessage: (id: string, body: SendExecutionMessageRequest, config?: RequestConfig) =>
    baseApi.post<SendMessageResponse>(API.EXECUTION_MESSAGES(id), body, config),

  approve: (id: string, body?: ApproveExecutionRequest, config?: RequestConfig) =>
    baseApi.post<void>(API.EXECUTION_APPROVE(id), body, config),
})

const outputSchemas = freeze({
  list: (config?: RequestConfig) => baseApi.get<OutputSchema[]>(API.OUTPUT_SCHEMAS, config),

  get: (id: string, config?: RequestConfig) => baseApi.get<OutputSchema>(API.OUTPUT_SCHEMA(id), config),

  create: (body: CreateOutputSchemaRequest, config?: RequestConfig) => baseApi.post<OutputSchema>(API.OUTPUT_SCHEMAS, body, config),

  update: (id: string, body: UpdateOutputSchemaRequest, config?: RequestConfig) =>
    baseApi.patch<OutputSchema>(API.OUTPUT_SCHEMA(id), body, config),

  delete: (id: string, config?: RequestConfig) => baseApi.del<void>(API.OUTPUT_SCHEMA(id), config),
})

const promptTemplates = freeze({
  list: (config?: RequestConfig) => baseApi.get<PromptTemplate[]>(API.PROMPT_TEMPLATES, config),

  get: (id: string, config?: RequestConfig) => baseApi.get<PromptTemplate>(API.PROMPT_TEMPLATE(id), config),

  create: (body: CreatePromptTemplateRequest, config?: RequestConfig) => baseApi.post<PromptTemplate>(API.PROMPT_TEMPLATES, body, config),

  update: (id: string, body: UpdatePromptTemplateRequest, config?: RequestConfig) =>
    baseApi.patch<PromptTemplate>(API.PROMPT_TEMPLATE(id), body, config),

  delete: (id: string, config?: RequestConfig) => baseApi.del<void>(API.PROMPT_TEMPLATE(id), config),
})

const costs = freeze({
  list: (config?: RequestConfig) => baseApi.get<CostsResponse>(API.COSTS, config),
})

const results = freeze({
  list: (config?: RequestConfig) => baseApi.get<Result[]>(API.RESULTS, config),

  get: (id: string, config?: RequestConfig) => baseApi.get<Result>(API.RESULT(id), config),
})

const workflows = freeze({
  list: (config?: RequestConfig) => baseApi.get<Workflow[]>(API.WORKFLOWS, config),

  get: (id: string, config?: RequestConfig) => baseApi.get<Workflow>(API.WORKFLOW(id), config),

  create: (body: CreateWorkflowRequest, config?: RequestConfig) => baseApi.post<Workflow>(API.WORKFLOWS, body, config),

  update: (id: string, body: UpdateWorkflowRequest, config?: RequestConfig) => baseApi.patch<Workflow>(API.WORKFLOW(id), body, config),

  delete: (id: string, config?: RequestConfig) => baseApi.del<void>(API.WORKFLOW(id), config),

  listSteps: (workflowId: string, config?: RequestConfig) => baseApi.get<WorkflowStep[]>(API.WORKFLOW_STEPS(workflowId), config),

  createStep: (workflowId: string, body: CreateStepRequest, config?: RequestConfig) =>
    baseApi.post<WorkflowStep>(API.WORKFLOW_STEPS(workflowId), body, config),

  getStep: (workflowId: string, stepId: string, config?: RequestConfig) =>
    baseApi.get<WorkflowStep>(API.WORKFLOW_STEP(workflowId, stepId), config),

  updateStep: (workflowId: string, stepId: string, body: UpdateStepRequest, config?: RequestConfig) =>
    baseApi.patch<WorkflowStep>(API.WORKFLOW_STEP(workflowId, stepId), body, config),

  deleteStep: (workflowId: string, stepId: string, config?: RequestConfig) =>
    baseApi.del<void>(API.WORKFLOW_STEP(workflowId, stepId), config),

  listEdges: (workflowId: string, config?: RequestConfig) => baseApi.get<WorkflowStepEdge[]>(API.WORKFLOW_EDGES(workflowId), config),

  createEdge: (workflowId: string, body: EdgeRequest, config?: RequestConfig) =>
    baseApi.post<WorkflowStepEdge>(API.WORKFLOW_EDGES(workflowId), body, config),

  deleteEdge: (workflowId: string, edgeId: string, config?: RequestConfig) =>
    baseApi.del<void>(API.WORKFLOW_EDGE(workflowId, edgeId), config),

  listStepDocuments: (workflowId: string, stepId: string, config?: RequestConfig) =>
    baseApi.get<Document[]>(API.STEP_DOCUMENTS(workflowId, stepId), config),

  addStepDocument: (workflowId: string, stepId: string, docId: string, config?: RequestConfig) =>
    baseApi.post<void>(API.STEP_DOCUMENT(workflowId, stepId, docId), undefined, config),

  removeStepDocument: (workflowId: string, stepId: string, docId: string, config?: RequestConfig) =>
    baseApi.del<void>(API.STEP_DOCUMENT(workflowId, stepId, docId), config),

  listDocumentDefs: (workflowId: string, stepId: string, config?: RequestConfig) =>
    baseApi.get<DocumentDef[]>(API.STEP_DOCUMENT_DEFS(workflowId, stepId), config),

  createDocumentDef: (workflowId: string, stepId: string, body: CreateDocumentDefRequest, config?: RequestConfig) =>
    baseApi.post<DocumentDef>(API.STEP_DOCUMENT_DEFS(workflowId, stepId), body, config),

  updateDocumentDef: (workflowId: string, stepId: string, defId: string, body: UpdateDocumentDefRequest, config?: RequestConfig) =>
    baseApi.patch<DocumentDef>(API.STEP_DOCUMENT_DEF(workflowId, stepId, defId), body, config),

  deleteDocumentDef: (workflowId: string, stepId: string, defId: string, config?: RequestConfig) =>
    baseApi.del<void>(API.STEP_DOCUMENT_DEF(workflowId, stepId, defId), config),

  listRosterAgents: (workflowId: string, stepId: string, config?: RequestConfig) =>
    baseApi.get<RosterAgent[]>(API.STEP_AGENT_ROSTER(workflowId, stepId), config),

  createRosterAgent: (workflowId: string, stepId: string, body: CreateRosterAgentRequest, config?: RequestConfig) =>
    baseApi.post<RosterAgent>(API.STEP_AGENT_ROSTER(workflowId, stepId), body, config),

  deleteRosterAgent: (workflowId: string, stepId: string, agentId: string, config?: RequestConfig) =>
    baseApi.del<void>(API.STEP_ROSTER_AGENT(workflowId, stepId, agentId), config),

  listRoomStepMembers: (workflowId: string, stepId: string, config?: RequestConfig) =>
    baseApi.get<RoomStepMember[]>(API.STEP_ROOM_MEMBERS(workflowId, stepId), config),

  getStepSession: (workflowId: string, stepId: string, config?: RequestConfig) =>
    baseApi.get<Session>(API.STEP_CHAT_SESSION(workflowId, stepId), config),

  getOrCreateStepSession: (workflowId: string, stepId: string, config?: RequestConfig) =>
    baseApi.post<Session>(API.STEP_CHAT_SESSION(workflowId, stepId), undefined, config),

  clearStepMessages: (workflowId: string, stepId: string, config?: RequestConfig) =>
    baseApi.del<void>(API.STEP_CHAT_MESSAGES(workflowId, stepId), config),

  getStepChatDebug: (workflowId: string, stepId: string, config?: RequestConfig) =>
    baseApi.get<StepChatDebugResponse>(API.STEP_CHAT_DEBUG(workflowId, stepId), config),

  getStepLastRun: (workflowId: string, stepId: string, config?: RequestConfig) =>
    baseApi.get<StepLastRunResponse>(API.STEP_LAST_RUN(workflowId, stepId), config),

  run: (id: string, body?: { initial_input?: string }, config?: RequestConfig) =>
    baseApi.post<WorkflowRunResponse>(API.WORKFLOW_RUN(id), body ?? {}, config),

  listExecutions: (workflowId: string, config?: RequestConfig) =>
    baseApi.get<WorkflowExecutionSummary[]>(API.WORKFLOW_EXECUTIONS(workflowId), config),

  getAllNotes: (workflowId: string, config?: RequestConfig) =>
    baseApi.get<Array<{ step_id: string; content: string }>>(API.WORKFLOW_NOTES(workflowId), config),
})

const contextResponse = freeze({
  get: (config?: RequestConfig) => baseApi.get<unknown>(API.CONTEXT_RESPONSE, config),
})

const modes = freeze({
  list: (config?: RequestConfig) => baseApi.get<unknown>(API.MODES, config),
})

const toolRouters = freeze({
  list: (config?: RequestConfig) => baseApi.get<ToolRouter[]>(API.TOOL_ROUTERS, config),

  get: (id: string, config?: RequestConfig) => baseApi.get<ToolRouter>(API.TOOL_ROUTER(id), config),

  create: (body: CreateToolRouterRequest, config?: RequestConfig) => baseApi.post<ToolRouter>(API.TOOL_ROUTERS, body, config),

  update: (id: string, body: UpdateToolRouterRequest, config?: RequestConfig) => baseApi.put<ToolRouter>(API.TOOL_ROUTER(id), body, config),

  delete: (id: string, config?: RequestConfig) => baseApi.del<void>(API.TOOL_ROUTER(id), config),

  getTools: (id: string, config?: RequestConfig) => baseApi.get<Tool[]>(API.TOOL_ROUTER_TOOLS(id), config),

  setTools: (id: string, body: SetRouterToolsRequest, config?: RequestConfig) => baseApi.put<void>(API.TOOL_ROUTER_TOOLS(id), body, config),
})

const routerModes = freeze({
  listByRouter: (routerId: string, config?: RequestConfig) => baseApi.get<RouterMode[]>(API.ROUTER_MODES_BY_ROUTER(routerId), config),

  createForRouter: (routerId: string, body: CreateRouterModeRequest, config?: RequestConfig) =>
    baseApi.post<RouterMode>(API.ROUTER_MODES_BY_ROUTER(routerId), body, config),

  get: (id: string, config?: RequestConfig) => baseApi.get<RouterMode>(API.ROUTER_MODE(id), config),

  update: (id: string, body: UpdateRouterModeRequest, config?: RequestConfig) => baseApi.put<RouterMode>(API.ROUTER_MODE(id), body, config),

  delete: (id: string, config?: RequestConfig) => baseApi.del<void>(API.ROUTER_MODE(id), config),

  getTools: (id: string, config?: RequestConfig) => baseApi.get<Tool[]>(API.MODE_TOOLS(id), config),

  setTools: (id: string, body: SetModeToolsRequest, config?: RequestConfig) => baseApi.put<void>(API.MODE_TOOLS(id), body, config),
})

const rooms = freeze({
  get: (id: string, config?: RequestConfig) => baseApi.get<Room>(API.ROOM(id), config),

  create: (body: CreateRoomRequest, config?: RequestConfig) => baseApi.post<Room>(API.ROOMS, body, config),

  update: (id: string, body: UpdateRoomRequest, config?: RequestConfig) => baseApi.put<Room>(API.ROOM(id), body, config),

  delete: (id: string, config?: RequestConfig) => baseApi.del<void>(API.ROOM(id), config),

  listMembers: (id: string, config?: RequestConfig) => baseApi.get<RoomMember[]>(API.ROOM_MEMBERS(id), config),

  addMember: (id: string, body: AddRoomMemberRequest, config?: RequestConfig) => baseApi.post<void>(API.ROOM_MEMBERS(id), body, config),

  setMembers: (id: string, body: SetRoomMembersRequest, config?: RequestConfig) => baseApi.put<void>(API.ROOM_MEMBERS(id), body, config),

  removeMember: (id: string, agentId: string, config?: RequestConfig) => baseApi.del<void>(API.ROOM_MEMBER(id, agentId), config),

  createSession: (id: string, config?: RequestConfig) => baseApi.post<RoomSession>(API.ROOM_SESSIONS(id), undefined, config),
})

const roomSessions = freeze({
  get: (id: string, config?: RequestConfig) => baseApi.get<RoomSession>(API.ROOM_SESSION(id), config),

  sendMessage: (id: string, body: RoomMessageRequest, config?: RequestConfig) =>
    baseApi.post<void>(API.ROOM_SESSION_MESSAGES(id), body, config),

  getTranscript: (id: string, config?: RequestConfig) => baseApi.get<RoomTranscriptEntry[]>(API.ROOM_SESSION_TRANSCRIPT(id), config),

  close: (id: string, config?: RequestConfig) => baseApi.post<RoomSession>(API.ROOM_SESSION_CLOSE(id), undefined, config),

  listOutputs: (id: string, config?: RequestConfig) => baseApi.get<RoomOutput[]>(API.ROOM_SESSION_OUTPUTS(id), config),
})

const collections = freeze({
  list: (config?: RequestConfig) => baseApi.get<Collection[]>(API.COLLECTIONS, config),

  get: (id: string, config?: RequestConfig) => baseApi.get<Collection>(API.COLLECTION(id), config),

  create: (body: CreateCollectionRequest, config?: RequestConfig) => baseApi.post<Collection>(API.COLLECTIONS, body, config),

  update: (id: string, body: UpdateCollectionRequest, config?: RequestConfig) => baseApi.put<Collection>(API.COLLECTION(id), body, config),

  delete: (id: string, config?: RequestConfig) => baseApi.del<void>(API.COLLECTION(id), config),

  run: (id: string, config?: RequestConfig) => baseApi.post<CollectionRun>(API.COLLECTION_RUN(id), undefined, config),

  getRunStatus: (runId: string, config?: RequestConfig) => baseApi.get<CollectionRun>(API.COLLECTION_RUN_STATUS(runId), config),
})

const protocols = freeze({
  list: (config?: RequestConfig) => baseApi.get<Protocol[]>(API.PROTOCOLS, config),

  get: (id: string, config?: RequestConfig) => baseApi.get<Protocol>(API.PROTOCOL(id), config),

  create: (body: CreateProtocolRequest, config?: RequestConfig) => baseApi.post<Protocol>(API.PROTOCOLS, body, config),

  update: (id: string, body: UpdateProtocolRequest, config?: RequestConfig) => baseApi.put<Protocol>(API.PROTOCOL(id), body, config),

  delete: (id: string, config?: RequestConfig) => baseApi.del<void>(API.PROTOCOL(id), config),

  listTypes: (config?: RequestConfig) => baseApi.get<ProtocolTypesResponse>(API.PROTOCOL_TYPES, config),

  createPort: (protocolId: string, body: CreatePortRequest, config?: RequestConfig) =>
    baseApi.post<ProtocolPort>(API.PROTOCOL_PORTS(protocolId), body, config),

  deletePort: (protocolId: string, portId: string, config?: RequestConfig) =>
    baseApi.del<void>(API.PROTOCOL_PORT(protocolId, portId), config),

  preview: (id: string, config?: RequestConfig) => baseApi.post<unknown>(API.PROTOCOL_PREVIEW(id), undefined, config),
})

// ============================================================================
// Merge base API methods with typed endpoints into single `api` export
// ============================================================================

export const api = Object.freeze({
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
  config: appConfig,
  stats,
  agentExecutions,
  outputSchemas,
  promptTemplates,
  costs,
  results,
  workflows,
  contextResponse,
  modes,
  toolRouters,
  routerModes,
  rooms,
  roomSessions,
  collections,
  protocols,
})

export type Api = typeof api
