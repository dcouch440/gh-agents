import { ApiError } from './client'

const isApiError = (error: unknown): error is ApiError =>
  error instanceof ApiError

const isHttpError = (error: unknown): error is ApiError & { readonly type: 'http_error'; readonly status: number } =>
  error instanceof ApiError && error.type === 'http_error'

const isNetworkError = (error: unknown): error is ApiError & { readonly type: 'network_error' } =>
  error instanceof ApiError && error.type === 'network_error'

const isTimeoutError = (error: unknown): error is ApiError & { readonly type: 'timeout_error' } =>
  error instanceof ApiError && error.type === 'timeout_error'

const isAbortError = (error: unknown): error is ApiError & { readonly type: 'abort_error' } =>
  error instanceof ApiError && error.type === 'abort_error'

/**
 * The server is throttling us. Distinct from a plain `http_error` because the
 * right response is to slow down and retry, not to report a failure.
 */
const isRateLimitError = (error: unknown): error is ApiError & { readonly type: 'rate_limit_error'; readonly status: number } =>
  error instanceof ApiError && error.type === 'rate_limit_error'

/** A 429 carries a status like any other HTTP failure, it just has its own type. */
const hasHttpStatus = (error: unknown): error is ApiError & { readonly status: number } =>
  isHttpError(error) || isRateLimitError(error)

const hasStatus = (error: unknown, status: number): error is ApiError & { readonly status: number } =>
  hasHttpStatus(error) && error.status === status

const isClientError = (error: unknown): boolean =>
  hasHttpStatus(error) && error.status >= 400 && error.status < 500

const isServerError = (error: unknown): boolean =>
  hasHttpStatus(error) && error.status >= 500 && error.status < 600

export { isApiError, isHttpError, isNetworkError, isTimeoutError, isAbortError, isRateLimitError, hasStatus, isClientError, isServerError }
