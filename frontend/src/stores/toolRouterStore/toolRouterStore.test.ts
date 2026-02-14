import { toolRouterStore } from '.'
import { nmSize, nmGet, createNormalizedMap } from '../lib'

const {
  mockList,
  mockGet,
  mockCreate,
  mockUpdate,
  mockDelete,
  mockGetTools,
  mockSetTools,
  mockListModes,
  mockCreateMode,
  mockUpdateMode,
  mockDeleteMode,
  mockGetModeTools,
  mockSetModeTools,
} = vi.hoisted(() => ({
  mockList: vi.fn(),
  mockGet: vi.fn(),
  mockCreate: vi.fn(),
  mockUpdate: vi.fn(),
  mockDelete: vi.fn(),
  mockGetTools: vi.fn(),
  mockSetTools: vi.fn(),
  mockListModes: vi.fn(),
  mockCreateMode: vi.fn(),
  mockUpdateMode: vi.fn(),
  mockDeleteMode: vi.fn(),
  mockGetModeTools: vi.fn(),
  mockSetModeTools: vi.fn(),
}))

vi.mock('@/api', () => ({
  api: {
    toolRouters: {
      list: mockList,
      get: mockGet,
      create: mockCreate,
      update: mockUpdate,
      delete: mockDelete,
      getTools: mockGetTools,
      setTools: mockSetTools,
    },
    routerModes: {
      listByRouter: mockListModes,
      createForRouter: mockCreateMode,
      update: mockUpdateMode,
      delete: mockDeleteMode,
      getTools: mockGetModeTools,
      setTools: mockSetModeTools,
    },
  },
}))

const router1 = {
  id: 'r1',
  user_id: 'u1',
  name: 'Router 1',
  description: null,
  system_prompt: 'prompt',
  model_id: 'claude-3',
  is_active: true,
  parent_router_id: null,
  level: 0,
  created_at: '2024-01-01T00:00:00Z',
  updated_at: '2024-01-01T00:00:00Z',
}
const router2 = {
  id: 'r2',
  user_id: 'u1',
  name: 'Router 2',
  description: null,
  system_prompt: 'prompt',
  model_id: 'claude-3',
  is_active: true,
  parent_router_id: null,
  level: 0,
  created_at: '2024-01-01T00:00:00Z',
  updated_at: '2024-01-01T00:00:00Z',
}

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

const mode1 = {
  id: 'm1',
  router_id: 'r1',
  mode_key: 'default',
  display_name: 'Default',
  description: '',
  system_prompt: '',
  temperature: 0.7,
  max_tokens: 8192,
  append_to_agent_system_prompt: false,
  append_to_agent_tools: true,
  display_order: 0,
  created_at: '2024-01-01T00:00:00Z',
  updated_at: '2024-01-01T00:00:00Z',
}

beforeEach(() => {
  vi.clearAllMocks()
  toolRouterStore.store.setState({
    items: createNormalizedMap(),
    toolsByRouter: {},
    modesByRouter: {},
    toolsByMode: {},
    modeToRouter: {},
    loading: false,
    error: null,
  })
})

describe('toolRouterStore', () => {
  describe('router CRUD', () => {
    it('fetchAll populates store', async () => {
      mockList.mockResolvedValue([router1, router2])

      await toolRouterStore.fetchAll()

      const state = toolRouterStore.store.getState()
      expect(nmSize(state.items)).toBe(2)
      expect(nmGet(state.items, 'r1')?.name).toBe('Router 1')
      expect(state.loading).toBe(false)
    })

    it('fetchAll sets error on failure', async () => {
      mockList.mockRejectedValue(new Error('Network error'))

      await toolRouterStore.fetchAll()

      expect(toolRouterStore.store.getState().error).toBe('Network error')
    })

    it('create adds router to store', async () => {
      mockCreate.mockResolvedValue(router1)

      const result = await toolRouterStore.create({ name: 'Router 1', system_prompt: 'prompt', model_id: 'claude-3' })

      expect(result).toEqual(router1)
      expect(nmGet(toolRouterStore.store.getState().items, 'r1')).toEqual(router1)
    })

    it('remove optimistically deletes', async () => {
      mockList.mockResolvedValue([router1, router2])
      mockDelete.mockResolvedValue(undefined)
      await toolRouterStore.fetchAll()

      await toolRouterStore.remove('r1')

      expect(nmSize(toolRouterStore.store.getState().items)).toBe(1)
    })
  })

  describe('router tools', () => {
    it('fetchRouterTools stores tools', async () => {
      mockGetTools.mockResolvedValue([tool1])

      const result = await toolRouterStore.fetchRouterTools('r1')

      expect(result).toEqual([tool1])
      expect(toolRouterStore.store.getState().toolsByRouter['r1']).toEqual([tool1])
    })

    it('setRouterTools calls API then re-fetches', async () => {
      mockSetTools.mockResolvedValue(undefined)
      mockGetTools.mockResolvedValue([tool1])

      await toolRouterStore.setRouterTools('r1', { tool_ids: ['t1'] })

      expect(mockSetTools).toHaveBeenCalledWith('r1', { tool_ids: ['t1'] })
      expect(mockGetTools).toHaveBeenCalledWith('r1')
    })
  })

  describe('modes', () => {
    it('fetchModes stores modes by router and builds reverse lookup', async () => {
      mockListModes.mockResolvedValue([mode1])

      const result = await toolRouterStore.fetchModes('r1')

      expect(result).toEqual([mode1])
      expect(toolRouterStore.store.getState().modesByRouter['r1']).toEqual([mode1])
      expect(toolRouterStore.store.getState().modeToRouter['m1']).toBe('r1')
    })

    it('createMode appends to modesByRouter and updates reverse lookup', async () => {
      mockCreateMode.mockResolvedValue(mode1)

      const result = await toolRouterStore.createMode('r1', {
        mode_key: 'default',
        display_name: 'Default',
        description: '',
        system_prompt: '',
      })

      expect(result).toEqual(mode1)
      expect(toolRouterStore.store.getState().modesByRouter['r1']).toEqual([mode1])
      expect(toolRouterStore.store.getState().modeToRouter['m1']).toBe('r1')
    })

    it('updateMode replaces mode in list', async () => {
      // Pre-populate
      toolRouterStore.store.setState({ modesByRouter: { r1: [mode1] } })
      const updated = { ...mode1, display_name: 'Updated' }
      mockUpdateMode.mockResolvedValue(updated)

      const result = await toolRouterStore.updateMode('m1', { display_name: 'Updated' })

      expect(result.display_name).toBe('Updated')
      expect(toolRouterStore.store.getState().modesByRouter['r1']?.[0]?.display_name).toBe('Updated')
    })

    it('deleteMode removes mode from list and cleans reverse lookup', async () => {
      toolRouterStore.store.setState({ modesByRouter: { r1: [mode1] }, modeToRouter: { m1: 'r1' } })
      mockDeleteMode.mockResolvedValue(undefined)

      await toolRouterStore.deleteMode('m1')

      expect(toolRouterStore.store.getState().modesByRouter['r1']).toEqual([])
      expect(toolRouterStore.store.getState().modeToRouter['m1']).toBeUndefined()
    })
  })

  describe('mode tools', () => {
    it('fetchModeTools stores tools by mode', async () => {
      mockGetModeTools.mockResolvedValue([tool1])

      const result = await toolRouterStore.fetchModeTools('m1')

      expect(result).toEqual([tool1])
      expect(toolRouterStore.store.getState().toolsByMode['m1']).toEqual([tool1])
    })

    it('setModeTools calls API then re-fetches', async () => {
      mockSetModeTools.mockResolvedValue(undefined)
      mockGetModeTools.mockResolvedValue([tool1])

      await toolRouterStore.setModeTools('m1', { tool_ids: ['t1'] })

      expect(mockSetModeTools).toHaveBeenCalledWith('m1', { tool_ids: ['t1'] })
      expect(mockGetModeTools).toHaveBeenCalledWith('m1')
    })
  })

  describe('selectors', () => {
    it('selectModes returns empty array for unknown router', () => {
      expect(toolRouterStore.selectModes('unknown')(toolRouterStore.store.getState())).toEqual([])
    })

    it('selectModeTools returns empty array for unknown mode', () => {
      expect(toolRouterStore.selectModeTools('unknown')(toolRouterStore.store.getState())).toEqual([])
    })
  })
})
