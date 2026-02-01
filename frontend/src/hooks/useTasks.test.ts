import { renderHook, waitFor } from '@testing-library/react'
import { useTasks, useTask } from './useTasks'
import { mockTask } from '@/test/fixtures'

const { mockGet } = vi.hoisted(() => ({ mockGet: vi.fn() }))

vi.mock('@/api', () => ({ api: { get: mockGet } }))
vi.mock('@/constants', async () => {
  const actual = await vi.importActual<Record<string, unknown>>('@/constants')
  return { ...actual, USE_MOCK_DATA: false }
})

describe('useTasks', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('useTasks', () => {
    it('fetches and returns tasks', async () => {
      mockGet.mockResolvedValue([mockTask])
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
      mockGet.mockRejectedValue(new Error('Server error'))
      const { result } = renderHook(() => useTasks())

      await waitFor(() => {
        expect(result.current.loading).toBe(false)
      })

      expect(result.current.error).toBe('Server error')
    })
  })

  describe('useTask', () => {
    it('fetches a single task by id', async () => {
      mockGet.mockResolvedValue(mockTask)
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
      expect(mockGet).not.toHaveBeenCalled()
    })
  })
})
