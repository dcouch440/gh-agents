// App
export const APP_NAME = 'nexor'
export const APP_VERSION = '0.1.0'

// API
export const API_BASE: string = (import.meta.env.VITE_API_URL as string | undefined) ?? '/api'
export const WS_URL: string = (import.meta.env.VITE_WS_URL as string | undefined) ?? `${window.location.protocol === 'https:' ? 'wss:' : 'ws:'}//${window.location.host}/ws`

// WS Channels
export const WS_CHANNEL = {
  FEED: 'feed',
  TASKS: 'tasks',
  AGENTS: 'agents',
  SESSIONS: 'sessions',
  PIPELINES: 'pipelines',
  ROUTING: 'routing',
} as const

export type WsChannel = (typeof WS_CHANNEL)[keyof typeof WS_CHANNEL]

// WS Reconnect
export const WS_RECONNECT_BASE_MS = 1000
export const WS_RECONNECT_MAX_MS = 30000

// Polling
export const STATS_POLL_INTERVAL_MS = 5000

// Routes
export const ROUTES = {
  DASHBOARD: '/',
  CHAT: '/chat',
  CHAT_SESSION: '/chat/:sessionId',
  AGENTS: '/agents',
  AGENT_DETAIL: '/agents/:id',
  PIPELINES: '/pipelines',
  PIPELINE_DETAIL: '/pipelines/:id',
  PIPELINE_RUN: '/pipelines/:id/runs/:runId',
  TASKS: '/tasks',
  DOCUMENTS: '/documents',
  SETTINGS: '/settings',
  SHOWCASE: '/showcase',
} as const

// Reducer Action Types
export const ACTION = {
  SET_ALL: 'SET_ALL',
  SET_LOADING: 'SET_LOADING',
  SET_ERROR: 'SET_ERROR',
  UPDATE_ONE: 'UPDATE_ONE',
  REMOVE_ONE: 'REMOVE_ONE',
  APPEND: 'APPEND',
  CLEAR: 'CLEAR',
  UPDATE: 'UPDATE',
  SET_PIPELINES: 'SET_PIPELINES',
  SET_RUNS: 'SET_RUNS',
  UPDATE_RUN: 'UPDATE_RUN',
} as const

// API Endpoints
export const API = {
  // Auth
  AUTH_LOGIN: '/auth/login',
  AUTH_REGISTER: '/auth/register',
  AUTH_ME: '/auth/me',

  // Agents
  AGENTS: '/agents',
  AGENT: (id: string) => `/agents/${id}`,
  AGENT_TOOLS: (id: string) => `/agents/${id}/tools`,
  AGENT_CONTEXT: (id: string) => `/agents/${id}/context`,

  // Tasks
  TASKS: '/tasks',
  TASK: (id: string) => `/tasks/${id}`,

  // Tools
  TOOLS: '/tools',
  TOOL: (id: string) => `/tools/${id}`,

  // Documents
  DOCUMENTS: '/documents',
  DOCUMENT: (id: string) => `/documents/${id}`,
  DOCUMENTS_SEARCH: (q: string) => `/documents/search?q=${encodeURIComponent(q)}`,

  // Sessions
  SESSIONS: '/sessions',
  SESSION: (id: string) => `/sessions/${id}`,
  SESSION_CHAT: (id: string) => `/sessions/${id}/chat`,
  SESSION_HISTORY: (id: string) => `/sessions/${id}/history`,
  SESSION_CHAT_STREAM: (sessionId: string, messageId: string) => `/sessions/${sessionId}/chat/${messageId}/stream`,

  // Chat
  CHAT: '/chat',
  CHAT_HISTORY: '/chat/history',
  CHAT_STREAM: (messageId: string) => `/chat/${messageId}/stream`,

  // Modes
  MODES: '/modes',

  // Config
  CONFIG: '/config',

  // Stats
  STATS: '/stats',

  // Pipelines
  PIPELINES: '/pipelines',
  PIPELINE: (id: string) => `/pipelines/${id}`,
  PIPELINE_STAGE_RENDER: (id: string, stage: number) => `/pipelines/${id}/stages/${stage}/render`,
  PIPELINE_SIDE_TASKS: (id: string, stage: number) => `/pipelines/${id}/stages/${stage}/side-tasks`,
  PIPELINE_SIDE_TASK: (id: string, stage: number, taskId: string) => `/pipelines/${id}/stages/${stage}/side-tasks/${taskId}`,
  PIPELINE_RUNS: '/pipeline-runs',
  PIPELINE_RUN: (id: string) => `/pipeline-runs/${id}`,
  PIPELINE_RUN_APPROVE: (id: string) => `/pipeline-runs/${id}/approve`,

  // Context Response
  CONTEXT_RESPONSE: '/context-response',
} as const

// Mock Data
export const USE_MOCK_DATA = import.meta.env.VITE_USE_MOCK_DATA === 'true'

// Local Storage
export const LS_AUTH_TOKEN = 'nexor_auth_token'
export const LS_THEME = 'nexor_theme'
