import { renderHook, waitFor, act } from '@testing-library/react'
import { useConfig } from './useConfig'
import { mockConfig } from '@/test/fixtures'

const { mockGet, mockUpdate } = vi.hoisted(() => ({ mockGet: vi.fn(), mockUpdate: vi.fn() }))

vi.mock('@/api', () => ({ api: { config: { get: mockGet, update: mockUpdate } } }))
describe('useConfig', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('fetches and returns config', async () => {
    mockGet.mockResolvedValue(mockConfig)
    const { result } = renderHook(() => useConfig())

    await waitFor(() => {
      expect(result.current.loading).toBe(false)
    })

    expect(result.current.config).toEqual(mockConfig)
    expect(result.current.error).toBeNull()
  })

  it('updates config via patch', async () => {
    mockGet.mockResolvedValue(mockConfig)
    const updated = { ...mockConfig, verbosity: 'verbose' }
    mockUpdate.mockResolvedValue(updated)

    const { result } = renderHook(() => useConfig())

    await waitFor(() => {
      expect(result.current.loading).toBe(false)
    })

    await act(async () => {
      await result.current.updateConfig({ verbosity: 'verbose' })
    })

    expect(mockUpdate).toHaveBeenCalledWith({ verbosity: 'verbose' })
    expect(result.current.config).toEqual(updated)
  })

  it('sets error on failure', async () => {
    mockGet.mockRejectedValue(new Error('Failed'))
    const { result } = renderHook(() => useConfig())

    await waitFor(() => {
      expect(result.current.loading).toBe(false)
    })

    expect(result.current.error).toBe('Failed')
  })
})
