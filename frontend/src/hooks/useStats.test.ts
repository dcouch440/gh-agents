import { renderHook, waitFor } from '@testing-library/react'
import { useStats } from './useStats'
import { mockUsageSummary } from '@/test/fixtures'

const { mockGet } = vi.hoisted(() => ({ mockGet: vi.fn() }))

vi.mock('@/api', () => ({ api: { get: mockGet } }))
vi.mock('@/constants', async () => {
  const actual = await vi.importActual<Record<string, unknown>>('@/constants')
  return { ...actual, USE_MOCK_DATA: false, STATS_POLL_INTERVAL_MS: 100_000 }
})

describe('useStats', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('fetches and returns stats', async () => {
    mockGet.mockResolvedValue([mockUsageSummary])
    const { result } = renderHook(() => useStats())

    await waitFor(() => {
      expect(result.current.loading).toBe(false)
    })

    expect(result.current.stats).toEqual([mockUsageSummary])
    expect(result.current.error).toBeNull()
  })

  it('sets error on failure', async () => {
    mockGet.mockRejectedValue(new Error('Failed'))
    const { result } = renderHook(() => useStats())

    await waitFor(() => {
      expect(result.current.loading).toBe(false)
    })

    expect(result.current.error).toBe('Failed')
  })
})
