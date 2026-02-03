import { renderHook, waitFor } from '@testing-library/react'
import { useTasks, useTask } from './useTasks'
import { mockTask } from '@/test/fixtures'

const { mockList, mockGetOne, mockGet } = vi.hoisted(() => ({
  mockList: vi.fn(),
  mockGetOne: vi.fn(),
  mockGet: vi.fn(),
}))

vi.mock('@/api', () => ({
  api: { tasks: { list: mockList, get: mockGetOne }, get: mockGet },
}))
describe('useTasks', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('useTasks', () => {
    it('fetches and returns tasks', async () => {
      mockList.mockResolvedValue({ items: [mockTask] })
      const { result } = renderHook(() => useTasks())

      await waitFor(() => {
        expect(result.current.loading).toBe(false)
      })

      expect(result.current.tasks).toEqual([mockTask])
      expect(result.current.error).toBeNull()
    })

    it('passes status filter as query param', async () => {
      mockGet.mockResolvedValue([mockTask])
      renderHook(() => useTasks('pending'))

      await waitFor(() => {
        expect(mockGet).toHaveBeenCalledWith('/tasks?status=pending')
      })
    })

    it('sets error on failure', async () => {
      mockList.mockRejectedValue(new Error('Server error'))
      const { result } = renderHook(() => useTasks())

      await waitFor(() => {
        expect(result.current.loading).toBe(false)
      })

      expect(result.current.error).toBe('Server error')
    })
  })

  describe('useTask', () => {
    it('fetches a single task by id', async () => {
      mockGetOne.mockResolvedValue(mockTask)
      const { result } = renderHook(() => useTask('task-001'))

      await waitFor(() => {
        expect(result.current.loading).toBe(false)
      })

      expect(result.current.task).toEqual(mockTask)
    })

    it('returns null when id is null', async () => {
      const { result } = renderHook(() => useTask(null))

      await waitFor(() => {
        expect(result.current.loading).toBe(false)
      })

      expect(result.current.task).toBeNull()
      expect(mockGetOne).not.toHaveBeenCalled()
    })
  })
})
