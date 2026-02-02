import { API_BASE, LS_AUTH_TOKEN } from '@/constants'

// ============================================================================
// Types
// ============================================================================

type HttpMethod = 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE'

type RequestConfig = {
  timeout?: number
  retries?: number
  retryDelay?: number
  signal?: AbortSignal
  headers?: Record<string, string>
  onUploadProgress?: (progress: number) => void
}

type RequestContext = {
  url: string
  method: HttpMethod
  body?: unknown
  config: RequestConfig
}

type ResponseContext<T> = {
  data: T
  status: number
  statusText: string
  headers: Headers
}

type Interceptor = {
  onRequest?: (ctx: RequestContext) => RequestContext | Promise<RequestContext>
  onResponse?: <T>(ctx: ResponseContext<T>) => ResponseContext<T> | Promise<ResponseContext<T>>
  onError?: (error: ApiError) => ApiError | Promise<ApiError>
}

// ============================================================================
// Error Types
// ============================================================================

type ApiErrorType =
  | 'network_error'
  | 'timeout_error'
  | 'abort_error'
  | 'http_error'
  | 'parse_error'
  | 'validation_error'

class ApiError extends Error {
  readonly type: ApiErrorType
  readonly status?: number
  readonly statusText?: string
  readonly body?: unknown
  readonly url: string

  constructor(
    type: ApiErrorType,
    message: string,
    url: string,
    details?: { status?: number; statusText?: string; body?: unknown }
  ) {
    super(message)
    this.name = 'ApiError'
    this.type = type
    this.url = url
    this.status = details?.status
    this.statusText = details?.statusText
    this.body = details?.body
  }

  static network(url: string, originalError: Error): ApiError {
    return new ApiError('network_error', `Network error: ${originalError.message}`, url)
  }

  static timeout(url: string): ApiError {
    return new ApiError('timeout_error', 'Request timeout', url)
  }

  static abort(url: string): ApiError {
    return new ApiError('abort_error', 'Request aborted', url)
  }

  static http(url: string, status: number, statusText: string, body: unknown): ApiError {
    return new ApiError('http_error', `${status} ${statusText}`, url, { status, statusText, body })
  }

  static parse(url: string, originalError: Error): ApiError {
    return new ApiError('parse_error', `Failed to parse response: ${originalError.message}`, url)
  }
}

// ============================================================================
// Client State
// ============================================================================

type InFlightRequest = {
  promise: Promise<unknown>
  controller: AbortController
}

const inFlightRequests = new Map<string, InFlightRequest>()
const interceptors: Interceptor[] = []

let defaultTimeout = 30000
let defaultRetries = 0
let defaultRetryDelay = 1000
let requestLogger: ((ctx: RequestContext) => void) | null = null
let responseLogger: (<T>(ctx: ResponseContext<T>) => void) | null = null

// ============================================================================
// Configuration
// ============================================================================

const configure = (options: {
  timeout?: number
  retries?: number
  retryDelay?: number
  requestLogger?: (ctx: RequestContext) => void
  responseLogger?: <T>(ctx: ResponseContext<T>) => void
}): void => {
  if (options.timeout !== undefined) defaultTimeout = options.timeout
  if (options.retries !== undefined) defaultRetries = options.retries
  if (options.retryDelay !== undefined) defaultRetryDelay = options.retryDelay
  if (options.requestLogger !== undefined) requestLogger = options.requestLogger
  if (options.responseLogger !== undefined) responseLogger = options.responseLogger
}

const addInterceptor = (interceptor: Interceptor): (() => void) => {
  interceptors.push(interceptor)
  return () => {
    const idx = interceptors.indexOf(interceptor)
    if (idx > -1) interceptors.splice(idx, 1)
  }
}

// ============================================================================
// Core Request Logic
// ============================================================================

const getToken = (): string | null => localStorage.getItem(LS_AUTH_TOKEN)

const buildHeaders = (customHeaders?: Record<string, string>): Record<string, string> => {
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    ...customHeaders,
  }
  const token = getToken()
  if (token) {
    headers['Authorization'] = `Bearer ${token}`
  }
  return headers
}

const sleep = (ms: number): Promise<void> => new Promise((resolve) => setTimeout(resolve, ms))

const executeRequest = async <T>(
  path: string,
  method: HttpMethod,
  body?: unknown,
  config: RequestConfig = {}
): Promise<T> => {
  const url = `${API_BASE}${path}`
  const timeout = config.timeout ?? defaultTimeout
  const retries = config.retries ?? defaultRetries
  const retryDelay = config.retryDelay ?? defaultRetryDelay

  let context: RequestContext = {
    url,
    method,
    body,
    config,
  }

  // Run request interceptors
  for (const interceptor of interceptors) {
    if (interceptor.onRequest) {
      context = await interceptor.onRequest(context)
    }
  }

  // Log request
  if (requestLogger) {
    requestLogger(context)
  }

  let lastError: ApiError | null = null

  for (let attempt = 0; attempt <= retries; attempt++) {
    if (attempt > 0) {
      await sleep(retryDelay * Math.pow(2, attempt - 1))
    }

    try {
      const controller = config.signal ? new AbortController() : new AbortController()
      const timeoutId = setTimeout(() => controller.abort(), timeout)

      if (config.signal) {
        config.signal.addEventListener('abort', () => controller.abort())
      }

      const headers = buildHeaders(config.headers)
      const fetchOptions: RequestInit = {
        method: context.method,
        headers,
        signal: controller.signal,
      }

      if (context.body !== undefined) {
        if (context.body instanceof FormData) {
          delete headers['Content-Type']
          fetchOptions.body = context.body
        } else {
          fetchOptions.body = JSON.stringify(context.body)
        }
      }

      const res = await fetch(context.url, fetchOptions)
      clearTimeout(timeoutId)

      if (!res.ok) {
        const bodyText = await res.text().catch(() => null)
        let parsedBody: unknown = bodyText
        try {
          if (bodyText) parsedBody = JSON.parse(bodyText)
        } catch {
          // Keep as text
        }
        const error = ApiError.http(context.url, res.status, res.statusText, parsedBody)

        // Run error interceptors
        let interceptedError = error
        for (const interceptor of interceptors) {
          if (interceptor.onError) {
            interceptedError = await interceptor.onError(interceptedError)
          }
        }
        throw interceptedError
      }

      let data: T
      if (res.status === 204) {
        data = undefined as T
      } else {
        const text = await res.text()
        try {
          data = (text ? JSON.parse(text) : undefined) as T
        } catch (e) {
          throw ApiError.parse(context.url, e instanceof Error ? e : new Error('Parse failed'))
        }
      }

      let responseContext: ResponseContext<T> = {
        data,
        status: res.status,
        statusText: res.statusText,
        headers: res.headers,
      }

      // Run response interceptors
      for (const interceptor of interceptors) {
        if (interceptor.onResponse) {
          responseContext = await interceptor.onResponse(responseContext)
        }
      }

      // Log response
      if (responseLogger) {
        responseLogger(responseContext)
      }

      return responseContext.data
    } catch (e) {
      if (e instanceof DOMException && e.name === 'AbortError') {
        if (config.signal?.aborted) {
          throw ApiError.abort(context.url)
        }
        throw ApiError.timeout(context.url)
      }
      if (e instanceof ApiError) {
        lastError = e
        // Don't retry on 4xx errors (client errors)
        if (e.status && e.status >= 400 && e.status < 500) {
          throw e
        }
      } else if (e instanceof Error) {
        lastError = ApiError.network(context.url, e)
      } else {
        lastError = ApiError.network(context.url, new Error('Unknown error'))
      }

      if (attempt === retries) {
        throw lastError
      }
    }
  }

  throw lastError!
}

// ============================================================================
// Request Deduplication
// ============================================================================

const getCacheKey = (path: string, method: HttpMethod, body?: unknown): string => {
  const bodyKey = body ? JSON.stringify(body) : ''
  return `${method}:${path}:${bodyKey}`
}

const deduplicate = async <T>(
  path: string,
  method: HttpMethod,
  body: unknown,
  execute: () => Promise<T>
): Promise<T> => {
  // Only deduplicate GET requests
  if (method !== 'GET') {
    return execute()
  }

  const key = getCacheKey(path, method, body)
  const existing = inFlightRequests.get(key)

  if (existing) {
    return existing.promise as Promise<T>
  }

  const controller = new AbortController()
  const promise = execute().finally(() => {
    inFlightRequests.delete(key)
  })

  inFlightRequests.set(key, { promise, controller })
  return promise
}

// ============================================================================
// Public API
// ============================================================================

const request = async <T>(
  path: string,
  method: HttpMethod,
  body?: unknown,
  config?: RequestConfig
): Promise<T> => {
  return deduplicate(path, method, body, () => executeRequest<T>(path, method, body, config))
}

const api = {
  get: <T>(path: string, config?: RequestConfig) => request<T>(path, 'GET', undefined, config),

  post: <T>(path: string, body?: unknown, config?: RequestConfig) =>
    request<T>(path, 'POST', body, config),

  patch: <T>(path: string, body: unknown, config?: RequestConfig) =>
    request<T>(path, 'PATCH', body, config),

  put: <T>(path: string, body: unknown, config?: RequestConfig) =>
    request<T>(path, 'PUT', body, config),

  del: <T = void>(path: string, config?: RequestConfig) => request<T>(path, 'DELETE', undefined, config),
}

const cancelInFlightRequests = (): void => {
  for (const [, { controller }] of inFlightRequests) {
    controller.abort()
  }
  inFlightRequests.clear()
}

// ============================================================================
// Exports
// ============================================================================

export { api, ApiError, configure, addInterceptor, cancelInFlightRequests }
export type { RequestConfig, RequestContext, ResponseContext, Interceptor, ApiErrorType }
