import { renderHook, act, waitFor } from '@testing-library/react'
import { useCreateTask } from './useCreateTask'
import { mockTask } from '@/test/fixtures'

const { mockPost } = vi.hoisted(() => ({ mockPost: vi.fn() }))

vi.mock('@/api', () => ({ api: { post: mockPost } }))

describe('useCreateTask', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('creates a task and returns it', async () => {
    mockPost.mockResolvedValue(mockTask)
    const { result } = renderHook(() => useCreateTask())

    let task: unknown
    await act(async () => {
      task = await result.current.mutate({ title: 'Test task', description: 'A task for testing' })
    })

    expect(task).toEqual(mockTask)
    expect(mockPost).toHaveBeenCalledWith('/tasks', { title: 'Test task', description: 'A task for testing' })
    expect(result.current.error).toBeNull()
    expect(result.current.loading).toBe(false)
  })

  it('sets loading to true while mutating', async () => {
    let resolve: (v: unknown) => void
    mockPost.mockReturnValue(new Promise((r) => { resolve = r }))
    const { result } = renderHook(() => useCreateTask())

    let promise: Promise<unknown>
    act(() => {
      promise = result.current.mutate({ title: 'Test', description: 'desc' })
    })

    await waitFor(() => {
      expect(result.current.loading).toBe(true)
    })

    await act(async () => {
      resolve!(mockTask)
      await promise!
    })

    expect(result.current.loading).toBe(false)
  })

  it('sets error and throws on failure', async () => {
    mockPost.mockRejectedValue(new Error('Create failed'))
    const { result } = renderHook(() => useCreateTask())

    await act(async () => {
      await expect(result.current.mutate({ title: 'Bad', description: 'fail' })).rejects.toThrow('Create failed')
    })

    expect(result.current.error).toBe('Create failed')
    expect(result.current.loading).toBe(false)
  })
})
