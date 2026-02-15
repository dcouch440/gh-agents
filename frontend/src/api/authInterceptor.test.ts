import { describe, it, expect, vi, beforeEach } from 'vitest'
import { ApiError } from './client'

// ── Mocks ────────────────────────────────────────────────────────────────────

const { mockLogout, mockAddInterceptor } = vi.hoisted(() => ({
  mockLogout: vi.fn(),
  mockAddInterceptor: vi.fn(() => vi.fn()),
}))

vi.mock('@/stores/authStore', () => ({
  authStore: { logout: mockLogout },
}))

vi.mock('./client', async () => {
  const actual = await vi.importActual<Record<string, unknown>>('./client')
  return { ...actual, addInterceptor: mockAddInterceptor }
})

// ── Setup ────────────────────────────────────────────────────────────────────

type OnErrorHandler = (error: InstanceType<typeof ApiError>) => InstanceType<typeof ApiError>

let onError: OnErrorHandler

beforeEach(async () => {
  vi.clearAllMocks()
  mockAddInterceptor.mockClear()

  vi.resetModules()
  const { setupAuthInterceptor } = await import('./authInterceptor')
  setupAuthInterceptor()

  const interceptor = mockAddInterceptor.mock.calls[0][0] as { onError: OnErrorHandler }
  onError = interceptor.onError
})

// ── Tests ────────────────────────────────────────────────────────────────────

describe('setupAuthInterceptor', () => {
  it('calls authStore.logout on 401 response', () => {
    const error = ApiError.http('/api/agents', 401, 'Unauthorized', null)

    onError(error)

    expect(mockLogout).toHaveBeenCalledOnce()
  })

  it('returns the original error so callers still receive it', () => {
    const error = ApiError.http('/api/agents', 401, 'Unauthorized', null)

    const result = onError(error)

    expect(result).toBe(error)
  })

  it('does not logout on 401 for /auth/login endpoint', () => {
    const error = ApiError.http('/api/auth/login', 401, 'Unauthorized', null)

    onError(error)

    expect(mockLogout).not.toHaveBeenCalled()
  })

  it('does not logout on 401 for /auth/register endpoint', () => {
    const error = ApiError.http('/api/auth/register', 401, 'Unauthorized', null)

    onError(error)

    expect(mockLogout).not.toHaveBeenCalled()
  })

  it('does not logout on 403 response', () => {
    const error = ApiError.http('/api/agents', 403, 'Forbidden', null)

    onError(error)

    expect(mockLogout).not.toHaveBeenCalled()
  })

  it('does not logout on 500 response', () => {
    const error = ApiError.http('/api/agents', 500, 'Internal Server Error', null)

    onError(error)

    expect(mockLogout).not.toHaveBeenCalled()
  })

  it('does not logout on network errors', () => {
    const error = ApiError.network('/api/agents', new Error('fetch failed'))

    onError(error)

    expect(mockLogout).not.toHaveBeenCalled()
  })
})
