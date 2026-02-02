// Main API export with both low-level methods and typed endpoints
export { api } from './api'

// Additional exports from client
export { ApiError, configure, addInterceptor, cancelInFlightRequests } from './client'
export type { RequestConfig, RequestContext, ResponseContext, Interceptor, ApiErrorType } from './client'

// SSE support
export { createSSEStream } from './sse'
export type { SSECallbacks, SSEEvent } from './sse'
