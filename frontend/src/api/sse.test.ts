import { describe, it, expect, beforeEach, vi } from 'vitest'
import { createSSEStream } from './sse'
import type { SSECallbacks } from './sse'

const { mockFetch, mockGetItem } = vi.hoisted(() => ({
  mockFetch: vi.fn(),
  mockGetItem: vi.fn(),
}))

vi.stubGlobal('fetch', mockFetch)
vi.stubGlobal('localStorage', {
  getItem: mockGetItem,
  setItem: vi.fn(),
  removeItem: vi.fn(),
  clear: vi.fn(),
  key: vi.fn(),
  length: 0,
})

vi.mock('@/constants', async () => {
  const actual = await vi.importActual<Record<string, unknown>>('@/constants')
  return { ...actual, API_BASE: 'http://localhost:3000/api' }
})

const createMockReader = (chunks: string[]) => {
  let index = 0
  return {
    read: vi.fn(async () => {
      if (index >= chunks.length) {
        return { done: true, value: undefined }
      }
      const chunk = chunks[index]
      index++
      return { done: false, value: new TextEncoder().encode(chunk) }
    }),
  }
}

describe('createSSEStream', () => {
  let callbacks: SSECallbacks

  beforeEach(() => {
    vi.clearAllMocks()
    callbacks = {
      onEvent: vi.fn(),
      onDone: vi.fn(),
      onError: vi.fn(),
    }
  })

  it('creates stream with auth token', () => {
    mockGetItem.mockReturnValue('test-token')
    const mockReader = createMockReader([])
    mockFetch.mockResolvedValue({
      ok: true,
      body: { getReader: () => mockReader },
    })

    createSSEStream('/test', callbacks)

    expect(mockFetch).toHaveBeenCalledWith('http://localhost:3000/api/test', {
      headers: {
        Accept: 'text/event-stream',
        Authorization: 'Bearer test-token',
      },
      signal: expect.any(AbortSignal),
    })
  })

  it('creates stream without auth token', () => {
    mockGetItem.mockReturnValue(null)
    const mockReader = createMockReader([])
    mockFetch.mockResolvedValue({
      ok: true,
      body: { getReader: () => mockReader },
    })

    createSSEStream('/test', callbacks)

    expect(mockFetch).toHaveBeenCalledWith('http://localhost:3000/api/test', {
      headers: {
        Accept: 'text/event-stream',
      },
      signal: expect.any(AbortSignal),
    })
  })

  it('parses simple data event', async () => {
    mockGetItem.mockReturnValue(null)
    const mockReader = createMockReader(['data: hello\n\n'])
    mockFetch.mockResolvedValue({
      ok: true,
      body: { getReader: () => mockReader },
    })

    createSSEStream('/test', callbacks)

    await vi.waitFor(() => {
      expect(callbacks.onEvent).toHaveBeenCalledWith({
        event: 'message',
        data: 'hello',
      })
    })
  })

  it('parses custom event type', async () => {
    mockGetItem.mockReturnValue(null)
    const mockReader = createMockReader(['event: update\n', 'data: test\n\n'])
    mockFetch.mockResolvedValue({
      ok: true,
      body: { getReader: () => mockReader },
    })

    createSSEStream('/test', callbacks)

    await vi.waitFor(() => {
      expect(callbacks.onEvent).toHaveBeenCalledWith({
        event: 'update',
        data: 'test',
      })
    })
  })

  it('handles multiple events', async () => {
    mockGetItem.mockReturnValue(null)
    const mockReader = createMockReader([
      'data: first\n\n',
      'event: custom\n',
      'data: second\n\n',
      'data: third\n\n',
    ])
    mockFetch.mockResolvedValue({
      ok: true,
      body: { getReader: () => mockReader },
    })

    createSSEStream('/test', callbacks)

    await vi.waitFor(() => {
      expect(callbacks.onEvent).toHaveBeenCalledTimes(3)
      expect(callbacks.onEvent).toHaveBeenNthCalledWith(1, {
        event: 'message',
        data: 'first',
      })
      expect(callbacks.onEvent).toHaveBeenNthCalledWith(2, {
        event: 'custom',
        data: 'second',
      })
      expect(callbacks.onEvent).toHaveBeenNthCalledWith(3, {
        event: 'message',
        data: 'third',
      })
    })
  })

  it('calls onDone when stream ends', async () => {
    mockGetItem.mockReturnValue(null)
    const mockReader = createMockReader(['data: test\n\n'])
    mockFetch.mockResolvedValue({
      ok: true,
      body: { getReader: () => mockReader },
    })

    createSSEStream('/test', callbacks)

    await vi.waitFor(() => {
      expect(callbacks.onDone).toHaveBeenCalledTimes(1)
    })
  })

  it('calls onDone when [DONE] received', async () => {
    mockGetItem.mockReturnValue(null)
    const mockReader = createMockReader(['data: [DONE]\n\n'])
    mockFetch.mockResolvedValue({
      ok: true,
      body: { getReader: () => mockReader },
    })

    createSSEStream('/test', callbacks)

    await vi.waitFor(() => {
      expect(callbacks.onDone).toHaveBeenCalledTimes(1)
      expect(callbacks.onEvent).not.toHaveBeenCalled()
    })
  })

  it('calls onDone when done event received', async () => {
    mockGetItem.mockReturnValue(null)
    const mockReader = createMockReader(['event: done\n', 'data: complete\n\n'])
    mockFetch.mockResolvedValue({
      ok: true,
      body: { getReader: () => mockReader },
    })

    createSSEStream('/test', callbacks)

    await vi.waitFor(() => {
      expect(callbacks.onDone).toHaveBeenCalledTimes(1)
      expect(callbacks.onEvent).not.toHaveBeenCalled()
    })
  })

  it('calls onError on failed response', async () => {
    mockGetItem.mockReturnValue(null)
    mockFetch.mockResolvedValue({
      ok: false,
      status: 500,
      statusText: 'Internal Server Error',
    })

    createSSEStream('/test', callbacks)

    await vi.waitFor(() => {
      expect(callbacks.onError).toHaveBeenCalledWith(
        new Error('SSE request failed: 500 Internal Server Error'),
      )
    })
  })

  it('calls onError when body is not readable', async () => {
    mockGetItem.mockReturnValue(null)
    mockFetch.mockResolvedValue({
      ok: true,
      body: null,
    })

    createSSEStream('/test', callbacks)

    await vi.waitFor(() => {
      expect(callbacks.onError).toHaveBeenCalledWith(
        new Error('Response body is not readable'),
      )
    })
  })

  it('calls onError on fetch failure', async () => {
    mockGetItem.mockReturnValue(null)
    mockFetch.mockRejectedValue(new Error('Network error'))

    createSSEStream('/test', callbacks)

    await vi.waitFor(() => {
      expect(callbacks.onError).toHaveBeenCalledWith(new Error('Network error'))
    })
  })

  it('does not call onError on abort', async () => {
    mockGetItem.mockReturnValue(null)
    const abortError = new DOMException('Aborted', 'AbortError')
    mockFetch.mockRejectedValue(abortError)

    createSSEStream('/test', callbacks)

    await vi.waitFor(() => {
      expect(mockFetch).toHaveBeenCalled()
    })

    await new Promise((resolve) => setTimeout(resolve, 50))
    expect(callbacks.onError).not.toHaveBeenCalled()
  })

  it('returns abort function', () => {
    mockGetItem.mockReturnValue(null)
    const mockReader = createMockReader([])
    mockFetch.mockResolvedValue({
      ok: true,
      body: { getReader: () => mockReader },
    })

    const abort = createSSEStream('/test', callbacks)

    expect(typeof abort).toBe('function')
  })

  it('aborts stream when abort function called', async () => {
    mockGetItem.mockReturnValue(null)
    const mockReader = createMockReader(['data: test\n\n'])
    mockFetch.mockResolvedValue({
      ok: true,
      body: { getReader: () => mockReader },
    })

    const abort = createSSEStream('/test', callbacks)
    abort()

    await new Promise((resolve) => setTimeout(resolve, 50))
  })

  it('handles buffered incomplete lines', async () => {
    mockGetItem.mockReturnValue(null)
    const mockReader = createMockReader(['data: hel', 'lo\n\n'])
    mockFetch.mockResolvedValue({
      ok: true,
      body: { getReader: () => mockReader },
    })

    createSSEStream('/test', callbacks)

    await vi.waitFor(() => {
      expect(callbacks.onEvent).toHaveBeenCalledWith({
        event: 'message',
        data: 'hello',
      })
    })
  })

  it('resets event type after empty line', async () => {
    mockGetItem.mockReturnValue(null)
    const mockReader = createMockReader([
      'event: custom\n',
      '\n',
      'data: message\n\n',
    ])
    mockFetch.mockResolvedValue({
      ok: true,
      body: { getReader: () => mockReader },
    })

    createSSEStream('/test', callbacks)

    await vi.waitFor(() => {
      expect(callbacks.onEvent).toHaveBeenCalledWith({
        event: 'message',
        data: 'message',
      })
    })
  })

  it('handles non-Error exceptions in fetch', async () => {
    mockGetItem.mockReturnValue(null)
    mockFetch.mockRejectedValue('string error')

    createSSEStream('/test', callbacks)

    await vi.waitFor(() => {
      expect(callbacks.onError).toHaveBeenCalledWith(
        new Error('SSE connection failed'),
      )
    })
  })
})
