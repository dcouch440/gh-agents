import { documentStore } from './documentStore'
import { nmSize, nmGet } from './lib'

const { mockList, mockCreate, mockDelete, mockSearch } = vi.hoisted(() => ({
  mockList: vi.fn(),
  mockCreate: vi.fn(),
  mockDelete: vi.fn(),
  mockSearch: vi.fn(),
}))

vi.mock('@/api', () => ({
  api: {
    documents: {
      list: mockList,
      get: vi.fn(),
      create: mockCreate,
      update: vi.fn(),
      delete: mockDelete,
      search: mockSearch,
    },
  },
}))

const doc1 = { id: 'd1', user_id: 'u1', session_id: null, title: 'README', content: '# Hello', summary: null, doc_type: 'markdown', ref_tag: null, tags: null, created_at: '2025-01-01T00:00:00Z', updated_at: '2025-01-01T00:00:00Z' }
const doc2 = { id: 'd2', user_id: 'u1', session_id: null, title: 'Notes', content: 'Some notes', summary: 'Notes summary', doc_type: 'text', ref_tag: 'notes', tags: ['dev'], created_at: '2025-01-01T00:00:00Z', updated_at: '2025-01-01T00:00:00Z' }

beforeEach(() => {
  vi.clearAllMocks()
  documentStore.store.setState({ items: { byId: new Map(), _array: [], _version: 0 }, loading: false, error: null })
})

describe('documentStore', () => {
  it('fetchAll populates store', async () => {
    mockList.mockResolvedValue({ items: [doc1, doc2] })

    await documentStore.fetchAll()

    const state = documentStore.store.getState()
    expect(nmSize(state.items)).toBe(2)
    expect(nmGet(state.items, 'd1')?.title).toBe('README')
  })

  it('create adds item to store', async () => {
    mockCreate.mockResolvedValue(doc1)

    const result = await documentStore.create({ title: 'README', content: '# Hello', doc_type: 'markdown' })

    expect(result).toEqual(doc1)
    expect(nmGet(documentStore.store.getState().items, 'd1')).toEqual(doc1)
  })

  it('remove deletes item from store', async () => {
    mockList.mockResolvedValue({ items: [doc1, doc2] })
    mockDelete.mockResolvedValue(undefined)

    await documentStore.fetchAll()
    await documentStore.remove('d1')

    expect(nmSize(documentStore.store.getState().items)).toBe(1)
    expect(nmGet(documentStore.store.getState().items, 'd1')).toBeUndefined()
  })

  it('search replaces store items with results', async () => {
    mockList.mockResolvedValue({ items: [doc1, doc2] })
    await documentStore.fetchAll()

    mockSearch.mockResolvedValue({ items: [doc2] })
    await documentStore.search('notes')

    const state = documentStore.store.getState()
    expect(nmSize(state.items)).toBe(1)
    expect(nmGet(state.items, 'd2')?.title).toBe('Notes')
  })
})
