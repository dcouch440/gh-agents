import { render, screen, waitFor } from '@testing-library/react'
import { StatsProvider } from './StatsContext'
import { useStatsContext } from '@/hooks/useStatsContext'
import { mockUsageSummary } from '@/test/fixtures'

// ── Mocks ────────────────────────────────────────────────────────────────────

const { mockGet } = vi.hoisted(() => ({ mockGet: vi.fn() }))

vi.mock('@/api', () => ({
  api: { get: mockGet },
}))

vi.mock('@/constants', async () => {
  const actual = await vi.importActual<Record<string, unknown>>('@/constants')
  return { ...actual, USE_MOCK_DATA: false, STATS_POLL_INTERVAL_MS: 100_000 }
})

// ── Test consumer ────────────────────────────────────────────────────────────

function TestConsumer() {
  const { stats, loading, error } = useStatsContext()
  if (loading) return <div>loading</div>
  if (error) return <div>error: {error}</div>
  return (
    <div>
      {stats.map((s) => (
        <div key={`${s.tier}-${s.model_id}`} data-testid="stat">
          {s.tier}:{s.call_count}
        </div>
      ))}
    </div>
  )
}

// ── Tests ────────────────────────────────────────────────────────────────────

describe('StatsContext', () => {
  describe('StatsProvider', () => {
    beforeEach(() => {
      vi.clearAllMocks()
      mockGet.mockResolvedValue([mockUsageSummary])
    })

    it('fetches stats on mount', async () => {
      render(
        <StatsProvider>
          <TestConsumer />
        </StatsProvider>,
      )

      expect(screen.getByText('loading')).toBeInTheDocument()

      await waitFor(() => {
        expect(screen.getByTestId('stat')).toHaveTextContent('worker:10')
      })
    })

    it('throws when hook is used outside provider', () => {
      const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
      expect(() => render(<TestConsumer />)).toThrow('useStatsContext must be used within StatsProvider')
      spy.mockRestore()
    })
  })
})
