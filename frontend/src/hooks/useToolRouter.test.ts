import { renderHook, waitFor } from '@testing-library/react'
import { useToolRouter } from './useToolRouter'
import { mockToolRouter } from '@/test/fixtures'

const { mockGet } = vi.hoisted(() => ({
  mockGet: vi.fn(),
}))

vi.mock('@/api', () => ({
  api: {
    toolRouters: {
      get: mockGet,
    },
  },
}))

describe('useToolRouter', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('returns null router when id is null', async () => {
    const { result } = renderHook(() => useToolRouter(null))

    await waitFor(() => {
      expect(result.current.loading).toBe(false)
    })

    expect(result.current.router).toBeNull()
    expect(result.current.error).toBeNull()
    expect(mockGet).not.toHaveBeenCalled()
  })

  it('fetches router when id is provided', async () => {
    mockGet.mockResolvedValue(mockToolRouter)
    const { result } = renderHook(() => useToolRouter('router-001'))

    await waitFor(() => {
      expect(result.current.loading).toBe(false)
    })

    expect(result.current.router).toEqual(mockToolRouter)
    expect(result.current.error).toBeNull()
    expect(mockGet).toHaveBeenCalledWith('router-001')
  })

  it('handles fetch error', async () => {
    mockGet.mockRejectedValue(new Error('Not found'))
    const { result } = renderHook(() => useToolRouter('router-001'))

    await waitFor(() => {
      expect(result.current.loading).toBe(false)
    })

    expect(result.current.router).toBeNull()
    expect(result.current.error).toBe('Not found')
  })

  it('reloads when reload is called', async () => {
    mockGet.mockResolvedValue(mockToolRouter)
    const { result } = renderHook(() => useToolRouter('router-001'))

    await waitFor(() => {
      expect(result.current.loading).toBe(false)
    })

    const updated = { ...mockToolRouter, name: 'Updated' }
    mockGet.mockResolvedValue(updated)

    await result.current.reload()

    await waitFor(() => {
      expect(result.current.router).toEqual(updated)
    })

    expect(mockGet).toHaveBeenCalledTimes(2)
  })
})
