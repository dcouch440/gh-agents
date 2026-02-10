import { agentStore } from './agentStore'
import { nmSize, nmGet, createNormalizedMap } from './lib'

const { mockList, mockGet, mockCreate, mockUpdate, mockDelete, mockGetTools, mockSetTools, mockGetContext, mockSetContext } = vi.hoisted(
  () => ({
    mockList: vi.fn(),
    mockGet: vi.fn(),
    mockCreate: vi.fn(),
    mockUpdate: vi.fn(),
    mockDelete: vi.fn(),
    mockGetTools: vi.fn(),
    mockSetTools: vi.fn(),
    mockGetContext: vi.fn(),
    mockSetContext: vi.fn(),
  }),
)

vi.mock('@/api', () => ({
  api: {
    agents: {
      list: mockList,
      get: mockGet,
      create: mockCreate,
      update: mockUpdate,
      delete: mockDelete,
      getTools: mockGetTools,
      setTools: mockSetTools,
      getContext: mockGetContext,
      setContext: mockSetContext,
    },
  },
}))

const agent1 = {
  id: 'a1',
  name: 'Agent 1',
  system_prompt: '',
  model_provider: 'anthropic',
  model_id: 'claude-3',
  model_max_tokens: 4096,
  model_temperature: 0.7,
  status: 'idle',
  output_schema_id: null,
  router_id: null,
  version: 1,
}
const agent2 = {
  id: 'a2',
  name: 'Agent 2',
  system_prompt: '',
  model_provider: 'anthropic',
  model_id: 'claude-3',
  model_max_tokens: 4096,
  model_temperature: 0.7,
  status: 'idle',
  output_schema_id: null,
  router_id: null,
  version: 1,
}
const stats = { total: 2, available: 2, max: 10 }

const tool1 = {
  id: 't1',
  name: 'Grep',
  description: 'Search',
  category: 'search',
  parameter_schema: {},
  output_schema: {},
  enabled: true,
  is_builtin: true,
}
const doc1 = {
  id: 'd1',
  name: 'Doc 1',
  doc_type: 'text',
  size_bytes: 100,
  created_at: '2024-01-01T00:00:00Z',
  updated_at: '2024-01-01T00:00:00Z',
}

beforeEach(() => {
  vi.clearAllMocks()
  agentStore.store.setState({
    items: createNormalizedMap(),
    stats: null,
    toolsByAgent: {},
    contextByAgent: {},
    loading: false,
    error: null,
  })
})

describe('agentStore', () => {
  describe('CRUD', () => {
    it('fetchAll populates agents and stats', async () => {
      mockList.mockResolvedValue({ agents: [agent1, agent2], stats })

      await agentStore.fetchAll()

      const state = agentStore.store.getState()
      expect(nmSize(state.items)).toBe(2)
      expect(nmGet(state.items, 'a1')?.name).toBe('Agent 1')
      expect(state.stats).toEqual(stats)
      expect(state.loading).toBe(false)
    })

    it('fetchAll sets error on failure', async () => {
      mockList.mockRejectedValue(new Error('Network error'))

      await agentStore.fetchAll()

      const state = agentStore.store.getState()
      expect(state.error).toBe('Network error')
      expect(state.loading).toBe(false)
    })

    it('fetchOne upserts agent', async () => {
      mockGet.mockResolvedValue(agent1)

      const result = await agentStore.fetchOne('a1')

      expect(result).toEqual(agent1)
      expect(nmGet(agentStore.store.getState().items, 'a1')).toEqual(agent1)
    })

    it('create adds agent to store', async () => {
      mockCreate.mockResolvedValue(agent1)

      const result = await agentStore.create({ name: 'Agent 1' })

      expect(result).toEqual(agent1)
      expect(nmGet(agentStore.store.getState().items, 'a1')).toEqual(agent1)
    })

    it('update replaces agent in store', async () => {
      mockList.mockResolvedValue({ agents: [agent1], stats })
      await agentStore.fetchAll()

      const updated = { ...agent1, name: 'Updated' }
      mockUpdate.mockResolvedValue(updated)

      const result = await agentStore.update('a1', { name: 'Updated' })

      expect(result.name).toBe('Updated')
      expect(nmGet(agentStore.store.getState().items, 'a1')?.name).toBe('Updated')
    })

    it('remove optimistically deletes then calls API', async () => {
      mockList.mockResolvedValue({ agents: [agent1, agent2], stats })
      mockDelete.mockResolvedValue(undefined)
      await agentStore.fetchAll()

      await agentStore.remove('a1')

      expect(nmSize(agentStore.store.getState().items)).toBe(1)
      expect(nmGet(agentStore.store.getState().items, 'a1')).toBeUndefined()
    })

    it('remove rolls back on API failure', async () => {
      mockList.mockResolvedValue({ agents: [agent1], stats })
      await agentStore.fetchAll()

      mockDelete.mockRejectedValue(new Error('Server error'))

      await expect(agentStore.remove('a1')).rejects.toThrow('Server error')
      expect(nmSize(agentStore.store.getState().items)).toBe(1)
    })
  })

  describe('sub-resources', () => {
    it('fetchTools stores tools by agent', async () => {
      mockGetTools.mockResolvedValue({ agent_id: 'a1', tools: [tool1] })

      const result = await agentStore.fetchTools('a1')

      expect(result).toEqual([tool1])
      expect(agentStore.store.getState().toolsByAgent['a1']).toEqual([tool1])
    })

    it('setTools calls API then re-fetches', async () => {
      mockSetTools.mockResolvedValue(undefined)
      mockGetTools.mockResolvedValue({ agent_id: 'a1', tools: [tool1] })

      await agentStore.setTools('a1', ['t1'])

      expect(mockSetTools).toHaveBeenCalledWith('a1', ['t1'])
      expect(mockGetTools).toHaveBeenCalledWith('a1')
    })

    it('fetchContext stores documents by agent', async () => {
      mockGetContext.mockResolvedValue({ agent_id: 'a1', documents: [doc1] })

      const result = await agentStore.fetchContext('a1')

      expect(result).toEqual([doc1])
      expect(agentStore.store.getState().contextByAgent['a1']).toEqual([doc1])
    })

    it('setContext calls API then re-fetches', async () => {
      mockSetContext.mockResolvedValue(undefined)
      mockGetContext.mockResolvedValue({ agent_id: 'a1', documents: [doc1] })

      await agentStore.setContext('a1', ['d1'])

      expect(mockSetContext).toHaveBeenCalledWith('a1', ['d1'])
      expect(mockGetContext).toHaveBeenCalledWith('a1')
    })
  })

  describe('selectors', () => {
    it('selectById returns undefined for missing agent', () => {
      const result = agentStore.selectById('missing')(agentStore.store.getState())
      expect(result).toBeUndefined()
    })

    it('selectTools returns empty array for unknown agent', () => {
      const result = agentStore.selectTools('unknown')(agentStore.store.getState())
      expect(result).toEqual([])
    })

    it('selectContext returns empty array for unknown agent', () => {
      const result = agentStore.selectContext('unknown')(agentStore.store.getState())
      expect(result).toEqual([])
    })
  })
})
