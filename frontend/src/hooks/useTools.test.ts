import { renderHook, waitFor } from '@testing-library/react'
import { useTools } from './useTools'
import { mockTool } from '@/test/fixtures'

const { mockGet } = vi.hoisted(() => ({ mockGet: vi.fn() }))

vi.mock('@/api', () => ({ api: { get: mockGet } }))
vi.mock('@/constants', async () => {
  const actual = await vi.importActual<Record<string, unknown>>('@/constants')
  return { ...actual, USE_MOCK_DATA: false }
})

describe('useTools', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('fetches and returns tools', async () => {
    mockGet.mockResolvedValue([mockTool])
    const { result } = renderHook(() => useTools())

    await waitFor(() => {
      expect(result.current.loading).toBe(false)
    })

    expect(result.current.tools).toEqual([mockTool])
    expect(result.current.error).toBeNull()
  })

  it('sets error on failure', async () => {
    mockGet.mockRejectedValue(new Error('Failed'))
    const { result } = renderHook(() => useTools())

    await waitFor(() => {
      expect(result.current.loading).toBe(false)
    })

    expect(result.current.error).toBe('Failed')
  })
})
