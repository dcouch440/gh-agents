import type {
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
    try {
      body = await res.json();
    } catch {
      body = await res.text();
    }
    throw new ApiError(res.status, body);
  }

  if (res.status === 204) {
    return undefined as T;
  }

  return res.json() as Promise<T>;
}

export const api = {
  health: () => fetchApi<HealthResponse>('/health'),

  auth: {
    login: (password: string) =>
      fetchApi<LoginResponse>('/auth/login', {
        method: 'POST',
        body: JSON.stringify({ password }),
      }),
    me: () => fetchApi<AuthMeResponse>('/auth/me'),
  },

  chat: {
    send: (message: string) =>
      fetchApi<ChatSendResponse>('/chat', {
        method: 'POST',
        body: JSON.stringify({ message }),
      }),
    history: (limit = 50, offset = 0) =>
      fetchApi<ChatMessage[]>(
        `/chat/history?limit=${limit}&offset=${offset}`,
      ),
    clear: () => fetchApi<void>('/chat/history', { method: 'DELETE' }),
  },
};
