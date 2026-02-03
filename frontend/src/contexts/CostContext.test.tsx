import { render, screen, waitFor } from '@testing-library/react'
import { CostProvider } from './CostContext'
import { useCostContext } from '@/hooks/useCostContext'
import { mockCostResponse } from '@/test/fixtures'

// ── Mocks ────────────────────────────────────────────────────────────────────

const { mockList } = vi.hoisted(() => ({ mockList: vi.fn() }))

vi.mock('@/api', () => ({
  api: { costs: { list: mockList } },
}))

// ── Test consumer ────────────────────────────────────────────────────────────

function TestConsumer() {
  const { costs, loading, error } = useCostContext()
  if (loading) return <div>loading</div>
  if (error) return <div>error: {error}</div>
  if (!costs) return <div>no data</div>
  return (
    <div>
      <div data-testid="total">{costs.total_spend}</div>
      {costs.models.map((r) => (
        <div key={r.model_id} data-testid="row">
          {r.model_id}:{r.total_cost_usd}
        </div>
      ))}
    </div>
  )
}

// ── Tests ────────────────────────────────────────────────────────────────────

describe('CostContext', () => {
  describe('CostProvider', () => {
    beforeEach(() => {
      vi.clearAllMocks()
      mockList.mockResolvedValue(mockCostResponse)
    })

    it('fetches costs on mount', async () => {
      render(
        <CostProvider>
          <TestConsumer />
        </CostProvider>,
      )

      expect(screen.getByText('loading')).toBeInTheDocument()

      await waitFor(() => {
        expect(screen.getByTestId('total')).toHaveTextContent('0.15')
      })

      expect(screen.getByTestId('row')).toHaveTextContent('claude-sonnet-4-20250514:0.15')
    })

    it('throws when hook is used outside provider', () => {
      const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
      expect(() => render(<TestConsumer />)).toThrow('useCostContext must be used within CostProvider')
      spy.mockRestore()
    })
  })
})
