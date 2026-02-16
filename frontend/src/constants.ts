// App
export const APP_NAME = 'nexor'
export const APP_VERSION = '0.1.0'

// API
export const API_BASE: string = (import.meta.env.VITE_API_URL as string | undefined) ?? '/api'
export const WS_URL: string =
  (import.meta.env.VITE_WS_URL as string | undefined) ??
  `${window.location.protocol === 'https:' ? 'wss:' : 'ws:'}//${window.location.host}/ws`

// WS Types (re-exported from types/ws.ts)
export { WS_TOPIC, WS_STATUS, WS_MSG, WS_CONTROL, WORKFLOW_EVENT, ROOM_EVENT, SESSION_EVENT } from '@/types/ws'
export type { WsTopic, WsStatus, WsWireMessage } from '@/types/ws'

// SSE Events (re-exported from types/streaming.ts)
export { SSE_EVENT, isContentEvent } from '@/types/streaming'

// WS Reconnect
export const WS_RECONNECT_BASE_MS = 1000
export const WS_RECONNECT_MAX_MS = 30000

// Polling
export const STATS_POLL_INTERVAL_MS = 5000

// Routes
export const ROUTES = {
  LOGIN: '/login',
  DASHBOARD: '/',
  CHAT: '/chat',
  CHAT_SESSION: '/chat/:sessionId',
  AGENTS: '/agents',
  AGENT_WORKSHOP: '/agents/workshop/:sessionId?',
  AGENT_DETAIL: '/agents/:id',
  TASKS: '/tasks',
  DOCUMENTS: '/documents',
  DOCUMENT_DETAIL: '/documents/:id',
  WORKFLOWS: '/workflows',
  WORKFLOW_EDITOR: '/workflows/:id',
  WORKFLOW_RUNS: '/workflows/:id/runs',
  WORKFLOW_RUN_DETAIL: '/workflows/:id/runs/:runId',
  SCHEMAS: '/schemas',
  SCHEMA_DETAIL: '/schemas/:id',
  PROMPTS: '/prompts',
  PROMPT_DETAIL: '/prompts/:id',
  RESULTS: '/results',
  COSTS: '/costs',
  SETTINGS: '/settings',
  SHOWCASE: '/showcase',
  REVIEW_QUEUE: '/review-queue',
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
  SET_TREE: 'SET_TREE',
  UPDATE_AGENT_EXECUTION: 'UPDATE_AGENT_EXECUTION',
  ADD_FOR_EACH_NODES: 'ADD_FOR_EACH_NODES',
  UPDATE_STAGE_EXECUTION: 'UPDATE_STAGE_EXECUTION',
  SET_CURRENT: 'SET_CURRENT',
  CLEAR_CURRENT: 'CLEAR_CURRENT',
  SET_MESSAGES: 'SET_MESSAGES',
  APPEND_MESSAGE: 'APPEND_MESSAGE',
  SET_QUEUE: 'SET_QUEUE',
  ADD_TO_QUEUE: 'ADD_TO_QUEUE',
  REMOVE_FROM_QUEUE: 'REMOVE_FROM_QUEUE',
  DISMISS_NOTIFICATION: 'DISMISS_NOTIFICATION',
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
  SESSION_CHAT_CANCEL: (sessionId: string, messageId: string) => `/sessions/${sessionId}/chat/${messageId}/cancel`,
  SESSION_MESSAGES: (id: string) => `/sessions/${id}/messages`,

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
  PIPELINE_RUN_CANCEL: (id: string) => `/pipeline-runs/${id}/cancel`,

  // Workflows
  WORKFLOWS: '/workflows',
  WORKFLOW: (id: string) => `/workflows/${id}`,
  WORKFLOW_STEPS: (wid: string) => `/workflows/${wid}/steps`,
  WORKFLOW_STEP: (wid: string, sid: string) => `/workflows/${wid}/steps/${sid}`,
  WORKFLOW_EDGES: (wid: string) => `/workflows/${wid}/edges`,
  WORKFLOW_EDGE: (wid: string, eid: string) => `/workflows/${wid}/edges/${eid}`,
  STEP_DOCUMENTS: (wid: string, sid: string) => `/workflows/${wid}/steps/${sid}/documents`,
  STEP_DOCUMENT: (wid: string, sid: string, did: string) => `/workflows/${wid}/steps/${sid}/documents/${did}`,
  STEP_DOCUMENT_DEFS: (wid: string, sid: string) => `/workflows/${wid}/steps/${sid}/document-defs`,
  STEP_DOCUMENT_DEF: (wid: string, sid: string, did: string) => `/workflows/${wid}/steps/${sid}/document-defs/${did}`,
  STEP_AGENT_ROSTER: (wid: string, sid: string) => `/workflows/${wid}/steps/${sid}/agent-roster`,
  STEP_ROSTER_AGENT: (wid: string, sid: string, rid: string) => `/workflows/${wid}/steps/${sid}/agent-roster/${rid}`,
  STEP_ROOM_MEMBERS: (wid: string, sid: string) => `/workflows/${wid}/steps/${sid}/room-members`,
  STEP_CHAT_SESSION: (wid: string, sid: string) => `/workflows/${wid}/steps/${sid}/chat/session`,
  STEP_CHAT_MESSAGES: (wid: string, sid: string) => `/workflows/${wid}/steps/${sid}/chat/messages`,
  STEP_CHAT_DEBUG: (wid: string, sid: string) => `/workflows/${wid}/steps/${sid}/chat/debug`,
  STEP_LAST_RUN: (wid: string, sid: string) => `/workflows/${wid}/steps/${sid}/last-run`,
  WORKFLOW_RUN: (id: string) => `/workflows/${id}/run`,
  WORKFLOW_EXECUTIONS: (id: string) => `/workflows/${id}/executions`,
  WORKFLOW_EXECUTION_STEPS: (wid: string, eid: string) => `/workflows/${wid}/executions/${eid}/steps`,
  WORKFLOW_EXECUTION_STEP: (wid: string, eid: string, sid: string) => `/workflows/${wid}/executions/${eid}/steps/${sid}`,
  WORKFLOW_NOTES: (id: string) => `/workflows/${id}/notes`,
  WORKFLOW_REBASE: (id: string) => `/workflows/${id}/rebase`,
  WORKFLOW_TEMPLATES: (id: string) => `/workflows/${id}/templates`,

  // Pipeline Stage Members
  STAGE_MEMBERS: (pid: string, num: number) => `/pipelines/${pid}/stages/${num}/members`,
  STAGE_MEMBER: (pid: string, num: number, mid: string) => `/pipelines/${pid}/stages/${num}/members/${mid}`,

  // Agent Executions
  AGENT_EXECUTIONS: '/agent-executions',
  AGENT_EXECUTION: (id: string) => `/agent-executions/${id}`,
  EXECUTION_MESSAGES: (id: string) => `/agent-executions/${id}/messages`,
  EXECUTION_MESSAGE_STREAM: (id: string, streamId: string) => `/agent-executions/${id}/messages/${streamId}/stream`,
  EXECUTION_APPROVE: (id: string) => `/agent-executions/${id}/approve`,
  AGENT_EXECUTION_CANCEL: (id: string) => `/agent-executions/${id}/cancel`,

  // Pipeline Run Tree
  PIPELINE_RUN_TREE: (runId: string) => `/pipeline-runs/${runId}/tree`,

  // Output Schemas
  OUTPUT_SCHEMAS: '/output-schemas',
  OUTPUT_SCHEMA: (id: string) => `/output-schemas/${id}`,

  // Prompt Templates
  PROMPT_TEMPLATES: '/prompt-templates',
  PROMPT_TEMPLATE: (id: string) => `/prompt-templates/${id}`,

  // Costs
  COSTS: '/costs',

  // Results
  RESULTS: '/results',
  RESULT: (id: string) => `/results/${id}`,

  // Tool Routers
  TOOL_ROUTERS: '/tool-routers',
  TOOL_ROUTER: (id: string) => `/tool-routers/${id}`,
  TOOL_ROUTER_TOOLS: (id: string) => `/tool-routers/${id}/tools`,

  // Router Modes
  ROUTER_MODES_BY_ROUTER: (routerId: string) => `/tool-routers/${routerId}/modes`,
  ROUTER_MODE: (id: string) => `/router-modes/${id}`,
  MODE_TOOLS: (id: string) => `/router-modes/${id}/tools`,

  // Context Response
  CONTEXT_RESPONSE: '/context-response',

  // Rooms
  ROOMS: '/rooms',
  ROOM: (id: string) => `/rooms/${id}`,
  ROOM_MEMBERS: (id: string) => `/rooms/${id}/members`,
  ROOM_MEMBER: (id: string, agentId: string) => `/rooms/${id}/members/${agentId}`,
  ROOM_SESSIONS: (id: string) => `/rooms/${id}/sessions`,

  // Room Sessions
  ROOM_SESSION: (id: string) => `/room-sessions/${id}`,
  ROOM_SESSION_MESSAGES: (id: string) => `/room-sessions/${id}/messages`,
  ROOM_SESSION_TRANSCRIPT: (id: string) => `/room-sessions/${id}/transcript`,
  ROOM_SESSION_CLOSE: (id: string) => `/room-sessions/${id}/close`,
  ROOM_SESSION_OUTPUTS: (id: string) => `/room-sessions/${id}/outputs`,

  // Collections
  COLLECTIONS: '/collections',
  COLLECTION: (id: string) => `/collections/${id}`,
  COLLECTION_RUN: (id: string) => `/collections/${id}/run`,
  COLLECTION_RUN_STATUS: (runId: string) => `/collections/runs/${runId}/status`,

  // Protocols
  PROTOCOLS: '/protocols',
  PROTOCOL: (id: string) => `/protocols/${id}`,
  PROTOCOL_TYPES: '/protocols/types',
  PROTOCOL_PORTS: (id: string) => `/protocols/${id}/ports`,
  PROTOCOL_PORT: (protocolId: string, portId: string) => `/protocols/${protocolId}/ports/${portId}`,
  PROTOCOL_PREVIEW: (id: string) => `/protocols/${id}/preview`,
  PROTOCOL_APPLY: (id: string, stepId: string) => `/protocols/${id}/apply/${stepId}`,
  PROTOCOL_UNAPPLY: (protocolId: string, stepId: string) => `/protocols/${protocolId}/unapply/${stepId}`,
} as const

// Local Storage
export const LS_AUTH_TOKEN = 'nexor_auth_token'
export const LS_THEME = 'nexor_theme'
export const LS_SIDEBAR_COLLAPSED = 'nexor_sidebar_collapsed'
export const LS_RECENT_COMMANDS = 'nexor_recent_commands'
export const LS_LEFT_PANEL_OPEN = 'nexor_left_panel_open'
export const LS_LEFT_PANEL_SECTION = 'nexor_left_panel_section'
export const LS_RIGHT_PANEL_WIDTH = 'nexor_right_panel_width'

// Layout
export const LAYOUT = {
  TOPBAR_HEIGHT: 34,
  RAIL_WIDTH: 36,
  PANEL_WIDTH: 220,
  PANEL_MIN_WIDTH: 240,
  PANEL_MAX_WIDTH: 480,
} as const

/** @deprecated Use LAYOUT instead */
export const SIDEBAR = {
  WIDTH_EXPANDED: 260,
  WIDTH_COLLAPSED: 64,
} as const

// Semantic identity colors (mode-independent — used for visual identity, not surfaces)
// Surface/overlay tokens are in src/theme/customTokens.ts and accessed via theme.palette.custom.*
export const DESIGN = {
  // Port type colors
  PORT_STRING: '#3b82f6',
  PORT_JSON: '#a78bfa',
  PORT_ARRAY: '#2dd4bf',
  PORT_NUMBER: '#f59e0b',
  PORT_ANY: '#7d8590',

  // Syntax highlighting
  SYN_KEYWORD: '#ff7b72',
  SYN_STRING: '#a5d6ff',
  SYN_VARIABLE: '#2dd4bf',
  SYN_COMMENT: '#484f58',
  SYN_FUNCTION: '#d2a8ff',
  SYN_TAG: '#7ee787',
} as const

// Focus Mode
export const FOCUS_MODE = {
  ARTIFACT_BAR_HEIGHT: 92,
  NAV_BAR_HEIGHT: 48,
  CARD_WIDTH: 112,
  CARD_HEIGHT: 68,
  Z_INDEX: 1400,
  CONTENT_MAX_WIDTH: 960,
  HEADER_HEIGHT: 80,
  TAB_STRIP_HEIGHT: 40,
} as const

// Auto-save
export const AUTO_SAVE_DEBOUNCE_MS = 500

// Animation
export const ANIMATION = {
  FAST: 150,
  NORMAL: 200,
  SLOW: 300,
  PAGE_TRANSITION: 250,
} as const

// Command Palette
export const COMMAND_PALETTE = {
  MAX_RECENT: 5,
  MAX_RESULTS: 10,
} as const
