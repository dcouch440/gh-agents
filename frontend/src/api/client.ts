import { API_BASE, LS_AUTH_TOKEN } from '@/constants'

class ApiError extends Error {
  constructor(
    public status: number,
    public statusText: string,
    public body: unknown,
  ) {
    super(`${status} ${statusText}`)
    this.name = 'ApiError'
  }
}

const getToken = (): string | null => localStorage.getItem(LS_AUTH_TOKEN)

const request = async <T>(path: string, opts?: RequestInit): Promise<T> => {
  const token = getToken()
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    ...((opts?.headers as Record<string, string>) ?? {}),
  }
  if (token) {
    headers['Authorization'] = `Bearer ${token}`
  }

  const res = await fetch(`${API_BASE}${path}`, { ...opts, headers })

  if (!res.ok) {
    const body = await res.text().catch(() => null)
    throw new ApiError(res.status, res.statusText, body)
  }

  if (res.status === 204) return undefined as T

  return res.json() as Promise<T>
}

export const api = {
  get: <T>(path: string) => request<T>(path),

  post: <T>(path: string, body?: unknown) =>
    request<T>(path, {
      method: 'POST',
      body: body !== undefined ? JSON.stringify(body) : undefined,
    }),

  patch: <T>(path: string, body: unknown) =>
    request<T>(path, {
      method: 'PATCH',
      body: JSON.stringify(body),
    }),

  put: <T>(path: string, body: unknown) =>
    request<T>(path, {
      method: 'PUT',
      body: JSON.stringify(body),
    }),

  del: <T = void>(path: string) =>
    request<T>(path, { method: 'DELETE' }),
}

export { ApiError }
