import { toolStore } from './toolStore'
import { nmSize, nmGet } from './lib'

const { mockList, mockCreate, mockDelete } = vi.hoisted(() => ({
  mockList: vi.fn(),
  mockCreate: vi.fn(),
  mockDelete: vi.fn(),
}))

vi.mock('@/api', () => ({
  api: {
    tools: {
      list: mockList,
      get: vi.fn(),
      create: mockCreate,
      update: vi.fn(),
      delete: mockDelete,
    },
  },
}))

const tool1 = { id: 't1', name: 'Grep', description: 'Search', category: 'search', parameter_schema: {}, output_schema: {}, enabled: true, is_builtin: true }
const tool2 = { id: 't2', name: 'Write', description: 'Write files', category: 'fs', parameter_schema: {}, output_schema: {}, enabled: true, is_builtin: true }

beforeEach(() => {
  vi.clearAllMocks()
  toolStore.store.setState({ items: { byId: new Map(), _array: [], _version: 0 }, loading: false, error: null })
})

describe('toolStore', () => {
  it('fetchAll populates store', async () => {
    mockList.mockResolvedValue([tool1, tool2])

    await toolStore.fetchAll()

    const state = toolStore.store.getState()
    expect(nmSize(state.items)).toBe(2)
    expect(nmGet(state.items, 't1')?.name).toBe('Grep')
  })

  it('create adds item to store', async () => {
    mockCreate.mockResolvedValue(tool1)

    const result = await toolStore.create({ name: 'Grep', description: 'Search', category: 'search' })

    expect(result).toEqual(tool1)
    expect(nmGet(toolStore.store.getState().items, 't1')).toEqual(tool1)
  })

  it('remove deletes item from store', async () => {
    mockList.mockResolvedValue([tool1, tool2])
    mockDelete.mockResolvedValue(undefined)

    await toolStore.fetchAll()
    await toolStore.remove('t1')

    expect(nmSize(toolStore.store.getState().items)).toBe(1)
    expect(nmGet(toolStore.store.getState().items, 't1')).toBeUndefined()
  })
})
