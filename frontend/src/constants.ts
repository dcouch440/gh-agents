// App
export const APP_NAME = 'nexor'
export const APP_VERSION = '0.1.0'

// API
export const API_BASE = import.meta.env.VITE_API_URL ?? '/api'
export const WS_URL = import.meta.env.VITE_WS_URL ?? `${window.location.protocol === 'https:' ? 'wss:' : 'ws:'}//${window.location.host}/ws`

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

// Mock Data
export const USE_MOCK_DATA = import.meta.env.VITE_USE_MOCK_DATA === 'true'

// Local Storage
export const LS_AUTH_TOKEN = 'nexor_auth_token'
export const LS_THEME = 'nexor_theme'
