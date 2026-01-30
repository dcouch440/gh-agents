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
    login: (password: string) =>
      fetchApi<{ token: string; expires_in: number }>('/auth/login', {
        method: 'POST',
        body: JSON.stringify({ password }),
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
    stream: (messageId: string, onToken: (text: string) => void, onDone: () => void, onError: (err: string) => void) => {
      const eventSource = new EventSource(
        `${API_BASE}/chat/${messageId}/stream`
      );
      eventSource.onmessage = (event) => {
        onToken(event.data);
      };
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
  tier: string;
  status: string;
  current_task?: string;
}

export interface ChatMessage {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  timestamp: string;
}

export interface Config {
  verbosity: string;
  models: Record<string, unknown>;
  pool: Record<string, unknown>;
}
