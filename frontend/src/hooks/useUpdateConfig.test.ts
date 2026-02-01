import { renderHook, act, waitFor } from '@testing-library/react'
import { useUpdateConfig } from './useUpdateConfig'
import { mockConfig } from '@/test/fixtures'

const { mockPatch } = vi.hoisted(() => ({ mockPatch: vi.fn() }))

vi.mock('@/api', () => ({ api: { patch: mockPatch } }))

vi.mock('@/constants', async () => {
  const actual = await vi.importActual<Record<string, unknown>>('@/constants')
  return { ...actual, USE_MOCK_DATA: false }
})

describe('useUpdateConfig', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('updates config and returns it', async () => {
    mockPatch.mockResolvedValue(mockConfig)
    const { result } = renderHook(() => useUpdateConfig())

    let config: unknown
    await act(async () => {
      config = await result.current.mutate({ verbosity: 'normal' })
    })

    expect(config).toEqual(mockConfig)
    expect(mockPatch).toHaveBeenCalledWith('/config', { verbosity: 'normal' })
    expect(result.current.error).toBeNull()
    expect(result.current.loading).toBe(false)
  })

  it('sets loading to true while mutating', async () => {
    let resolve: (v: unknown) => void
    mockPatch.mockReturnValue(new Promise((r) => { resolve = r }))
    const { result } = renderHook(() => useUpdateConfig())

    let promise: Promise<unknown>
    act(() => {
      promise = result.current.mutate({ verbosity: 'verbose' })
    })

    await waitFor(() => {
      expect(result.current.loading).toBe(true)
    })

    await act(async () => {
      resolve!(mockConfig)
      await promise!
    })

    expect(result.current.loading).toBe(false)
  })

  it('sets error and throws on failure', async () => {
    mockPatch.mockRejectedValue(new Error('Update failed'))
    const { result } = renderHook(() => useUpdateConfig())

    await act(async () => {
      await expect(result.current.mutate({ verbosity: 'normal' })).rejects.toThrow('Update failed')
    })

    expect(result.current.error).toBe('Update failed')
    expect(result.current.loading).toBe(false)
  })
})
