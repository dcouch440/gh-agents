import { configStore } from './configStore'
import type { Config } from '@/types/config'
import type { UsageSummary } from '@/types/stats'

const { mockConfigGet, mockConfigUpdate, mockStatsGet } = vi.hoisted(() => ({
  mockConfigGet: vi.fn(),
  mockConfigUpdate: vi.fn(),
  mockStatsGet: vi.fn(),
}))

vi.mock('@/api', () => ({
  api: {
    config: { get: mockConfigGet, update: mockConfigUpdate },
    stats: { get: mockStatsGet },
  },
}))

const mockConfig: Config = {
  verbosity: 'normal',
  models: {
    orchestrator: { provider: 'anthropic', model_id: 'claude-opus-4-5-20251101', max_tokens: 16384, temperature: 0.7 },
    worker: { provider: 'anthropic', model_id: 'claude-sonnet-4-20250514', max_tokens: 8192, temperature: 0.7 },
    utility: { provider: 'anthropic', model_id: 'claude-3-5-haiku-20241022', max_tokens: 4096, temperature: 0.7 },
  },
  pool: { max_agents: 12 },
  autonomy: 'supervised',
  git_strategy: 'branch',
  sandbox_mode: 'docker',
}

const mockStats: UsageSummary = {
  model_id: 'claude-sonnet-4-20250514',
  total_input: 5000,
  total_output: 2000,
  call_count: 10,
}

beforeEach(() => {
  vi.clearAllMocks()
  configStore.store.setState({
    config: null,
    stats: null,
    loading: false,
    error: null,
  })
})

describe('configStore', () => {
  describe('fetchConfig', () => {
    it('populates config', async () => {
      mockConfigGet.mockResolvedValue(mockConfig)
      await configStore.fetchConfig()

      const s = configStore.store.getState()
      expect(s.config).toEqual(mockConfig)
      expect(s.loading).toBe(false)
      expect(s.error).toBeNull()
    })

    it('sets error on failure', async () => {
      mockConfigGet.mockRejectedValue(new Error('Forbidden'))
      await configStore.fetchConfig()

      const s = configStore.store.getState()
      expect(s.error).toBe('Forbidden')
      expect(s.loading).toBe(false)
    })
  })

  describe('updateConfig', () => {
    it('updates and stores new config', async () => {
      const updated = { ...mockConfig, autonomy: 'autonomous' }
      mockConfigUpdate.mockResolvedValue(updated)
      await configStore.updateConfig({ autonomy: 'autonomous' })

      const s = configStore.store.getState()
      expect(s.config).toEqual(updated)
      expect(s.loading).toBe(false)
    })

    it('sets error on failure', async () => {
      mockConfigUpdate.mockRejectedValue(new Error('Validation error'))
      await configStore.updateConfig({ autonomy: 'invalid' })

      expect(configStore.store.getState().error).toBe('Validation error')
    })
  })

  describe('fetchStats', () => {
    it('stores stats as array', async () => {
      mockStatsGet.mockResolvedValue(mockStats)
      await configStore.fetchStats()

      expect(configStore.store.getState().stats).toEqual([mockStats])
    })

    it('handles array response', async () => {
      mockStatsGet.mockResolvedValue([mockStats])
      await configStore.fetchStats()

      expect(configStore.store.getState().stats).toEqual([mockStats])
    })

    it('sets error on failure', async () => {
      mockStatsGet.mockRejectedValue(new Error('Stats unavailable'))
      await configStore.fetchStats()

      expect(configStore.store.getState().error).toBe('Stats unavailable')
    })
  })

  describe('selectors', () => {
    it('selectConfig returns null initially', () => {
      expect(configStore.selectConfig(configStore.store.getState())).toBeNull()
    })

    it('selectStats returns null initially', () => {
      expect(configStore.selectStats(configStore.store.getState())).toBeNull()
    })
  })
})
