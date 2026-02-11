import { protocolStore } from './protocolStore'
import { nmSize, nmGet, createNormalizedMap } from './lib'
import type { Protocol, ProtocolTypeInfo } from '@/types/protocol'

const { mockList, mockGet, mockCreate, mockUpdate, mockDelete, mockListTypes, mockCreatePort, mockDeletePort } = vi.hoisted(() => ({
  mockList: vi.fn(),
  mockGet: vi.fn(),
  mockCreate: vi.fn(),
  mockUpdate: vi.fn(),
  mockDelete: vi.fn(),
  mockListTypes: vi.fn(),
  mockCreatePort: vi.fn(),
  mockDeletePort: vi.fn(),
}))

vi.mock('@/api', () => ({
  api: {
    protocols: {
      list: mockList,
      get: mockGet,
      create: mockCreate,
      update: mockUpdate,
      delete: mockDelete,
      listTypes: mockListTypes,
      createPort: mockCreatePort,
      deletePort: mockDeletePort,
    },
  },
}))

const proto1: Protocol = {
  id: 'proto-1',
  name: 'Test Protocol',
  description: 'A test protocol',
  protocol_type: 'review',
  config: {},
  version: 1,
  ports: [],
  agent: null,
  output_schema: null,
  prompt_template: null,
}

const proto2: Protocol = {
  id: 'proto-2',
  name: 'Code Gen',
  description: 'Generates code',
  protocol_type: 'generation',
  config: {},
  version: 1,
  ports: [],
  agent: null,
  output_schema: null,
  prompt_template: null,
}

const proto3: Protocol = {
  id: 'proto-3',
  name: 'Review 2',
  description: 'Another review protocol',
  protocol_type: 'review',
  config: {},
  version: 1,
  ports: [],
  agent: null,
  output_schema: null,
  prompt_template: null,
}

const typeInfo: ProtocolTypeInfo[] = [
  { name: 'review', description: 'Code review protocol' },
  { name: 'generation', description: 'Code generation protocol' },
]

beforeEach(() => {
  vi.clearAllMocks()
  protocolStore.store.setState({
    items: createNormalizedMap(),
    types: [],
    loading: false,
    error: null,
  })
})

describe('protocolStore', () => {
  describe('selectors', () => {
    it('selectAll returns array from NormalizedMap', async () => {
      mockList.mockResolvedValue([proto1, proto2])
      await protocolStore.fetchAll()

      const result = protocolStore.selectAll(protocolStore.store.getState())
      expect(result).toHaveLength(2)
      expect(result).toEqual(expect.arrayContaining([proto1, proto2]))
    })

    it('selectAll returns empty array for empty store', () => {
      const result = protocolStore.selectAll(protocolStore.store.getState())
      expect(result).toEqual([])
    })

    it('selectById returns specific protocol', async () => {
      mockList.mockResolvedValue([proto1, proto2])
      await protocolStore.fetchAll()

      const result = protocolStore.selectById('proto-1')(protocolStore.store.getState())
      expect(result).toEqual(proto1)
    })

    it('selectById returns undefined for missing id', () => {
      const result = protocolStore.selectById('missing')(protocolStore.store.getState())
      expect(result).toBeUndefined()
    })

    it('selectTypes returns types array', async () => {
      mockListTypes.mockResolvedValue({ types: typeInfo })
      await protocolStore.fetchTypes()

      const result = protocolStore.selectTypes(protocolStore.store.getState())
      expect(result).toEqual(typeInfo)
    })

    it('selectTypes returns empty array when no types loaded', () => {
      const result = protocolStore.selectTypes(protocolStore.store.getState())
      expect(result).toEqual([])
    })

    it('selectByType filters protocols by protocol_type', async () => {
      mockList.mockResolvedValue([proto1, proto2, proto3])
      await protocolStore.fetchAll()

      const reviews = protocolStore.selectByType('review')(protocolStore.store.getState())
      expect(reviews).toHaveLength(2)
      expect(reviews).toEqual(expect.arrayContaining([proto1, proto3]))

      const generations = protocolStore.selectByType('generation')(protocolStore.store.getState())
      expect(generations).toHaveLength(1)
      expect(generations[0]).toEqual(proto2)
    })

    it('selectByType returns empty array for unmatched type', async () => {
      mockList.mockResolvedValue([proto1])
      await protocolStore.fetchAll()

      const result = protocolStore.selectByType('nonexistent')(protocolStore.store.getState())
      expect(result).toEqual([])
    })

    it('selectLoading returns loading state', () => {
      expect(protocolStore.selectLoading(protocolStore.store.getState())).toBe(false)
    })

    it('selectError returns error state', () => {
      expect(protocolStore.selectError(protocolStore.store.getState())).toBeNull()
    })
  })

  describe('fetchAll', () => {
    it('populates items from API response', async () => {
      mockList.mockResolvedValue([proto1, proto2])

      await protocolStore.fetchAll()

      const state = protocolStore.store.getState()
      expect(nmSize(state.items)).toBe(2)
      expect(nmGet(state.items, 'proto-1')).toEqual(proto1)
      expect(nmGet(state.items, 'proto-2')).toEqual(proto2)
      expect(state.loading).toBe(false)
      expect(state.error).toBeNull()
    })

    it('sets loading to true then false on success', async () => {
      let loadingDuringCall = false
      mockList.mockImplementation(() => {
        loadingDuringCall = protocolStore.store.getState().loading
        return Promise.resolve([proto1])
      })

      await protocolStore.fetchAll()

      expect(loadingDuringCall).toBe(true)
      expect(protocolStore.store.getState().loading).toBe(false)
    })

    it('clears previous error on new fetch', async () => {
      protocolStore.store.setState({ error: 'old error' })
      mockList.mockResolvedValue([proto1])

      await protocolStore.fetchAll()

      expect(protocolStore.store.getState().error).toBeNull()
    })

    it('sets error string on failure', async () => {
      mockList.mockRejectedValue(new Error('Network error'))

      await protocolStore.fetchAll()

      const state = protocolStore.store.getState()
      expect(state.error).toBe('Network error')
      expect(state.loading).toBe(false)
    })

    it('sets fallback error for non-Error rejection', async () => {
      mockList.mockRejectedValue('something went wrong')

      await protocolStore.fetchAll()

      const state = protocolStore.store.getState()
      expect(state.error).toBe('protocols: unknown error')
      expect(state.loading).toBe(false)
    })
  })

  describe('fetchOne', () => {
    it('puts returned protocol into store', async () => {
      mockGet.mockResolvedValue(proto1)

      const result = await protocolStore.fetchOne('proto-1')

      expect(result).toEqual(proto1)
      expect(nmGet(protocolStore.store.getState().items, 'proto-1')).toEqual(proto1)
    })

    it('upserts existing protocol', async () => {
      mockList.mockResolvedValue([proto1])
      await protocolStore.fetchAll()

      const updated = { ...proto1, name: 'Updated Protocol' }
      mockGet.mockResolvedValue(updated)

      await protocolStore.fetchOne('proto-1')

      expect(nmGet(protocolStore.store.getState().items, 'proto-1')?.name).toBe('Updated Protocol')
      expect(nmSize(protocolStore.store.getState().items)).toBe(1)
    })
  })

  describe('fetchTypes', () => {
    it('stores types array from API', async () => {
      mockListTypes.mockResolvedValue({ types: typeInfo })

      await protocolStore.fetchTypes()

      expect(protocolStore.store.getState().types).toEqual(typeInfo)
    })

    it('sets error on failure', async () => {
      mockListTypes.mockRejectedValue(new Error('Types fetch failed'))

      await protocolStore.fetchTypes()

      expect(protocolStore.store.getState().error).toBe('Types fetch failed')
    })

    it('sets fallback error for non-Error rejection', async () => {
      mockListTypes.mockRejectedValue(42)

      await protocolStore.fetchTypes()

      expect(protocolStore.store.getState().error).toBe('protocols: unknown error')
    })
  })

  describe('create', () => {
    it('adds protocol to store and returns it', async () => {
      mockCreate.mockResolvedValue(proto1)

      const result = await protocolStore.create({
        name: 'Test Protocol',
        protocol_type: 'review',
      })

      expect(result).toEqual(proto1)
      expect(nmGet(protocolStore.store.getState().items, 'proto-1')).toEqual(proto1)
    })

    it('preserves existing items when adding', async () => {
      mockList.mockResolvedValue([proto1])
      await protocolStore.fetchAll()

      mockCreate.mockResolvedValue(proto2)
      await protocolStore.create({ name: 'Code Gen', protocol_type: 'generation' })

      expect(nmSize(protocolStore.store.getState().items)).toBe(2)
      expect(nmGet(protocolStore.store.getState().items, 'proto-1')).toEqual(proto1)
      expect(nmGet(protocolStore.store.getState().items, 'proto-2')).toEqual(proto2)
    })
  })

  describe('update', () => {
    it('updates existing protocol in store', async () => {
      mockList.mockResolvedValue([proto1])
      await protocolStore.fetchAll()

      const updated = { ...proto1, name: 'Updated', version: 2 }
      mockUpdate.mockResolvedValue(updated)

      const result = await protocolStore.update('proto-1', { name: 'Updated' })

      expect(result.name).toBe('Updated')
      expect(result.version).toBe(2)
      expect(nmGet(protocolStore.store.getState().items, 'proto-1')?.name).toBe('Updated')
    })
  })

  describe('remove', () => {
    it('optimistically deletes from store', async () => {
      mockList.mockResolvedValue([proto1, proto2])
      mockDelete.mockResolvedValue(undefined)
      await protocolStore.fetchAll()

      await protocolStore.remove('proto-1')

      expect(nmSize(protocolStore.store.getState().items)).toBe(1)
      expect(nmGet(protocolStore.store.getState().items, 'proto-1')).toBeUndefined()
      expect(nmGet(protocolStore.store.getState().items, 'proto-2')).toEqual(proto2)
    })

    it('rolls back on API failure', async () => {
      mockList.mockResolvedValue([proto1])
      await protocolStore.fetchAll()

      mockDelete.mockRejectedValue(new Error('Server error'))

      await expect(protocolStore.remove('proto-1')).rejects.toThrow('Server error')

      expect(nmSize(protocolStore.store.getState().items)).toBe(1)
      expect(nmGet(protocolStore.store.getState().items, 'proto-1')).toEqual(proto1)
    })

    it('sets error on API failure', async () => {
      mockList.mockResolvedValue([proto1])
      await protocolStore.fetchAll()

      mockDelete.mockRejectedValue(new Error('Delete failed'))

      await expect(protocolStore.remove('proto-1')).rejects.toThrow('Delete failed')

      expect(protocolStore.store.getState().error).toBe('Delete failed')
    })
  })

  describe('sub-resources: ports', () => {
    it('createPort calls API then refetches protocol', async () => {
      const updatedProto = { ...proto1, ports: [{ id: 'port-1', port_name: 'input', description: '', agent_id: 'a1', display_order: 0 }] }
      mockCreatePort.mockResolvedValue({ id: 'port-1', port_name: 'input', description: '', agent_id: 'a1', display_order: 0 })
      mockGet.mockResolvedValue(updatedProto)

      await protocolStore.createPort('proto-1', { port_name: 'input', agent_id: 'a1' })

      expect(mockCreatePort).toHaveBeenCalledWith('proto-1', { port_name: 'input', agent_id: 'a1' })
      expect(mockGet).toHaveBeenCalledWith('proto-1')
      expect(nmGet(protocolStore.store.getState().items, 'proto-1')).toEqual(updatedProto)
    })

    it('deletePort calls API then refetches protocol', async () => {
      mockList.mockResolvedValue([proto1])
      await protocolStore.fetchAll()

      const updatedProto = { ...proto1, ports: [] }
      mockDeletePort.mockResolvedValue(undefined)
      mockGet.mockResolvedValue(updatedProto)

      await protocolStore.deletePort('proto-1', 'port-1')

      expect(mockDeletePort).toHaveBeenCalledWith('proto-1', 'port-1')
      expect(mockGet).toHaveBeenCalledWith('proto-1')
      expect(nmGet(protocolStore.store.getState().items, 'proto-1')).toEqual(updatedProto)
    })
  })
})
