import { resultStore } from './resultStore'
import { nmSize, nmGet, createNormalizedMap } from './lib'

const { mockList, mockGet } = vi.hoisted(() => ({
  mockList: vi.fn(),
  mockGet: vi.fn(),
}))

vi.mock('@/api', () => ({
  api: {
    results: {
      list: mockList,
      get: mockGet,
    },
  },
}))

const result1 = { id: 'res1', agent_execution_id: 'exec1', output_schema_id: null, name: 'Result 1', data: { key: 'value' }, created_at: '2024-01-01T00:00:00Z' }
const result2 = { id: 'res2', agent_execution_id: 'exec1', output_schema_id: 's1', name: 'Result 2', data: {}, created_at: '2024-01-01T00:00:00Z' }

beforeEach(() => {
  vi.clearAllMocks()
  resultStore.store.setState({
    items: createNormalizedMap(),
    loading: false,
    error: null,
  })
})

describe('resultStore', () => {
  it('fetchAll populates store', async () => {
    mockList.mockResolvedValue({ items: [result1, result2] })

    await resultStore.fetchAll()

    const state = resultStore.store.getState()
    expect(nmSize(state.items)).toBe(2)
    expect(nmGet(state.items, 'res1')?.name).toBe('Result 1')
    expect(state.loading).toBe(false)
  })

  it('fetchAll sets error on failure', async () => {
    mockList.mockRejectedValue(new Error('Network error'))

    await resultStore.fetchAll()

    expect(resultStore.store.getState().error).toBe('Network error')
    expect(resultStore.store.getState().loading).toBe(false)
  })

  it('fetchOne upserts result', async () => {
    mockGet.mockResolvedValue(result1)

    const result = await resultStore.fetchOne('res1')

    expect(result).toEqual(result1)
    expect(nmGet(resultStore.store.getState().items, 'res1')).toEqual(result1)
  })

  it('selectById returns undefined for missing result', () => {
    expect(resultStore.selectById('missing')(resultStore.store.getState())).toBeUndefined()
  })
})
