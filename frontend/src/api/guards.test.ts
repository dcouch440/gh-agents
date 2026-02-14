import { describe, it, expect } from 'vitest'
import { ApiError } from './client'
import {
  isApiError,
  isHttpError,
  isNetworkError,
  isTimeoutError,
  isAbortError,
  hasStatus,
  isClientError,
  isServerError,
} from './guards'

describe('isApiError', () => {
  it('returns true for ApiError instances', () => {
    expect(isApiError(ApiError.network('/test', new Error('fail')))).toBe(true)
  })

  it('returns false for plain Error', () => {
    expect(isApiError(new Error('fail'))).toBe(false)
  })

  it('returns false for null', () => {
    expect(isApiError(null)).toBe(false)
  })

  it('returns false for undefined', () => {
    expect(isApiError(undefined)).toBe(false)
  })

  it('returns false for string', () => {
    expect(isApiError('error')).toBe(false)
  })
})

describe('isHttpError', () => {
  it('returns true for http_error type', () => {
    const error = ApiError.http('/test', 404, 'Not Found', null)
    expect(isHttpError(error)).toBe(true)
  })

  it('returns false for network_error type', () => {
    const error = ApiError.network('/test', new Error('fail'))
    expect(isHttpError(error)).toBe(false)
  })

  it('returns false for non-ApiError', () => {
    expect(isHttpError(new Error('fail'))).toBe(false)
  })

  it('narrows status to number', () => {
    const error: unknown = ApiError.http('/test', 404, 'Not Found', null)
    if (isHttpError(error)) {
      const status: number = error.status
      expect(status).toBe(404)
    } else {
      expect.fail('Expected isHttpError to return true')
    }
  })
})

describe('isNetworkError', () => {
  it('returns true for network_error type', () => {
    expect(isNetworkError(ApiError.network('/test', new Error('fail')))).toBe(true)
  })

  it('returns false for http_error type', () => {
    expect(isNetworkError(ApiError.http('/test', 500, 'Error', null))).toBe(false)
  })
})

describe('isTimeoutError', () => {
  it('returns true for timeout_error type', () => {
    expect(isTimeoutError(ApiError.timeout('/test'))).toBe(true)
  })

  it('returns false for abort_error type', () => {
    expect(isTimeoutError(ApiError.abort('/test'))).toBe(false)
  })
})

describe('isAbortError', () => {
  it('returns true for abort_error type', () => {
    expect(isAbortError(ApiError.abort('/test'))).toBe(true)
  })

  it('returns false for timeout_error type', () => {
    expect(isAbortError(ApiError.timeout('/test'))).toBe(false)
  })
})

describe('hasStatus', () => {
  it('returns true when status matches', () => {
    const error = ApiError.http('/test', 404, 'Not Found', null)
    expect(hasStatus(error, 404)).toBe(true)
  })

  it('returns false when status differs', () => {
    const error = ApiError.http('/test', 404, 'Not Found', null)
    expect(hasStatus(error, 500)).toBe(false)
  })

  it('returns false for non-http errors', () => {
    expect(hasStatus(ApiError.timeout('/test'), 408)).toBe(false)
  })

  it('returns false for non-ApiError', () => {
    expect(hasStatus(new Error('fail'), 500)).toBe(false)
  })
})

describe('isClientError', () => {
  it('returns true for 400', () => {
    expect(isClientError(ApiError.http('/test', 400, 'Bad Request', null))).toBe(true)
  })

  it('returns true for 404', () => {
    expect(isClientError(ApiError.http('/test', 404, 'Not Found', null))).toBe(true)
  })

  it('returns true for 422', () => {
    expect(isClientError(ApiError.http('/test', 422, 'Unprocessable', null))).toBe(true)
  })

  it('returns false for 500', () => {
    expect(isClientError(ApiError.http('/test', 500, 'Server Error', null))).toBe(false)
  })

  it('returns false for network errors', () => {
    expect(isClientError(ApiError.network('/test', new Error('fail')))).toBe(false)
  })
})

describe('isServerError', () => {
  it('returns true for 500', () => {
    expect(isServerError(ApiError.http('/test', 500, 'Server Error', null))).toBe(true)
  })

  it('returns true for 503', () => {
    expect(isServerError(ApiError.http('/test', 503, 'Unavailable', null))).toBe(true)
  })

  it('returns false for 404', () => {
    expect(isServerError(ApiError.http('/test', 404, 'Not Found', null))).toBe(false)
  })

  it('returns false for non-ApiError', () => {
    expect(isServerError(new Error('fail'))).toBe(false)
  })
})
