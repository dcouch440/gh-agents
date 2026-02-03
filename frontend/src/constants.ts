// App
export const APP_NAME = "nexor";
export const APP_VERSION = "0.1.0";

// API
export const API_BASE: string =
  (import.meta.env.VITE_API_URL as string | undefined) ?? "/api";
export const WS_URL: string =
  (import.meta.env.VITE_WS_URL as string | undefined) ??
  `${window.location.protocol === "https:" ? "wss:" : "ws:"}//${window.location.host}/ws`;

// WS Channels (must match backend serde tag names in ws.rs ServerMessage)
export const WS_CHANNEL = {
  FEED: "feed",
  TASKS: "task_update",
  AGENTS: "agent_update",
  SESSIONS: "session_update",
  PIPELINES: "pipeline_update",
  ROUTING: "routing_update",
} as const;

export type WsChannel = (typeof WS_CHANNEL)[keyof typeof WS_CHANNEL];

// WS Events for run-scoped subscriptions (from dag_executor broadcasts)
export const WS_EVENT = {
  AGENT_EXECUTION_UPDATE: "agent_execution_update",
  STAGE_EXECUTION_UPDATE: "stage_execution_update",
  PIPELINE_RUN_UPDATE: "pipeline_run_update",
  FOR_EACH_SPAWNED: "for_each_spawned",
  EXECUTION_MESSAGE: "execution_message",
} as const;

export type WsEvent = (typeof WS_EVENT)[keyof typeof WS_EVENT];

// WS Reconnect
export const WS_RECONNECT_BASE_MS = 1000;
export const WS_RECONNECT_MAX_MS = 30000;

// Polling
export const STATS_POLL_INTERVAL_MS = 5000;

// Routes
export const ROUTES = {
  DASHBOARD: "/",
  CHAT: "/chat",
  CHAT_SESSION: "/chat/:sessionId",
  AGENTS: "/agents",
  AGENT_CREATE: "/agents/new",
  AGENT_DETAIL: "/agents/:id",
  PIPELINES: "/pipelines",
  PIPELINE_DETAIL: "/pipelines/:id",
  PIPELINE_RUN: "/pipelines/:id/runs/:runId",
  TASKS: "/tasks",
  DOCUMENTS: "/documents",
  DOCUMENT_DETAIL: "/documents/:id",
  WORKFLOWS: "/workflows",
  WORKFLOW_EDITOR: "/workflows/:id",
  SCHEMAS: "/schemas",
  SCHEMA_DETAIL: "/schemas/:id",
  PROMPTS: "/prompts",
  PROMPT_DETAIL: "/prompts/:id",
  RESULTS: "/results",
  COSTS: "/costs",
  SETTINGS: "/settings",
  SHOWCASE: "/showcase",
} as const;

// Reducer Action Types
export const ACTION = {
  SET_ALL: "SET_ALL",
  SET_LOADING: "SET_LOADING",
  SET_ERROR: "SET_ERROR",
  UPDATE_ONE: "UPDATE_ONE",
  REMOVE_ONE: "REMOVE_ONE",
  APPEND: "APPEND",
  CLEAR: "CLEAR",
  UPDATE: "UPDATE",
  SET_PIPELINES: "SET_PIPELINES",
  SET_RUNS: "SET_RUNS",
  UPDATE_RUN: "UPDATE_RUN",
  SET_TREE: "SET_TREE",
  UPDATE_AGENT_EXECUTION: "UPDATE_AGENT_EXECUTION",
  ADD_FOR_EACH_NODES: "ADD_FOR_EACH_NODES",
  UPDATE_STAGE_EXECUTION: "UPDATE_STAGE_EXECUTION",
  SET_CURRENT: "SET_CURRENT",
  CLEAR_CURRENT: "CLEAR_CURRENT",
  SET_MESSAGES: "SET_MESSAGES",
  APPEND_MESSAGE: "APPEND_MESSAGE",
} as const;

// API Endpoints
export const API = {
  // Auth
  AUTH_LOGIN: "/auth/login",
  AUTH_REGISTER: "/auth/register",
  AUTH_ME: "/auth/me",

  // Agents
  AGENTS: "/agents",
  AGENT: (id: string) => `/agents/${id}`,
  AGENT_TOOLS: (id: string) => `/agents/${id}/tools`,
  AGENT_CONTEXT: (id: string) => `/agents/${id}/context`,

  // Tasks
  TASKS: "/tasks",
  TASK: (id: string) => `/tasks/${id}`,

  // Tools
  TOOLS: "/tools",
  TOOL: (id: string) => `/tools/${id}`,

  // Documents
  DOCUMENTS: "/documents",
  DOCUMENT: (id: string) => `/documents/${id}`,
  DOCUMENTS_SEARCH: (q: string) =>
    `/documents/search?q=${encodeURIComponent(q)}`,

  // Sessions
  SESSIONS: "/sessions",
  SESSION: (id: string) => `/sessions/${id}`,
  SESSION_CHAT: (id: string) => `/sessions/${id}/chat`,
  SESSION_HISTORY: (id: string) => `/sessions/${id}/history`,
  SESSION_CHAT_STREAM: (sessionId: string, messageId: string) =>
    `/sessions/${sessionId}/chat/${messageId}/stream`,

  // Chat
  CHAT: "/chat",
  CHAT_HISTORY: "/chat/history",
  CHAT_STREAM: (messageId: string) => `/chat/${messageId}/stream`,

  // Modes
  MODES: "/modes",

  // Config
  CONFIG: "/config",

  // Stats
  STATS: "/stats",

  // Pipelines
  PIPELINES: "/pipelines",
  PIPELINE: (id: string) => `/pipelines/${id}`,
  PIPELINE_STAGE_RENDER: (id: string, stage: number) =>
    `/pipelines/${id}/stages/${stage}/render`,
  PIPELINE_SIDE_TASKS: (id: string, stage: number) =>
    `/pipelines/${id}/stages/${stage}/side-tasks`,
  PIPELINE_SIDE_TASK: (id: string, stage: number, taskId: string) =>
    `/pipelines/${id}/stages/${stage}/side-tasks/${taskId}`,
  PIPELINE_RUNS: "/pipeline-runs",
  PIPELINE_RUN: (id: string) => `/pipeline-runs/${id}`,
  PIPELINE_RUN_APPROVE: (id: string) => `/pipeline-runs/${id}/approve`,
  PIPELINE_RUN_CANCEL: (id: string) => `/pipeline-runs/${id}/cancel`,

  // Workflows
  WORKFLOWS: "/workflows",
  WORKFLOW: (id: string) => `/workflows/${id}`,
  WORKFLOW_STEPS: (wid: string) => `/workflows/${wid}/steps`,
  WORKFLOW_STEP: (wid: string, sid: string) => `/workflows/${wid}/steps/${sid}`,
  WORKFLOW_EDGES: (wid: string) => `/workflows/${wid}/edges`,
  WORKFLOW_EDGE: (wid: string, eid: string) => `/workflows/${wid}/edges/${eid}`,
  STEP_DOCUMENTS: (wid: string, sid: string) =>
    `/workflows/${wid}/steps/${sid}/documents`,
  STEP_DOCUMENT: (wid: string, sid: string, did: string) =>
    `/workflows/${wid}/steps/${sid}/documents/${did}`,

  // Pipeline Stage Members
  STAGE_MEMBERS: (pid: string, num: number) =>
    `/pipelines/${pid}/stages/${num}/members`,
  STAGE_MEMBER: (pid: string, num: number, mid: string) =>
    `/pipelines/${pid}/stages/${num}/members/${mid}`,

  // Agent Executions
  AGENT_EXECUTION: (id: string) => `/agent-executions/${id}`,
  EXECUTION_MESSAGES: (id: string) => `/agent-executions/${id}/messages`,
  EXECUTION_APPROVE: (id: string) => `/agent-executions/${id}/approve`,
  AGENT_EXECUTION_CANCEL: (id: string) => `/agent-executions/${id}/cancel`,

  // Pipeline Run Tree
  PIPELINE_RUN_TREE: (runId: string) => `/pipeline-runs/${runId}/tree`,

  // Output Schemas
  OUTPUT_SCHEMAS: "/output-schemas",
  OUTPUT_SCHEMA: (id: string) => `/output-schemas/${id}`,

  // Prompt Templates
  PROMPT_TEMPLATES: "/prompt-templates",
  PROMPT_TEMPLATE: (id: string) => `/prompt-templates/${id}`,

  // Costs
  COSTS: "/costs",

  // Results
  RESULTS: "/results",
  RESULT: (id: string) => `/results/${id}`,

  // Context Response
  CONTEXT_RESPONSE: "/context-response",
} as const;

// Local Storage
export const LS_AUTH_TOKEN = "nexor_auth_token";
export const LS_THEME = "nexor_theme";
