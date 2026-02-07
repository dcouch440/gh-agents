import { costStore } from './costStore'
import type { CostResponse } from '@/types/cost'

const { mockList } = vi.hoisted(() => ({
  mockList: vi.fn(),
}))

vi.mock('@/api', () => ({
  api: {
    costs: { list: mockList },
  },
}))

const mockCost: CostResponse = {
  total_spend: 0.25,
  models: [
    {
      model_id: 'claude-sonnet-4-20250514',
      total_input_tokens: 10000,
      total_output_tokens: 5000,
      total_cost_usd: 0.25,
      call_count: 15,
    },
  ],
}

beforeEach(() => {
  vi.clearAllMocks()
  costStore.store.setState({
    summary: null,
    loading: false,
    error: null,
    lastFetched: null,
  })
})

describe('costStore', () => {
  describe('fetchSummary', () => {
    it('populates summary and lastFetched', async () => {
      mockList.mockResolvedValue(mockCost)
      await costStore.fetchSummary()

      const s = costStore.store.getState()
      expect(s.summary).toEqual(mockCost)
      expect(s.loading).toBe(false)
      expect(s.error).toBeNull()
      expect(s.lastFetched).toBeGreaterThan(0)
    })

    it('sets error on failure', async () => {
      mockList.mockRejectedValue(new Error('Server error'))
      await costStore.fetchSummary()

      const s = costStore.store.getState()
      expect(s.error).toBe('Server error')
      expect(s.loading).toBe(false)
      expect(s.summary).toBeNull()
    })
  })

  describe('selectIsStale', () => {
    it('returns true when never fetched', () => {
      expect(costStore.selectIsStale(costStore.store.getState())).toBe(true)
    })

    it('returns false when recently fetched', async () => {
      mockList.mockResolvedValue(mockCost)
      await costStore.fetchSummary()

      expect(costStore.selectIsStale(costStore.store.getState())).toBe(false)
    })

    it('returns true when fetched more than 60s ago', () => {
      costStore.store.setState({ lastFetched: Date.now() - 120_000 })

      expect(costStore.selectIsStale(costStore.store.getState())).toBe(true)
    })
  })

  describe('selectors', () => {
    it('selectSummary returns null initially', () => {
      expect(costStore.selectSummary(costStore.store.getState())).toBeNull()
    })

    it('selectLoading returns false initially', () => {
      expect(costStore.selectLoading(costStore.store.getState())).toBe(false)
    })
  })
})
