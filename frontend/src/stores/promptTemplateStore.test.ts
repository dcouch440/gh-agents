import { promptTemplateStore } from './promptTemplateStore'
import { nmSize, nmGet } from './lib'

const { mockList, mockCreate, mockDelete } = vi.hoisted(() => ({
  mockList: vi.fn(),
  mockCreate: vi.fn(),
  mockDelete: vi.fn(),
}))

vi.mock('@/api', () => ({
  api: {
    promptTemplates: {
      list: mockList,
      get: vi.fn(),
      create: mockCreate,
      update: vi.fn(),
      delete: mockDelete,
    },
  },
}))

const tpl1 = { id: 'pt1', user_id: 'u1', name: 'Summarize', description: null, template: 'Summarize: {text}', variables: ['text'], created_at: '2025-01-01T00:00:00Z', updated_at: '2025-01-01T00:00:00Z' }
const tpl2 = { id: 'pt2', user_id: 'u1', name: 'Translate', description: 'Translate text', template: 'Translate to {lang}: {text}', variables: ['lang', 'text'], created_at: '2025-01-01T00:00:00Z', updated_at: '2025-01-01T00:00:00Z' }

beforeEach(() => {
  vi.clearAllMocks()
  promptTemplateStore.store.setState({ items: { byId: new Map(), _array: [], _version: 0 }, loading: false, error: null })
})

describe('promptTemplateStore', () => {
  it('fetchAll populates store', async () => {
    mockList.mockResolvedValue({ items: [tpl1, tpl2] })

    await promptTemplateStore.fetchAll()

    const state = promptTemplateStore.store.getState()
    expect(nmSize(state.items)).toBe(2)
    expect(nmGet(state.items, 'pt1')?.name).toBe('Summarize')
  })

  it('create adds item to store', async () => {
    mockCreate.mockResolvedValue(tpl1)

    const result = await promptTemplateStore.create({ name: 'Summarize', template: 'Summarize: {text}' })

    expect(result).toEqual(tpl1)
    expect(nmGet(promptTemplateStore.store.getState().items, 'pt1')).toEqual(tpl1)
  })

  it('remove deletes item from store', async () => {
    mockList.mockResolvedValue({ items: [tpl1, tpl2] })
    mockDelete.mockResolvedValue(undefined)

    await promptTemplateStore.fetchAll()
    await promptTemplateStore.remove('pt1')

    expect(nmSize(promptTemplateStore.store.getState().items)).toBe(1)
    expect(nmGet(promptTemplateStore.store.getState().items, 'pt1')).toBeUndefined()
  })
})
