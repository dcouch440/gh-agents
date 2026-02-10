import { taskStore } from './taskStore'
import { nmSize, nmGet } from './lib'

const { mockList, mockCreate, mockDelete } = vi.hoisted(() => ({
  mockList: vi.fn(),
  mockCreate: vi.fn(),
  mockDelete: vi.fn(),
}))

vi.mock('@/api', () => ({
  api: {
    tasks: {
      list: mockList,
      get: vi.fn(),
      create: mockCreate,
      update: vi.fn(),
      delete: mockDelete,
    },
  },
}))

const task1 = {
  id: 'tk1',
  slice_id: null,
  title: 'Fix bug',
  description: 'Fix the login bug',
  assigned_agent: null,
  status: 'pending' as const,
  priority: 'normal' as const,
  context_files: [],
  metadata: null,
  depends_on: [],
  retry_count: 0,
  max_retries: 3,
  last_error: null,
  created_at: '2025-01-01T00:00:00Z',
  updated_at: '2025-01-01T00:00:00Z',
}
const task2 = {
  id: 'tk2',
  slice_id: null,
  title: 'Add feature',
  description: 'Add dark mode',
  assigned_agent: 'a1',
  status: 'in_progress' as const,
  priority: 'high' as const,
  context_files: [],
  metadata: null,
  depends_on: [],
  retry_count: 0,
  max_retries: 3,
  last_error: null,
  created_at: '2025-01-01T00:00:00Z',
  updated_at: '2025-01-01T00:00:00Z',
}

beforeEach(() => {
  vi.clearAllMocks()
  taskStore.store.setState({ items: { byId: new Map(), _array: [], _version: 0 }, loading: false, error: null })
})

describe('taskStore', () => {
  it('fetchAll populates store', async () => {
    mockList.mockResolvedValue([task1, task2])

    await taskStore.fetchAll()

    const state = taskStore.store.getState()
    expect(nmSize(state.items)).toBe(2)
    expect(nmGet(state.items, 'tk1')?.title).toBe('Fix bug')
  })

  it('create adds item to store', async () => {
    mockCreate.mockResolvedValue(task1)

    const result = await taskStore.create({ title: 'Fix bug' })

    expect(result).toEqual(task1)
    expect(nmGet(taskStore.store.getState().items, 'tk1')).toEqual(task1)
  })

  it('remove deletes item from store', async () => {
    mockList.mockResolvedValue([task1, task2])
    mockDelete.mockResolvedValue(undefined)

    await taskStore.fetchAll()
    await taskStore.remove('tk1')

    expect(nmSize(taskStore.store.getState().items)).toBe(1)
    expect(nmGet(taskStore.store.getState().items, 'tk1')).toBeUndefined()
  })
})
