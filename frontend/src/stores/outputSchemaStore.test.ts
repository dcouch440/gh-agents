import { outputSchemaStore } from './outputSchemaStore'
import { nmSize, nmGet } from './lib'

const { mockList, mockCreate, mockDelete } = vi.hoisted(() => ({
  mockList: vi.fn(),
  mockCreate: vi.fn(),
  mockDelete: vi.fn(),
}))

vi.mock('@/api', () => ({
  api: {
    outputSchemas: {
      list: mockList,
      get: vi.fn(),
      create: mockCreate,
      update: vi.fn(),
      delete: mockDelete,
    },
  },
}))

const schema1 = { id: 's1', name: 'JSON Output', schema: { type: 'object' }, created_at: '2025-01-01T00:00:00Z' }
const schema2 = { id: 's2', name: 'CSV Output', schema: { type: 'array' }, created_at: '2025-01-01T00:00:00Z' }

beforeEach(() => {
  vi.clearAllMocks()
  outputSchemaStore.store.setState({ items: { byId: new Map(), _array: [], _version: 0 }, loading: false, error: null })
})

describe('outputSchemaStore', () => {
  it('fetchAll populates store', async () => {
    mockList.mockResolvedValue({ items: [schema1, schema2] })

    await outputSchemaStore.fetchAll()

    const state = outputSchemaStore.store.getState()
    expect(nmSize(state.items)).toBe(2)
    expect(nmGet(state.items, 's1')?.name).toBe('JSON Output')
  })

  it('create adds item to store', async () => {
    mockCreate.mockResolvedValue(schema1)

    const result = await outputSchemaStore.create({ name: 'JSON Output', schema: { type: 'object' } })

    expect(result).toEqual(schema1)
    expect(nmGet(outputSchemaStore.store.getState().items, 's1')).toEqual(schema1)
  })

  it('remove deletes item from store', async () => {
    mockList.mockResolvedValue({ items: [schema1, schema2] })
    mockDelete.mockResolvedValue(undefined)

    await outputSchemaStore.fetchAll()
    await outputSchemaStore.remove('s1')

    expect(nmSize(outputSchemaStore.store.getState().items)).toBe(1)
    expect(nmGet(outputSchemaStore.store.getState().items, 's1')).toBeUndefined()
  })
})
