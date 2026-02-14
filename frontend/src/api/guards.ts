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

const hasStatus = (error: unknown, status: number): error is ApiError & { readonly type: 'http_error'; readonly status: number } =>
  isHttpError(error) && error.status === status

const isClientError = (error: unknown): boolean =>
  isHttpError(error) && error.status >= 400 && error.status < 500

const isServerError = (error: unknown): boolean =>
  isHttpError(error) && error.status >= 500 && error.status < 600

export { isApiError, isHttpError, isNetworkError, isTimeoutError, isAbortError, hasStatus, isClientError, isServerError }
