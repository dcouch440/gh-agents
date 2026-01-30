import type {
  AgentsListResponse,
  AuthMeResponse,
  ChatMessage,
  ChatSendResponse,
  HealthResponse,
  LoginResponse,
} from './types.js';

let baseUrl = 'http://127.0.0.1:3000';
let token: string | null = null;

export function setBaseUrl(url: string): void {
  baseUrl = url.replace(/\/+$/, '');
}

export function setToken(t: string): void {
  token = t;
}

export function getBaseUrl(): string {
  return baseUrl;
}

export function getToken(): string {
  return token ?? '';
}

class ApiError extends Error {
  constructor(
    public status: number,
    public body: unknown,
  ) {
    super(`API error ${status}`);
    this.name = 'ApiError';
  }
}

async function fetchApi<T>(
  endpoint: string,
  options: RequestInit = {},
): Promise<T> {
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    ...((options.headers as Record<string, string>) ?? {}),
  };

  if (token) {
    headers['Authorization'] = `Bearer ${token}`;
  }

  const res = await fetch(`${baseUrl}${endpoint}`, {
    ...options,
    headers,
  });

  if (!res.ok) {
    let body: unknown;
    const text = await res.text();
    try {
      body = JSON.parse(text);
    } catch {
      body = text;
    }
    throw new ApiError(res.status, body);
  }

  if (res.status === 204) {
    return undefined as T;
  }

  return res.json() as Promise<T>;
}

export const api = {
  health: () => fetchApi<HealthResponse>('/api/health'),

  auth: {
    login: (email: string, password: string) =>
      fetchApi<LoginResponse>('/api/auth/login', {
        method: 'POST',
        body: JSON.stringify({ email, password }),
      }),
    me: () => fetchApi<AuthMeResponse>('/api/auth/me'),
  },

  agents: {
    list: () => fetchApi<AgentsListResponse>('/api/agents'),
  },

  chat: {
    send: (message: string) =>
      fetchApi<ChatSendResponse>('/api/chat', {
        method: 'POST',
        body: JSON.stringify({ message }),
      }),
    history: (limit = 50, offset = 0) =>
      fetchApi<ChatMessage[]>(
        `/api/chat/history?limit=${limit}&offset=${offset}`,
      ),
    clear: () => fetchApi<void>('/api/chat/history', { method: 'DELETE' }),
  },
};
