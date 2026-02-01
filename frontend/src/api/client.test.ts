import { describe, it, expect, beforeEach, vi } from 'vitest'
import { api, ApiError } from './client'

const { mockFetch, mockGetItem, mockSetItem } = vi.hoisted(() => ({
  mockFetch: vi.fn(),
  mockGetItem: vi.fn(),
  mockSetItem: vi.fn(),
}))

vi.stubGlobal('fetch', mockFetch)
vi.stubGlobal('localStorage', {
  getItem: mockGetItem,
  setItem: mockSetItem,
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
  it('creates error with status, statusText, and body', () => {
    const error = new ApiError(404, 'Not Found', 'Resource not found')
    expect(error.status).toBe(404)
    expect(error.statusText).toBe('Not Found')
    expect(error.body).toBe('Resource not found')
    expect(error.message).toBe('404 Not Found')
    expect(error.name).toBe('ApiError')
  })
})

describe('api client', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('GET', () => {
    it('makes GET request without token', async () => {
      mockGetItem.mockReturnValue(null)
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: async () => ({ data: 'test' }),
      })

      const result = await api.get('/test')

      expect(mockFetch).toHaveBeenCalledWith('http://localhost:3000/api/test', {
        headers: { 'Content-Type': 'application/json' },
      })
      expect(result).toEqual({ data: 'test' })
    })

    it('makes GET request with token', async () => {
      mockGetItem.mockReturnValue('test-token')
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: async () => ({ data: 'test' }),
      })

      await api.get('/test')

      expect(mockFetch).toHaveBeenCalledWith('http://localhost:3000/api/test', {
        headers: {
          'Content-Type': 'application/json',
          Authorization: 'Bearer test-token',
        },
      })
    })

    it('handles 204 No Content', async () => {
      mockGetItem.mockReturnValue(null)
      mockFetch.mockResolvedValue({
        ok: true,
        status: 204,
      })

      const result = await api.get('/test')

      expect(result).toBeUndefined()
    })

    it('throws ApiError on failed request', async () => {
      mockGetItem.mockReturnValue(null)
      mockFetch.mockResolvedValue({
        ok: false,
        status: 404,
        statusText: 'Not Found',
        text: async () => 'Resource not found',
      })

      await expect(api.get('/test')).rejects.toThrow(ApiError)
      await expect(api.get('/test')).rejects.toMatchObject({
        status: 404,
        statusText: 'Not Found',
        body: 'Resource not found',
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
      })

      await expect(api.get('/test')).rejects.toMatchObject({
        status: 500,
        statusText: 'Internal Server Error',
        body: null,
      })
    })
  })

  describe('POST', () => {
    it('makes POST request with body', async () => {
      mockGetItem.mockReturnValue('test-token')
      mockFetch.mockResolvedValue({
        ok: true,
        status: 201,
        json: async () => ({ id: '123' }),
      })

      const result = await api.post('/test', { name: 'test' })

      expect(mockFetch).toHaveBeenCalledWith('http://localhost:3000/api/test', {
        method: 'POST',
        body: '{"name":"test"}',
        headers: {
          'Content-Type': 'application/json',
          Authorization: 'Bearer test-token',
        },
      })
      expect(result).toEqual({ id: '123' })
    })

    it('makes POST request without body', async () => {
      mockGetItem.mockReturnValue(null)
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: async () => ({ success: true }),
      })

      await api.post('/test')

      expect(mockFetch).toHaveBeenCalledWith('http://localhost:3000/api/test', {
        method: 'POST',
        body: undefined,
        headers: { 'Content-Type': 'application/json' },
      })
    })
  })

  describe('PATCH', () => {
    it('makes PATCH request with body', async () => {
      mockGetItem.mockReturnValue('test-token')
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: async () => ({ updated: true }),
      })

      const result = await api.patch('/test', { status: 'active' })

      expect(mockFetch).toHaveBeenCalledWith('http://localhost:3000/api/test', {
        method: 'PATCH',
        body: '{"status":"active"}',
        headers: {
          'Content-Type': 'application/json',
          Authorization: 'Bearer test-token',
        },
      })
      expect(result).toEqual({ updated: true })
    })
  })

  describe('PUT', () => {
    it('makes PUT request with body', async () => {
      mockGetItem.mockReturnValue('test-token')
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: async () => ({ replaced: true }),
      })

      const result = await api.put('/test', { name: 'new' })

      expect(mockFetch).toHaveBeenCalledWith('http://localhost:3000/api/test', {
        method: 'PUT',
        body: '{"name":"new"}',
        headers: {
          'Content-Type': 'application/json',
          Authorization: 'Bearer test-token',
        },
      })
      expect(result).toEqual({ replaced: true })
    })
  })

  describe('DELETE', () => {
    it('makes DELETE request', async () => {
      mockGetItem.mockReturnValue('test-token')
      mockFetch.mockResolvedValue({
        ok: true,
        status: 204,
      })

      const result = await api.del('/test')

      expect(mockFetch).toHaveBeenCalledWith('http://localhost:3000/api/test', {
        method: 'DELETE',
        headers: {
          'Content-Type': 'application/json',
          Authorization: 'Bearer test-token',
        },
      })
      expect(result).toBeUndefined()
    })

    it('makes DELETE request with response', async () => {
      mockGetItem.mockReturnValue('test-token')
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: async () => ({ deleted: true }),
      })

      const result = await api.del<{ deleted: boolean }>('/test')

      expect(result).toEqual({ deleted: true })
    })
  })
})
