import { useAuthStore } from '../store';

const API_BASE = '/api';

class ApiError extends Error {
  status: number;

  constructor(status: number, message: string) {
    super(message);
    this.name = 'ApiError';
    this.status = status;
  }
}

async function fetchApi<T>(
  endpoint: string,
  options: RequestInit = {}
): Promise<T> {
  const token = useAuthStore.getState().token;

  const headers: HeadersInit = {
    'Content-Type': 'application/json',
    ...(token && { Authorization: `Bearer ${token}` }),
    ...options.headers,
  };

  const response = await fetch(`${API_BASE}${endpoint}`, {
    ...options,
    headers,
  });

  if (!response.ok) {
    if (response.status === 401) {
      useAuthStore.getState().logout();
    }
    throw new ApiError(response.status, await response.text());
  }

  // Handle 204 No Content
  if (response.status === 204) {
    return undefined as T;
  }

  return response.json();
}

// API methods
export const api = {
  // Health
  health: () => fetchApi<{ status: string; version: string }>('/health'),

  // Auth
  auth: {
    setup: (password: string) =>
      fetchApi<{ message: string }>('/auth/setup', {
        method: 'POST',
        body: JSON.stringify({ password }),
      }),
    login: (email: string, password: string) =>
      fetchApi<{ token: string; expires_in: number }>('/auth/login', {
        method: 'POST',
        body: JSON.stringify({ email, password }),
      }),
    me: () => fetchApi<{ user: string; authenticated: boolean }>('/auth/me'),
  },

  // Tasks
  tasks: {
    list: (params?: { status?: string; limit?: number }) => {
      const query = new URLSearchParams(params as Record<string, string>).toString();
      return fetchApi<Task[]>(`/tasks${query ? `?${query}` : ''}`);
    },
    get: (id: string) => fetchApi<Task>(`/tasks/${id}`),
    create: (data: { title: string; description: string }) =>
      fetchApi<Task>('/tasks', {
        method: 'POST',
        body: JSON.stringify(data),
      }),
  },

  // Agents
  agents: {
    list: () => fetchApi<Agent[]>('/agents'),
  },

  // Chat
  chat: {
    send: (message: string) =>
      fetchApi<{ message_id: string }>('/chat', {
        method: 'POST',
        body: JSON.stringify({ message }),
      }),
    stream: (
      messageId: string,
      onToken: (text: string) => void,
      onDone: () => void,
      onError: (err: string) => void,
      onToolStart?: (data: { name: string; id: string }) => void,
      onToolEnd?: (data: { name: string; id: string }) => void,
      onDocUpdate?: (data: { doc_id: string; title: string }) => void,
    ) => {
      const token = useAuthStore.getState().token;
      const eventSource = new EventSource(
        `${API_BASE}/chat/${messageId}/stream${token ? `?token=${encodeURIComponent(token)}` : ''}`
      );
      eventSource.addEventListener('token', (event) => {
        onToken((event as MessageEvent).data);
      });
      eventSource.addEventListener('tool_start', (event) => {
        onToolStart?.(JSON.parse((event as MessageEvent).data));
      });
      eventSource.addEventListener('tool_end', (event) => {
        onToolEnd?.(JSON.parse((event as MessageEvent).data));
      });
      eventSource.addEventListener('doc_update', (event) => {
        onDocUpdate?.(JSON.parse((event as MessageEvent).data));
      });
      eventSource.addEventListener('done', () => {
        eventSource.close();
        onDone();
      });
      eventSource.addEventListener('error', (event) => {
        eventSource.close();
        onError((event as MessageEvent).data ?? 'Stream error');
      });
      eventSource.onerror = () => {
        eventSource.close();
        onError('Connection lost');
      };
      return eventSource;
    },
    history: (limit?: number, offset?: number) =>
      fetchApi<ChatMessage[]>(`/chat/history?limit=${limit ?? 50}&offset=${offset ?? 0}`),
    clear: () =>
      fetchApi<void>('/chat/history', { method: 'DELETE' }),
  },

  // Modes & Sessions
  modes: {
    list: () => fetchApi<ModeInfo[]>('/modes'),
  },

  sessions: {
    create: (modeId: string, title?: string) =>
      fetchApi<SessionResponse>('/sessions', {
        method: 'POST',
        body: JSON.stringify({ mode_id: modeId, title: title ?? '' }),
      }),
    list: () => fetchApi<SessionResponse[]>('/sessions'),
    get: (sessionId: string) => fetchApi<SessionResponse>(`/sessions/${sessionId}`),
    update: (sessionId: string, title: string) =>
      fetchApi<SessionResponse>(`/sessions/${sessionId}`, {
        method: 'PATCH',
        body: JSON.stringify({ title }),
      }),
    delete: (sessionId: string) =>
      fetchApi<void>(`/sessions/${sessionId}`, { method: 'DELETE' }),
    send: (sessionId: string, message: string) =>
      fetchApi<{ message_id: string }>(`/sessions/${sessionId}/chat`, {
        method: 'POST',
        body: JSON.stringify({ message }),
      }),
    stream: (
      sessionId: string,
      messageId: string,
      onToken: (text: string) => void,
      onDone: () => void,
      onError: (err: string) => void,
      onToolStart?: (data: { name: string; id: string }) => void,
      onToolEnd?: (data: { name: string; id: string }) => void,
      onDocUpdate?: (data: { doc_id: string; title: string }) => void,
    ) => {
      const token = useAuthStore.getState().token;
      const eventSource = new EventSource(
        `${API_BASE}/sessions/${sessionId}/chat/${messageId}/stream${token ? `?token=${encodeURIComponent(token)}` : ''}`
      );
      eventSource.addEventListener('token', (event) => {
        onToken((event as MessageEvent).data);
      });
      eventSource.addEventListener('tool_start', (event) => {
        onToolStart?.(JSON.parse((event as MessageEvent).data));
      });
      eventSource.addEventListener('tool_end', (event) => {
        onToolEnd?.(JSON.parse((event as MessageEvent).data));
      });
      eventSource.addEventListener('doc_update', (event) => {
        onDocUpdate?.(JSON.parse((event as MessageEvent).data));
      });
      eventSource.addEventListener('done', () => {
        eventSource.close();
        onDone();
      });
      eventSource.addEventListener('error', (event) => {
        eventSource.close();
        onError((event as MessageEvent).data ?? 'Stream error');
      });
      eventSource.onerror = () => {
        eventSource.close();
        onError('Connection lost');
      };
      return eventSource;
    },
    history: (sessionId: string, limit?: number) =>
      fetchApi<ChatMessage[]>(`/sessions/${sessionId}/history?limit=${limit ?? 50}`),
  },

  // Stats
  stats: {
    get: () => fetchApi<UsageSummaryRow[]>('/stats'),
  },

  // Indexing
  indexing: {
    status: () => fetchApi<IndexingStatus>('/indexing/status'),
    start: () => fetchApi<{ status: string }>('/indexing/start', { method: 'POST' }),
    stop: () => fetchApi<{ status: string }>('/indexing/stop', { method: 'POST' }),
  },

  // Config
  config: {
    get: () => fetchApi<Config>('/config'),
    update: (data: Partial<Config>) =>
      fetchApi<Config>('/config', {
        method: 'PATCH',
        body: JSON.stringify(data),
      }),
  },
};

// Types
export interface Task {
  id: string;
  title: string;
  description: string;
  status: string;
  priority: string;
  assigned_agent?: string;
  created_at: string;
}

export interface Agent {
  id: string;
  name?: string;
  tier: string;
  role?: string;
  status: string;
  model?: string;
  current_task?: string;
}

export interface ChatMessage {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  timestamp: string;
}

export interface ModelConfig {
  provider: string;
  model_id: string;
  max_tokens: number;
  temperature: number;
}

export interface TierModels {
  orchestrator: ModelConfig;
  worker: ModelConfig;
  utility: ModelConfig;
}

export interface PoolConfig {
  max_orchestrators: number;
  max_workers: number;
  max_utilities: number;
}

export interface Config {
  verbosity: string;
  models: TierModels;
  pool: PoolConfig;
  autonomy: string;
  git_strategy: string;
  sandbox_mode: string;
}

export interface ModeInfo {
  id: string;
  name: string;
  description: string;
}

export interface IndexingStatus {
  state: 'idle' | 'running' | 'complete' | 'failed';
  files_total: number;
  files_indexed: number;
  last_completed: string | null;
  error: string | null;
}

export interface UsageSummaryRow {
  tier: string;
  model_id: string;
  total_input: number;
  total_output: number;
  call_count: number;
}

export interface StatsResponse {
  token_usage: Record<string, number>;
  total_tokens: number;
  call_counts: Record<string, number>;
}

export interface SessionResponse {
  id: string;
  mode_id: string;
  title: string;
  created_at: string;
  updated_at: string;
}
