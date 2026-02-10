/* eslint-disable @typescript-eslint/require-await */
/* eslint-disable @typescript-eslint/no-unsafe-assignment */
/* eslint-disable @typescript-eslint/no-unsafe-member-access */
/* eslint-disable @typescript-eslint/no-unsafe-return */
import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest'
import { api, ApiError, configure, addInterceptor, cancelInFlightRequests } from './client'

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

describe('ApiError', () => {
  it('creates network error', () => {
    const original = new Error('Network failed')
    const error = ApiError.network('/test', original)
    expect(error.type).toBe('network_error')
    expect(error.message).toContain('Network failed')
    expect(error.url).toBe('/test')
  })

  it('creates timeout error', () => {
    const error = ApiError.timeout('/test')
    expect(error.type).toBe('timeout_error')
    expect(error.message).toBe('Request timeout')
    expect(error.url).toBe('/test')
  })

  it('creates abort error', () => {
    const error = ApiError.abort('/test')
    expect(error.type).toBe('abort_error')
    expect(error.message).toBe('Request aborted')
    expect(error.url).toBe('/test')
  })

  it('creates http error', () => {
    const error = ApiError.http('/test', 404, 'Not Found', 'Resource not found')
    expect(error.type).toBe('http_error')
    expect(error.status).toBe(404)
    expect(error.statusText).toBe('Not Found')
    expect(error.body).toBe('Resource not found')
    expect(error.message).toBe('404 Not Found')
    expect(error.url).toBe('/test')
  })

  it('creates parse error', () => {
    const original = new Error('Invalid JSON')
    const error = ApiError.parse('/test', original)
    expect(error.type).toBe('parse_error')
    expect(error.message).toContain('Invalid JSON')
    expect(error.url).toBe('/test')
  })
})

describe('api client', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  afterEach(() => {
    vi.restoreAllMocks()
    cancelInFlightRequests()
  })

  describe('GET', () => {
    it('makes GET request without token', async () => {
      mockGetItem.mockReturnValue(null)
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        text: async () => '{"data":"test"}',
        headers: new Headers(),
      })

      const result = await api.get('/test')

      expect(mockFetch).toHaveBeenCalledWith(
        'http://localhost:3000/api/test',
        expect.objectContaining({
          method: 'GET',
          headers: { 'Content-Type': 'application/json' },
        }),
      )
      expect(result).toEqual({ data: 'test' })
    })

    it('makes GET request with token', async () => {
      mockGetItem.mockReturnValue('test-token')
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        text: async () => '{"data":"test"}',
        headers: new Headers(),
      })

      await api.get('/test')

      expect(mockFetch).toHaveBeenCalledWith(
        'http://localhost:3000/api/test',
        expect.objectContaining({
          headers: {
            'Content-Type': 'application/json',
            Authorization: 'Bearer test-token',
          },
        }),
      )
    })

    it('handles 204 No Content', async () => {
      mockGetItem.mockReturnValue(null)
      mockFetch.mockResolvedValue({
        ok: true,
        status: 204,
        headers: new Headers(),
      })

      const result = await api.get('/test')

      expect(result).toBeUndefined()
    })

    it('throws ApiError on failed request with JSON body', async () => {
      mockGetItem.mockReturnValue(null)
      mockFetch.mockResolvedValue({
        ok: false,
        status: 404,
        statusText: 'Not Found',
        text: async () => '{"error":"Not found"}',
        headers: new Headers(),
      })

      await expect(api.get('/test')).rejects.toThrow(ApiError)
      await expect(api.get('/test')).rejects.toMatchObject({
        type: 'http_error',
        status: 404,
        statusText: 'Not Found',
        body: { error: 'Not found' },
      })
    })

    it('throws ApiError on failed request with text body', async () => {
      mockGetItem.mockReturnValue(null)
      mockFetch.mockResolvedValue({
        ok: false,
        status: 500,
        statusText: 'Internal Server Error',
        text: async () => 'Server error',
        headers: new Headers(),
      })

      await expect(api.get('/test')).rejects.toMatchObject({
        type: 'http_error',
        status: 500,
        body: 'Server error',
      })
    })

    it('handles response text error', async () => {
      mockGetItem.mockReturnValue(null)
      mockFetch.mockResolvedValue({
        ok: false,
        status: 500,
        statusText: 'Internal Server Error',
        text: async () => {
          throw new Error('Cannot read body')
        },
        headers: new Headers(),
      })

      await expect(api.get('/test')).rejects.toMatchObject({
        type: 'http_error',
        status: 500,
        body: null,
      })
    })

    it('throws parse error on invalid JSON', async () => {
      mockGetItem.mockReturnValue(null)
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        text: async () => 'invalid json',
        headers: new Headers(),
      })

      await expect(api.get('/test')).rejects.toMatchObject({
        type: 'parse_error',
      })
    })
  })

  describe('POST', () => {
    it('makes POST request with body', async () => {
      mockGetItem.mockReturnValue('test-token')
      mockFetch.mockResolvedValue({
        ok: true,
        status: 201,
        text: async () => '{"id":"123"}',
        headers: new Headers(),
      })

      const result = await api.post('/test', { name: 'test' })

      expect(mockFetch).toHaveBeenCalledWith(
        'http://localhost:3000/api/test',
        expect.objectContaining({
          method: 'POST',
          body: '{"name":"test"}',
          headers: {
            'Content-Type': 'application/json',
            Authorization: 'Bearer test-token',
          },
        }),
      )
      expect(result).toEqual({ id: '123' })
    })

    it('makes POST request without body', async () => {
      mockGetItem.mockReturnValue(null)
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        text: async () => '{"success":true}',
        headers: new Headers(),
      })

      await api.post('/test')

      const call = mockFetch.mock.calls[0]
      expect(call[0]).toBe('http://localhost:3000/api/test')
      expect(call[1]).toMatchObject({
        method: 'POST',
      })
      expect(call[1]?.body).toBeUndefined()
    })

    it('handles FormData body', async () => {
      mockGetItem.mockReturnValue(null)
      const formData = new FormData()
      formData.append('file', 'test')

      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        text: async () => '{"uploaded":true}',
        headers: new Headers(),
      })

      await api.post('/upload', formData)

      const call = mockFetch.mock.calls[0]
      expect(call[1]?.body).toBeInstanceOf(FormData)
      expect(call[1]?.headers).not.toHaveProperty('Content-Type')
    })
  })

  describe('PATCH', () => {
    it('makes PATCH request with body', async () => {
      mockGetItem.mockReturnValue('test-token')
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        text: async () => '{"updated":true}',
        headers: new Headers(),
      })

      const result = await api.patch('/test', { status: 'active' })

      expect(mockFetch).toHaveBeenCalledWith(
        'http://localhost:3000/api/test',
        expect.objectContaining({
          method: 'PATCH',
          body: '{"status":"active"}',
        }),
      )
      expect(result).toEqual({ updated: true })
    })
  })

  describe('PUT', () => {
    it('makes PUT request with body', async () => {
      mockGetItem.mockReturnValue('test-token')
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        text: async () => '{"replaced":true}',
        headers: new Headers(),
      })

      const result = await api.put('/test', { name: 'new' })

      expect(mockFetch).toHaveBeenCalledWith(
        'http://localhost:3000/api/test',
        expect.objectContaining({
          method: 'PUT',
          body: '{"name":"new"}',
        }),
      )
      expect(result).toEqual({ replaced: true })
    })
  })

  describe('DELETE', () => {
    it('makes DELETE request', async () => {
      mockGetItem.mockReturnValue('test-token')
      mockFetch.mockResolvedValue({
        ok: true,
        status: 204,
        headers: new Headers(),
      })

      const result = await api.del('/test')

      expect(mockFetch).toHaveBeenCalledWith(
        'http://localhost:3000/api/test',
        expect.objectContaining({
          method: 'DELETE',
        }),
      )
      expect(result).toBeUndefined()
    })

    it('makes DELETE request with response', async () => {
      mockGetItem.mockReturnValue('test-token')
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        text: async () => '{"deleted":true}',
        headers: new Headers(),
      })

      const result = await api.del<{ deleted: boolean }>('/test')

      expect(result).toEqual({ deleted: true })
    })
  })

  describe('Custom config', () => {
    it('uses custom headers', async () => {
      mockGetItem.mockReturnValue(null)
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        text: async () => '{}',
        headers: new Headers(),
      })

      await api.get('/test', {
        headers: {
          'X-Custom-Header': 'value',
        },
      })

      expect(mockFetch).toHaveBeenCalledWith(
        'http://localhost:3000/api/test',
        expect.objectContaining({
          headers: {
            'Content-Type': 'application/json',
            'X-Custom-Header': 'value',
          },
        }),
      )
    })

    it('respects custom timeout', async () => {
      mockGetItem.mockReturnValue(null)

      let abortSignal: AbortSignal | null = null
      mockFetch.mockImplementation(
        (_url, options) =>
          new Promise((resolve, reject) => {
            abortSignal = options?.signal ?? null
            abortSignal?.addEventListener('abort', () => {
              reject(new DOMException('Aborted', 'AbortError'))
            })
          }),
      )

      const promise = api.get('/test', { timeout: 10 })

      await expect(promise).rejects.toMatchObject({
        type: 'timeout_error',
      })
    }, 1000)
  })

  describe('Retry logic', () => {
    it('retries on network error', async () => {
      mockGetItem.mockReturnValue(null)

      mockFetch
        .mockRejectedValueOnce(new Error('Network failed'))
        .mockRejectedValueOnce(new Error('Network failed'))
        .mockResolvedValueOnce({
          ok: true,
          status: 200,
          text: async () => '{"success":true}',
          headers: new Headers(),
        })

      const result = await api.get('/test', { retries: 2, retryDelay: 1 })

      expect(result).toEqual({ success: true })
      expect(mockFetch).toHaveBeenCalledTimes(3)
    })

    it('retries on 5xx errors', async () => {
      mockGetItem.mockReturnValue(null)

      mockFetch
        .mockResolvedValueOnce({
          ok: false,
          status: 500,
          statusText: 'Internal Server Error',
          text: async () => 'Error',
          headers: new Headers(),
        })
        .mockResolvedValueOnce({
          ok: true,
          status: 200,
          text: async () => '{"success":true}',
          headers: new Headers(),
        })

      const result = await api.get('/test', { retries: 1, retryDelay: 1 })

      expect(result).toEqual({ success: true })
      expect(mockFetch).toHaveBeenCalledTimes(2)
    })

    it('does not retry on 4xx errors', async () => {
      mockGetItem.mockReturnValue(null)

      mockFetch.mockResolvedValue({
        ok: false,
        status: 404,
        statusText: 'Not Found',
        text: async () => 'Not found',
        headers: new Headers(),
      })

      await expect(api.get('/test', { retries: 2, retryDelay: 1 })).rejects.toMatchObject({
        type: 'http_error',
        status: 404,
      })

      expect(mockFetch).toHaveBeenCalledTimes(1)
    })

    it('respects exponential backoff', async () => {
      mockGetItem.mockReturnValue(null)

      const callTimes: number[] = []
      mockFetch
        .mockImplementationOnce(() => {
          callTimes.push(Date.now())
          return Promise.reject(new Error('Failed 1'))
        })
        .mockImplementationOnce(() => {
          callTimes.push(Date.now())
          return Promise.reject(new Error('Failed 2'))
        })
        .mockImplementationOnce(() => {
          callTimes.push(Date.now())
          return Promise.resolve({
            ok: true,
            status: 200,
            text: async () => '{}',
            headers: new Headers(),
          })
        })

      await api.get('/test', { retries: 2, retryDelay: 10 })

      expect(mockFetch).toHaveBeenCalledTimes(3)
      // Verify delays are increasing (10ms, 20ms)
      if (callTimes.length === 3) {
        const delay1 = callTimes[1] - callTimes[0]
        const delay2 = callTimes[2] - callTimes[1]
        expect(delay2).toBeGreaterThan(delay1)
      }
    })
  })

  describe('Request deduplication', () => {
    it('deduplicates identical GET requests', async () => {
      mockGetItem.mockReturnValue(null)
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        text: async () => '{"data":"test"}',
        headers: new Headers(),
      })

      const [result1, result2, result3] = await Promise.all([api.get('/test'), api.get('/test'), api.get('/test')])

      expect(mockFetch).toHaveBeenCalledTimes(1)
      expect(result1).toEqual({ data: 'test' })
      expect(result2).toEqual({ data: 'test' })
      expect(result3).toEqual({ data: 'test' })
    })

    it('does not deduplicate POST requests', async () => {
      mockGetItem.mockReturnValue(null)
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        text: async () => '{"created":true}',
        headers: new Headers(),
      })

      await Promise.all([api.post('/test', { data: 'test' }), api.post('/test', { data: 'test' })])

      expect(mockFetch).toHaveBeenCalledTimes(2)
    })
  })

  describe('Request cancellation', () => {
    it('cancels request with AbortController', async () => {
      mockGetItem.mockReturnValue(null)
      const controller = new AbortController()

      mockFetch.mockImplementation(
        () =>
          new Promise((_, reject) => {
            controller.signal.addEventListener('abort', () => {
              reject(new DOMException('Aborted', 'AbortError'))
            })
          }),
      )

      const promise = api.get('/test', { signal: controller.signal })

      controller.abort()

      await expect(promise).rejects.toMatchObject({
        type: 'abort_error',
      })
    })
  })

  describe('Interceptors', () => {
    it('calls request interceptor', async () => {
      mockGetItem.mockReturnValue(null)
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        text: async () => '{}',
        headers: new Headers(),
      })

      const onRequest = vi.fn((ctx) => {
        ctx.config.headers = {
          ...ctx.config.headers,
          'X-Intercepted': 'true',
        }
        return ctx
      })

      const remove = addInterceptor({ onRequest })

      await api.get('/test')

      expect(onRequest).toHaveBeenCalled()
      expect(mockFetch).toHaveBeenCalledWith(
        'http://localhost:3000/api/test',
        expect.objectContaining({
          headers: expect.objectContaining({
            'X-Intercepted': 'true',
          }),
        }),
      )

      remove()
    })

    it('calls response interceptor', async () => {
      mockGetItem.mockReturnValue(null)
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        text: async () => '{"value":42}',
        headers: new Headers(),
      })

      const onResponse = vi.fn((ctx) => {
        return {
          ...ctx,
          data: { ...ctx.data, intercepted: true },
        }
      })

      const remove = addInterceptor({ onResponse })

      const result = await api.get('/test')

      expect(onResponse).toHaveBeenCalled()
      expect(result).toEqual({ value: 42, intercepted: true })

      remove()
    })

    it('calls error interceptor', async () => {
      mockGetItem.mockReturnValue(null)
      mockFetch.mockResolvedValue({
        ok: false,
        status: 401,
        statusText: 'Unauthorized',
        text: async () => 'Unauthorized',
        headers: new Headers(),
      })

      const onError = vi.fn((error) => error)

      const remove = addInterceptor({ onError })

      await expect(api.get('/test')).rejects.toThrow()

      expect(onError).toHaveBeenCalled()
      expect(onError).toHaveBeenCalledWith(
        expect.objectContaining({
          type: 'http_error',
          status: 401,
        }),
      )

      remove()
    })

    it('removes interceptor', async () => {
      mockGetItem.mockReturnValue(null)
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        text: async () => '{}',
        headers: new Headers(),
      })

      const onRequest = vi.fn((ctx) => ctx)

      const remove = addInterceptor({ onRequest })
      remove()

      await api.get('/test')

      expect(onRequest).not.toHaveBeenCalled()
    })
  })

  describe('Configuration', () => {
    it('applies default configuration', async () => {
      mockGetItem.mockReturnValue(null)

      configure({
        timeout: 10,
        retries: 0,
        retryDelay: 200,
      })

      let abortSignal: AbortSignal | null = null
      mockFetch.mockImplementation(
        (_url, options) =>
          new Promise((resolve, reject) => {
            abortSignal = options?.signal ?? null
            abortSignal?.addEventListener('abort', () => {
              reject(new DOMException('Aborted', 'AbortError'))
            })
          }),
      )

      const promise = api.get('/test')

      await expect(promise).rejects.toMatchObject({
        type: 'timeout_error',
      })
    }, 1000)
  })

  describe('cancelInFlightRequests', () => {
    it('clears in-flight request cache', () => {
      mockGetItem.mockReturnValue(null)

      // Verify the function exists and can be called
      expect(() => cancelInFlightRequests()).not.toThrow()
    })
  })
})
