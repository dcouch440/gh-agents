import { renderHook, act, waitFor } from '@testing-library/react'
import { useApproveGate, useRenderStage, useSideTasks, useCreatePipeline, useDeletePipeline } from './usePipelineMutations'
import { mockTask, mockPipeline } from '@/test/fixtures'

const { mockPost, mockGet, mockDel } = vi.hoisted(() => ({
  mockPost: vi.fn(), mockGet: vi.fn(), mockDel: vi.fn(),
}))

const mockReload = vi.fn()

vi.mock('@/api', () => ({ api: { post: mockPost, get: mockGet, del: mockDel } }))
vi.mock('@/constants', async () => {
  const actual = await vi.importActual<Record<string, unknown>>('@/constants')
  return { ...actual, USE_MOCK_DATA: false }
})
vi.mock('@/hooks/usePipelineContext', () => ({
  usePipelineContext: () => ({ reload: mockReload }),
}))

describe('usePipelineMutations', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('useApproveGate', () => {
    it('approves a gate successfully', async () => {
      mockPost.mockResolvedValue(undefined)
      const { result } = renderHook(() => useApproveGate())

      await act(async () => {
        await result.current.mutate('run-001', { user_input: 'Looks good' })
      })

      expect(mockPost).toHaveBeenCalledWith('/pipeline-runs/run-001/approve', { user_input: 'Looks good' })
      expect(result.current.loading).toBe(false)
      expect(result.current.error).toBe(null)
    })

    it('approves a gate without body', async () => {
      mockPost.mockResolvedValue(undefined)
      const { result } = renderHook(() => useApproveGate())

      await act(async () => {
        await result.current.mutate('run-001')
      })

      expect(mockPost).toHaveBeenCalledWith('/pipeline-runs/run-001/approve', undefined)
    })

    it('sets error and throws on failure', async () => {
      mockPost.mockRejectedValue(new Error('Unauthorized'))
      const { result } = renderHook(() => useApproveGate())

      let caught: unknown
      await act(async () => {
        try {
          await result.current.mutate('run-001')
        } catch (e) {
          caught = e
        }
      })

      expect(caught).toBeInstanceOf(Error)
      expect((caught as Error).message).toBe('Unauthorized')
      expect(result.current.error).toBe('Unauthorized')
      expect(result.current.loading).toBe(false)
    })
  })

  describe('useRenderStage', () => {
    it('renders a stage and returns the prompt', async () => {
      const response = { rendered_prompt: 'Do the thing' }
      mockPost.mockResolvedValue(response)
      const { result } = renderHook(() => useRenderStage())

      let data: unknown
      await act(async () => {
        data = await result.current.mutate('pipeline-001', 2)
      })

      expect(data).toEqual(response)
      expect(mockPost).toHaveBeenCalledWith('/pipelines/pipeline-001/stages/2/render')
      expect(result.current.loading).toBe(false)
      expect(result.current.error).toBe(null)
    })

    it('sets error and throws on failure', async () => {
      mockPost.mockRejectedValue(new Error('Stage not found'))
      const { result } = renderHook(() => useRenderStage())

      let caught: unknown
      await act(async () => {
        try {
          await result.current.mutate('pipeline-001', 2)
        } catch (e) {
          caught = e
        }
      })

      expect(caught).toBeInstanceOf(Error)
      expect((caught as Error).message).toBe('Stage not found')
      expect(result.current.error).toBe('Stage not found')
      expect(result.current.loading).toBe(false)
    })
  })

  describe('useSideTasks', () => {
    describe('load', () => {
      it('loads side tasks and sets state', async () => {
        mockGet.mockResolvedValue([mockTask])
        const { result } = renderHook(() => useSideTasks())

        let tasks: unknown
        await act(async () => {
          tasks = await result.current.load('pipeline-001', 1)
        })

        expect(tasks).toEqual([mockTask])
        expect(result.current.tasks).toEqual([mockTask])
        expect(mockGet).toHaveBeenCalledWith('/pipelines/pipeline-001/stages/1/side-tasks')
        expect(result.current.loading).toBe(false)
        expect(result.current.error).toBe(null)
      })

      it('sets error and throws on failure', async () => {
        mockGet.mockRejectedValue(new Error('Load failed'))
        const { result } = renderHook(() => useSideTasks())

        let caught: unknown
        await act(async () => {
          try {
            await result.current.load('pipeline-001', 1)
          } catch (e) {
            caught = e
          }
        })

        expect(caught).toBeInstanceOf(Error)
        expect((caught as Error).message).toBe('Load failed')
        expect(result.current.error).toBe('Load failed')
        expect(result.current.tasks).toEqual([])
        expect(result.current.loading).toBe(false)
      })
    })

    describe('create', () => {
      it('creates a side task and returns it', async () => {
        mockPost.mockResolvedValue(mockTask)
        const { result } = renderHook(() => useSideTasks())
        const body = { title: 'New side task', description: 'Do something' }

        let task: unknown
        await act(async () => {
          task = await result.current.create('pipeline-001', 1, body)
        })

        expect(task).toEqual(mockTask)
        expect(mockPost).toHaveBeenCalledWith('/pipelines/pipeline-001/stages/1/side-tasks', body)
        expect(result.current.loading).toBe(false)
        expect(result.current.error).toBe(null)
      })

      it('sets error and throws on failure', async () => {
        mockPost.mockRejectedValue(new Error('Create failed'))
        const { result } = renderHook(() => useSideTasks())

        let caught: unknown
        await act(async () => {
          try {
            await result.current.create('pipeline-001', 1, { title: 'Fail', description: '' })
          } catch (e) {
            caught = e
          }
        })

        expect(caught).toBeInstanceOf(Error)
        expect((caught as Error).message).toBe('Create failed')
        expect(result.current.error).toBe('Create failed')
        expect(result.current.loading).toBe(false)
      })
    })

    describe('remove', () => {
      it('removes a side task', async () => {
        mockDel.mockResolvedValue(undefined)
        const { result } = renderHook(() => useSideTasks())

        await act(async () => {
          await result.current.remove('pipeline-001', 1, 'task-001')
        })

        expect(mockDel).toHaveBeenCalledWith('/pipelines/pipeline-001/stages/1/side-tasks/task-001')
        expect(result.current.loading).toBe(false)
        expect(result.current.error).toBe(null)
      })

      it('sets error and throws on failure', async () => {
        mockDel.mockRejectedValue(new Error('Delete failed'))
        const { result } = renderHook(() => useSideTasks())

        let caught: unknown
        await act(async () => {
          try {
            await result.current.remove('pipeline-001', 1, 'task-001')
          } catch (e) {
            caught = e
          }
        })

        expect(caught).toBeInstanceOf(Error)
        expect((caught as Error).message).toBe('Delete failed')
        expect(result.current.error).toBe('Delete failed')
        expect(result.current.loading).toBe(false)
      })
    })

    it('sets loading during mutation', async () => {
      let resolve: (v: unknown) => void
      mockGet.mockReturnValue(new Promise((r) => { resolve = r }))
      const { result } = renderHook(() => useSideTasks())

      act(() => {
        void result.current.load('pipeline-001', 1)
      })

      await waitFor(() => {
        expect(result.current.loading).toBe(true)
      })

      act(() => {
        resolve!([mockTask])
      })

      await waitFor(() => {
        expect(result.current.loading).toBe(false)
      })
    })
  })

  describe('useCreatePipeline', () => {
    it('creates a pipeline and calls reload', async () => {
      mockPost.mockResolvedValue(mockPipeline)
      const { result } = renderHook(() => useCreatePipeline())

      let pipeline: unknown
      await act(async () => {
        pipeline = await result.current.mutate({ name: 'Test pipeline', stages: [] })
      })

      expect(pipeline).toEqual(mockPipeline)
      expect(mockPost).toHaveBeenCalledWith('/pipelines', { name: 'Test pipeline', stages: [] })
      expect(mockReload).toHaveBeenCalledOnce()
      expect(result.current.loading).toBe(false)
      expect(result.current.error).toBeNull()
    })

    it('sets error and throws on failure', async () => {
      mockPost.mockRejectedValue(new Error('Create failed'))
      const { result } = renderHook(() => useCreatePipeline())

      await act(async () => {
        await expect(result.current.mutate({ name: 'Test', stages: [] })).rejects.toThrow('Create failed')
      })

      expect(result.current.error).toBe('Create failed')
      expect(result.current.loading).toBe(false)
    })
  })

  describe('useDeletePipeline', () => {
    it('deletes a pipeline and calls reload', async () => {
      mockDel.mockResolvedValue(undefined)
      const { result } = renderHook(() => useDeletePipeline())

      await act(async () => {
        await result.current.mutate('pipeline-001')
      })

      expect(mockDel).toHaveBeenCalledWith('/pipelines/pipeline-001')
      expect(mockReload).toHaveBeenCalledOnce()
      expect(result.current.loading).toBe(false)
      expect(result.current.error).toBeNull()
    })

    it('sets error and throws on failure', async () => {
      mockDel.mockRejectedValue(new Error('Delete failed'))
      const { result } = renderHook(() => useDeletePipeline())

      await act(async () => {
        await expect(result.current.mutate('pipeline-001')).rejects.toThrow('Delete failed')
      })

      expect(result.current.error).toBe('Delete failed')
      expect(result.current.loading).toBe(false)
    })
  })
})
